#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, unused)]

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::mem::size_of;
use std::os::raw::{c_char, c_int, c_void};
use std::path::Path;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Storage::FileSystem::*;
use windows_sys::Win32::System::Ioctl::*;

use crate::crc32::crc32;
use crate::disk_service::*;
use crate::fat_io;
use crate::types::STORAGE_BUS_TYPE;
use crate::types::*;
use crate::utility::*;
use crate::ventoy_log;
use crate::xz;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn DeviceIoControl(
        hDevice: HANDLE,
        dwIoControlCode: u32,
        lpInBuffer: *const c_void,
        nInBufferSize: u32,
        lpOutBuffer: *mut c_void,
        nOutBufferSize: u32,
        lpBytesReturned: *mut u32,
        lpOverlapped: *mut c_void,
    ) -> bool;

    fn WriteFile(
        hFile: HANDLE,
        lpBuffer: *const c_void,
        nNumberOfBytesToWrite: u32,
        lpNumberOfBytesWritten: *mut u32,
        lpOverlapped: *mut c_void,
    ) -> bool;

    fn ReadFile(
        hFile: HANDLE,
        lpBuffer: *mut c_void,
        nNumberOfBytesToRead: u32,
        lpNumberOfBytesRead: *mut u32,
        lpOverlapped: *mut c_void,
    ) -> bool;

    fn SetFilePointerEx(
        hFile: HANDLE,
        liDistanceToMove: i64,
        lpNewFilePointer: *mut i64,
        dwMoveMethod: u32,
    ) -> bool;
}

#[link(name = "advapi32")]
unsafe extern "system" {
    fn SystemFunction036(random_buffer: *mut c_void, random_buffer_length: u32) -> u8;
}

/// Safe RAII wrapper around a Win32 physical drive HANDLE.
/// Ensures CloseHandle is deterministically called when dropped.
pub struct PhysicalDriveHandle(HANDLE);

impl Drop for PhysicalDriveHandle {
    fn drop(&mut self) {
        if self.0 != INVALID_HANDLE_VALUE && !self.0.is_null() {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}

impl PhysicalDriveHandle {
    #[inline]
    pub fn raw(&self) -> HANDLE {
        self.0
    }
}

pub fn open_physical_drive(phy_drive: i32, write_access: bool) -> Option<PhysicalDriveHandle> {
    let path = format!("\\\\.\\PhysicalDrive{}\0", phy_drive);
    let access = if write_access {
        GENERIC_READ | GENERIC_WRITE
    } else {
        GENERIC_READ
    };
    let share = FILE_SHARE_READ | FILE_SHARE_WRITE;

    let handle = unsafe {
        CreateFileA(
            path.as_ptr(),
            access,
            share,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            0,
            0 as HANDLE,
        )
    };

    if handle == INVALID_HANDLE_VALUE || handle.is_null() {
        None
    } else {
        Some(PhysicalDriveHandle(handle))
    }
}

pub fn generate_random_guid() -> [u8; 16] {
    let mut guid = [0u8; 16];
    unsafe {
        if SystemFunction036(guid.as_mut_ptr() as *mut _, 16) == 0 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let b = now.to_le_bytes();
            guid[..16].copy_from_slice(&b[..16]);
        }
    }
    guid[6] = (guid[6] & 0x0F) | 0x40;
    guid[8] = (guid[8] & 0x3F) | 0x80;
    guid
}

pub fn write_data_to_phy_disk(handle: &PhysicalDriveHandle, mut offset: u64, buffer: &[u8]) -> bool {
    const CHUNK_SIZE: usize = 1024 * 1024; // 1 MiB chunks
    for chunk in buffer.chunks(CHUNK_SIZE) {
        let mut new_pos: i64 = 0;
        let set_res = unsafe {
            SetFilePointerEx(handle.raw(), offset as i64, &mut new_pos, FILE_BEGIN)
        };
        if !set_res || new_pos != offset as i64 {
            ventoy_log!("SetFilePointerEx failed at offset {}", offset);
            return false;
        }

        let mut bytes_written: u32 = 0;
        let ret = unsafe {
            WriteFile(
                handle.raw(),
                chunk.as_ptr() as *const c_void,
                chunk.len() as u32,
                &mut bytes_written,
                std::ptr::null_mut(),
            )
        };

        if !ret || (bytes_written as usize) != chunk.len() {
            ventoy_log!(
                "WriteFile failed at offset {}: written {} of {}",
                offset,
                bytes_written,
                chunk.len()
            );
            return false;
        }
        offset += chunk.len() as u64;
    }
    true
}

pub fn read_data_from_phy_disk(handle: &PhysicalDriveHandle, mut offset: u64, buffer: &mut [u8]) -> bool {
    const CHUNK_SIZE: usize = 1024 * 1024; // 1 MiB chunks
    for chunk in buffer.chunks_mut(CHUNK_SIZE) {
        let mut new_pos: i64 = 0;
        let set_res = unsafe {
            SetFilePointerEx(handle.raw(), offset as i64, &mut new_pos, FILE_BEGIN)
        };
        if !set_res || new_pos != offset as i64 {
            return false;
        }

        let mut bytes_read: u32 = 0;
        let ret = unsafe {
            ReadFile(
                handle.raw(),
                chunk.as_mut_ptr() as *mut c_void,
                chunk.len() as u32,
                &mut bytes_read,
                std::ptr::null_mut(),
            )
        };

        if !ret || (bytes_read as usize) != chunk.len() {
            return false;
        }
        offset += chunk.len() as u64;
    }
    true
}

pub fn get_physical_drive_count() -> i32 {
    let mut count = 0;
    for i in 0..VENTOY_MAX_PHY_DRIVE {
        if let Some(_handle) = open_physical_drive(i as i32, false) {
            count += 1;
        } else if i > 16 && count == 0 {
            break;
        }
    }
    count
}

pub fn is_ventoy_phy_drive(
    phy_drive: i32,
    _size_bytes: u64,
    out_mbr: Option<&mut MBR_HEAD>,
    out_part2_start: Option<&mut u64>,
    out_gpt_attr: Option<&mut u64>,
) -> bool {
    let handle = match open_physical_drive(phy_drive, false) {
        Some(h) => h,
        None => return false,
    };

    let mut mbr_buf = [0u8; 512];
    if !read_data_from_phy_disk(&handle, 0, &mut mbr_buf) {
        return false;
    }

    let mbr = match MBR_HEAD::from_bytes(&mbr_buf) {
        Some(m) => m,
        None => return false,
    };

    if mbr.byte55 != 0x55 || mbr.byte_aa != 0xAA {
        return false;
    }

    let mut is_ventoy = false;
    let mut part2_start: u64 = 0;
    let mut gpt_attr: u64 = 0;

    // Check MBR partition 2
    if mbr.part_tbl[1].fs_flag == 0xEF && mbr.part_tbl[1].sector_count == 65536 {
        part2_start = mbr.part_tbl[1].start_sector_id as u64;
        is_ventoy = true;
    } else if mbr.part_tbl[0].fs_flag == 0xEE {
        // Protective MBR -> check GPT
        let mut gpt_hdr_buf = [0u8; 512];
        if read_data_from_phy_disk(&handle, 512, &mut gpt_hdr_buf) {
            if let Some(gpt_hdr) = VTOY_GPT_HDR::from_bytes(&gpt_hdr_buf) {
                if &gpt_hdr.signature == b"EFI PART" {
                    let mut part2_buf = [0u8; 128];
                    let p2_offset = (gpt_hdr.part_tbl_start_lba * 512) + 128;
                    if read_data_from_phy_disk(&handle, p2_offset, &mut part2_buf) {
                        if let Some(part2) = VTOY_GPT_PART_TBL::from_bytes(&part2_buf) {
                            if (part2.last_lba - part2.start_lba + 1) == 65536 {
                                part2_start = part2.start_lba;
                                gpt_attr = part2.attr;
                                is_ventoy = true;
                            }
                        }
                    }
                }
            }
        }
    }

    drop(handle);

    if is_ventoy {
        let (_ver, _sb, is_valid) = get_ventoy_ver_in_phy_drive(phy_drive, part2_start);
        if !is_valid {
            is_ventoy = false;
        }
    }

    if is_ventoy {
        if let Some(out_m) = out_mbr {
            *out_m = mbr;
        }
        if let Some(out_p2) = out_part2_start {
            *out_p2 = part2_start;
        }
        if let Some(out_attr) = out_gpt_attr {
            *out_attr = gpt_attr;
        }
    }

    is_ventoy
}

pub fn get_ventoy_ver_in_phy_drive(
    phy_drive: i32,
    part2_start_sector: u64,
) -> (String, bool, bool) {
    let handle = match open_physical_drive(phy_drive, false) {
        Some(h) => h,
        None => return ("".to_string(), false, false),
    };

    // Read first 8 megabytes of partition 2 to inspect FAT
    let mut fat_buf = vec![0u8; 8 * SIZE_1MB];
    let offset = part2_start_sector * 512;
    let read_ok = read_data_from_phy_disk(&handle, offset, &mut fat_buf);
    drop(handle);

    if !read_ok || fat_buf.len() < 512 {
        return ("".to_string(), false, false);
    }

    // Check FAT boot sector signature 0x55, 0xAA
    if fat_buf[510] != 0x55 || fat_buf[511] != 0xAA {
        return ("".to_string(), false, false);
    }

    // Check for VTOY_EFI volume label in boot sector or volume label entries
    let has_vtoy_label = fat_buf[..512]
        .windows(8)
        .any(|w| w == b"VTOY_EFI" || w == b"VTOYEFI ");

    // Read actual version file from \ventoy\version inside Partition 2
    let ver_from_disk = match fat_io::read_file_from_image(&mut fat_buf, "ventoy/version") {
        Ok(data) => String::from_utf8_lossy(&data).trim().to_string(),
        Err(_) => match fat_io::read_file_from_image(&mut fat_buf, "version") {
            Ok(data) => String::from_utf8_lossy(&data).trim().to_string(),
            Err(_) => String::new(),
        },
    };

    let secure_boot = fat_io::is_secure_boot_enabled_in_image(&mut fat_buf);
    let is_valid = has_vtoy_label || !ver_from_disk.is_empty() || secure_boot;

    if is_valid {
        let final_ver = if !ver_from_disk.is_empty() {
            ver_from_disk
        } else {
            get_local_ventoy_version()
        };
        (final_ver, secure_boot, true)
    } else {
        ("".to_string(), false, false)
    }
}

pub fn scan_all_physical_drives() -> Vec<PHY_DRIVE_INFO> {
    let mut drive_list = Vec::new();

    for i in 0..32 {
        let handle = match open_physical_drive(i, false) {
            Some(h) => h,
            None => continue,
        };

        let mut length_info = GET_LENGTH_INFORMATION { Length: 0 };
        let mut bytes_ret: u32 = 0;
        let ret = unsafe {
            DeviceIoControl(
                handle.raw(),
                IOCTL_DISK_GET_LENGTH_INFO,
                std::ptr::null(),
                0,
                &mut length_info as *mut _ as *mut c_void,
                size_of::<GET_LENGTH_INFORMATION>() as u32,
                &mut bytes_ret,
                std::ptr::null_mut(),
            )
        };

        let mut size_in_bytes = if ret {
            length_info.Length as u64
        } else {
            0
        };

        if size_in_bytes == 0 {
            let mut geom_ex: DISK_GEOMETRY_EX = unsafe { std::mem::zeroed() };
            let ret_geom = unsafe {
                DeviceIoControl(
                    handle.raw(),
                    IOCTL_DISK_GET_DRIVE_GEOMETRY_EX,
                    std::ptr::null(),
                    0,
                    &mut geom_ex as *mut _ as *mut c_void,
                    size_of::<DISK_GEOMETRY_EX>() as u32,
                    &mut bytes_ret,
                    std::ptr::null_mut(),
                )
            };
            if ret_geom {
                size_in_bytes = geom_ex.DiskSize as u64;
            }
        }

        if size_in_bytes == 0 {
            continue;
        }

        // Query storage property
        let mut query = STORAGE_PROPERTY_QUERY {
            PropertyId: StorageDeviceProperty,
            QueryType: PropertyStandardQuery,
            AdditionalParameters: [0],
        };
        let mut desc_buf = vec![0u8; 1024];
        let ret_query = unsafe {
            DeviceIoControl(
                handle.raw(),
                IOCTL_STORAGE_QUERY_PROPERTY,
                &mut query as *mut _ as *mut c_void,
                size_of::<STORAGE_PROPERTY_QUERY>() as u32,
                desc_buf.as_mut_ptr() as *mut c_void,
                desc_buf.len() as u32,
                &mut bytes_ret,
                std::ptr::null_mut(),
            )
        };

        drop(handle);

        let mut drive_info = PHY_DRIVE_INFO::default();
        drive_info.id = drive_list.len() as i32;
        drive_info.phy_drive = i;
        drive_info.size_in_bytes = size_in_bytes;
        drive_info.bytes_per_logical_sector = 512;
        drive_info.bytes_per_physical_sector = 512;

        if ret_query && desc_buf.len() >= size_of::<STORAGE_DEVICE_DESCRIPTOR>() {
            let desc = unsafe {
                std::ptr::read_unaligned(desc_buf.as_ptr() as *const STORAGE_DEVICE_DESCRIPTOR)
            };
            let bus_type_raw = desc.BusType as u32;
            drive_info.bus_type = match bus_type_raw {
                7 => STORAGE_BUS_TYPE::BusTypeUsb,
                1 => STORAGE_BUS_TYPE::BusTypeScsi,
                3 => STORAGE_BUS_TYPE::BusTypeAta,
                11 => STORAGE_BUS_TYPE::BusTypeSata,
                17 => STORAGE_BUS_TYPE::BusTypeNvme,
                12 => STORAGE_BUS_TYPE::BusTypeSd,
                13 => STORAGE_BUS_TYPE::BusTypeMmc,
                _ => STORAGE_BUS_TYPE::BusTypeUnknown,
            };
            drive_info.removable_media = if desc.RemovableMedia { 1 } else { 0 };

            let extract_str = |offset: u32, target: &mut [c_char]| {
                if offset > 0 && (offset as usize) < desc_buf.len() {
                    let mut idx = offset as usize;
                    let mut out_idx = 0;
                    while idx < desc_buf.len() && out_idx < target.len() - 1 {
                        let b = desc_buf[idx];
                        if b == 0 { break; }
                        target[out_idx] = b as c_char;
                        idx += 1;
                        out_idx += 1;
                    }
                    target[out_idx] = 0;
                }
            };

            extract_str(desc.VendorIdOffset, &mut drive_info.vendor_id);
            extract_str(desc.ProductIdOffset, &mut drive_info.product_id);
            extract_str(desc.ProductRevisionOffset, &mut drive_info.product_rev);
            extract_str(desc.SerialNumberOffset, &mut drive_info.serial_number);
        }

        // Check if Ventoy is already on disk
        let mut part2_start: u64 = 0;
        let mut gpt_attr: u64 = 0;
        let mut mbr = MBR_HEAD::default();
        let is_vtoy = is_ventoy_phy_drive(
            i,
            size_in_bytes,
            Some(&mut mbr),
            Some(&mut part2_start),
            Some(&mut gpt_attr),
        );

        if is_vtoy {
            drive_info.mbr = mbr;
            let (ver, sb, is_valid) = get_ventoy_ver_in_phy_drive(i, part2_start);
            if is_valid {
                let ver_bytes = ver.as_bytes();
                for (j, &b) in ver_bytes.iter().take(31).enumerate() {
                    drive_info.ventoy_version[j] = b as c_char;
                }
                drive_info.secure_boot_support = if sb { 1 } else { 0 };
                drive_info.part2_gpt_attr = gpt_attr;
            } else {
                drive_info.secure_boot_support = 1;
            }
        } else {
            drive_info.secure_boot_support = 1;
        }

        drive_list.push(drive_info);
    }

    // Sort: USB / removable first
    drive_list.sort_by(|a, b| {
        let a_usb = a.bus_type == STORAGE_BUS_TYPE::BusTypeUsb || a.removable_media != 0;
        let b_usb = b.bus_type == STORAGE_BUS_TYPE::BusTypeUsb || b.removable_media != 0;
        if a_usb && !b_usb {
            std::cmp::Ordering::Less
        } else if !a_usb && b_usb {
            std::cmp::Ordering::Greater
        } else {
            a.phy_drive.cmp(&b.phy_drive)
        }
    });

    for (idx, drive) in drive_list.iter_mut().enumerate() {
        drive.id = idx as i32;
    }

    drive_list
}

pub fn ventoy_fill_mbr(disk_size_bytes: u64, mbr: &mut MBR_HEAD, _part_style: i32, fs_flag: u8) {
    *mbr = MBR_HEAD::default();
    let total_sectors = disk_size_bytes / 512;
    let efi_sectors = 65536u64; // 32MB
    let part1_start = VENTOY_PART1_START_SECTOR;
    let part1_sectors = total_sectors.saturating_sub(part1_start + efi_sectors);

    // Partition 1 (Ventoy Data)
    mbr.part_tbl[0].active = 0x00;
    mbr.part_tbl[0].fs_flag = fs_flag;
    mbr.part_tbl[0].start_sector_id = part1_start as u32;
    mbr.part_tbl[0].sector_count = part1_sectors as u32;

    // Partition 2 (VTOYEFI)
    mbr.part_tbl[1].active = 0x80;
    mbr.part_tbl[1].fs_flag = 0xEF;
    mbr.part_tbl[1].start_sector_id = (part1_start + part1_sectors) as u32;
    mbr.part_tbl[1].sector_count = efi_sectors as u32;
}

pub fn ventoy_fill_gpt(disk_size_bytes: u64, gpt: &mut VTOY_GPT_INFO) {
    *gpt = VTOY_GPT_INFO::default();
    let total_sectors = disk_size_bytes / 512;
    let efi_sectors = 65536u64;
    let part1_start = VENTOY_PART1_START_SECTOR;
    let part1_sectors = total_sectors.saturating_sub(part1_start + efi_sectors + 34);

    // Protective MBR
    gpt.mbr.byte55 = 0x55;
    gpt.mbr.byte_aa = 0xAA;
    gpt.mbr.part_tbl[0].fs_flag = 0xEE;
    gpt.mbr.part_tbl[0].start_sector_id = 1;
    gpt.mbr.part_tbl[0].sector_count = (total_sectors.min(0xFFFFFFFF) - 1) as u32;

    // Primary Header
    gpt.head.efi_start_lba = 1;
    gpt.head.efi_backup_lba = total_sectors - 1;
    gpt.head.part_area_start_lba = 34;
    gpt.head.part_area_end_lba = total_sectors - 34;
    gpt.head.part_tbl_start_lba = 2;
    gpt.head.disk_guid = generate_random_guid();

    // Part 1: Microsoft Basic Data (EBD0A0A2-B9E5-4433-87C0-68B6B72699C7)
    gpt.part_tbl[0].part_type = [0xA2, 0xA0, 0xD0, 0xEB, 0xE5, 0xB9, 0x33, 0x44, 0x87, 0xC0, 0x68, 0xB6, 0xB7, 0x26, 0x99, 0xC7];
    gpt.part_tbl[0].part_guid = generate_random_guid();
    gpt.part_tbl[0].start_lba = part1_start;
    gpt.part_tbl[0].last_lba = part1_start + part1_sectors - 1;
    let mut name0 = [0u16; 36];
    let p1_name = ['V' as u16, 'e' as u16, 'n' as u16, 't' as u16, 'o' as u16, 'y' as u16];
    name0[..p1_name.len()].copy_from_slice(&p1_name);
    gpt.part_tbl[0].name = name0;

    // Part 2: EFI System Partition (C12A7328-F81F-11D2-BA4B-00A0C93EC93B)
    gpt.part_tbl[1].part_type = [0x28, 0x73, 0x2A, 0xC1, 0x1F, 0xF8, 0xD2, 0x11, 0xBA, 0x4B, 0x00, 0xA0, 0xC9, 0x3E, 0xC9, 0x3B];
    gpt.part_tbl[1].part_guid = generate_random_guid();
    gpt.part_tbl[1].start_lba = part1_start + part1_sectors;
    gpt.part_tbl[1].last_lba = gpt.part_tbl[1].start_lba + efi_sectors - 1;
    gpt.part_tbl[1].attr = VENTOY_EFI_PART_ATTR;
    let mut name1 = [0u16; 36];
    let p2_name = ['V' as u16, 'T' as u16, 'O' as u16, 'Y' as u16, 'E' as u16, 'F' as u16, 'I' as u16];
    name1[..p2_name.len()].copy_from_slice(&p2_name);
    gpt.part_tbl[1].name = name1;

    // Calculate CRCs using safe serialized byte slices
    let part_tbl_bytes = gpt.part_table_bytes();
    gpt.head.part_tbl_crc = crc32(&part_tbl_bytes);

    gpt.head.crc = 0;
    let head_bytes = gpt.head.to_bytes_92();
    gpt.head.crc = crc32(&head_bytes);
}

pub fn install_ventoy_to_phy_drive(
    drive: &PHY_DRIVE_INFO,
    part_style: i32,
    secure_boot: bool,
    callback: Option<ProgressCallbackFunc>,
) -> i32 {
    let phy_drive_id = drive.phy_drive;
    let size_bytes = drive.size_in_bytes;

    let update_progress = |percent: i32, status: &str| {
        if let Some(cb) = callback {
            cb(percent, status);
        }
        ventoy_log!("[{}%] {}", percent, status);
    };

    update_progress(10, "Cleaning disk partitions...");
    if !disk_clean_disk(phy_drive_id as u32) {
        update_progress(10, "Failed to clean target disk partitions");
        return -2;
    }

    update_progress(20, "Decompressing Ventoy EFI image with lzma-rs...");
    let efi_xz_candidates = [
        "ventoy/ventoy.disk.img.xz",
        "ventoy\\ventoy.disk.img.xz",
        "ventoy.disk.img.xz",
    ];
    let mut efi_img_bytes = None;
    for cand in &efi_xz_candidates {
        if let Some(path) = find_asset_path(cand) {
            match xz::decompress_xz_file(path.to_str().unwrap_or(cand)) {
                Ok(bytes) => {
                    efi_img_bytes = Some(bytes);
                    break;
                }
                Err(e) => {
                    ventoy_log!("Failed to decompress {}: {}", cand, e);
                }
            }
        }
    }

    let mut efi_data = match efi_img_bytes {
        Some(b) => b,
        None => {
            update_progress(25, "Error: Could not find or decompress ventoy.disk.img.xz");
            return -1;
        }
    };

    if !secure_boot {
        update_progress(30, "Configuring EFI filesystem (Secure Boot disabled)...");
        let _ = fat_io::disable_secure_boot_in_image(&mut efi_data);
    } else {
        update_progress(30, "Preserving signed Secure Boot EFI binaries...");
    }

    update_progress(40, "Opening disk for writing...");
    let h_disk = match open_physical_drive(phy_drive_id, true) {
        Some(h) => h,
        None => {
            update_progress(40, "Failed to open physical drive with write access");
            return -3;
        }
    };

    let total_sectors = size_bytes / 512;
    let efi_sectors = 65536u64;
    let part1_start = VENTOY_PART1_START_SECTOR;
    let part1_sectors = total_sectors.saturating_sub(part1_start + efi_sectors + if part_style == 1 { 34 } else { 0 });
    let part2_start_sector = part1_start + part1_sectors;

    update_progress(50, "Writing EFI partition to disk...");
    let efi_offset = part2_start_sector * 512;
    if !write_data_to_phy_disk(&h_disk, efi_offset, &efi_data) {
        update_progress(50, "Failed to write EFI partition data");
        return -4;
    }

    update_progress(70, "Writing partition tables...");
    if part_style == 1 {
        // GPT
        let mut gpt = VTOY_GPT_INFO::default();
        ventoy_fill_gpt(size_bytes, &mut gpt);
        let gpt_bytes = gpt.primary_to_bytes();
        if !write_data_to_phy_disk(&h_disk, 0, &gpt_bytes) {
            update_progress(70, "Failed to write primary GPT partition table");
            return -5;
        }

        // Backup GPT Partition Entry Array (32 sectors before backup header)
        let backup_part_tbl_offset = (gpt.head.efi_backup_lba - 32) * 512;
        let part_tbl_bytes = gpt.part_table_bytes();
        if !write_data_to_phy_disk(&h_disk, backup_part_tbl_offset, &part_tbl_bytes) {
            update_progress(75, "Failed to write backup GPT partition array");
            return -6;
        }

        // Backup GPT Header
        let mut backup_head = gpt.head;
        backup_head.efi_start_lba = gpt.head.efi_backup_lba;
        backup_head.efi_backup_lba = gpt.head.efi_start_lba;
        backup_head.part_tbl_start_lba = gpt.head.efi_backup_lba - 32;
        backup_head.crc = 0;
        let backup_slice = backup_head.to_bytes_92();
        backup_head.crc = crc32(&backup_slice);

        let backup_offset = (gpt.head.efi_backup_lba * 512) as u64;
        let full_backup_bytes = backup_head.to_sector_bytes();
        if !write_data_to_phy_disk(&h_disk, backup_offset, &full_backup_bytes) {
            update_progress(78, "Failed to write backup GPT header");
            return -7;
        }
    } else {
        // MBR
        let mut mbr = MBR_HEAD::default();
        ventoy_fill_mbr(size_bytes, &mut mbr, part_style, 0x07);
        let mbr_bytes = mbr.to_bytes();
        if !write_data_to_phy_disk(&h_disk, 0, &mbr_bytes) {
            update_progress(70, "Failed to write MBR partition table");
            return -5;
        }
    }

    // Refresh disk layout
    let mut bytes_ret: u32 = 0;
    unsafe {
        DeviceIoControl(
            h_disk.raw(),
            IOCTL_DISK_UPDATE_PROPERTIES,
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            0,
            &mut bytes_ret,
            std::ptr::null_mut(),
        );
    }
    drop(h_disk);

    // Allow Windows PNP and partition manager time to recognize new partition table
    std::thread::sleep(std::time::Duration::from_millis(1500));

    update_progress(85, "Formatting Ventoy data partition (exFAT)...");
    if !crate::disk_service::disk_format_partition(phy_drive_id as u32, 1, "exFAT", 0) {
        update_progress(85, "Failed to format Ventoy data partition");
        return -8;
    }

    update_progress(100, "rVentoy installation completed successfully!");
    0
}

pub fn update_ventoy_to_phy_drive(
    drive: &PHY_DRIVE_INFO,
    callback: Option<ProgressCallbackFunc>,
) -> i32 {
    let phy_drive_id = drive.phy_drive;
    let size_bytes = drive.size_in_bytes;

    let update_progress = |percent: i32, status: &str| {
        if let Some(cb) = callback {
            cb(percent, status);
        }
        ventoy_log!("[{}%] {}", percent, status);
    };

    update_progress(10, "Starting rVentoy update...");
    let mut mbr = MBR_HEAD::default();
    let mut part2_start_sector: u64 = 0;
    let mut gpt_attr: u64 = 0;
    if !is_ventoy_phy_drive(
        phy_drive_id,
        size_bytes,
        Some(&mut mbr),
        Some(&mut part2_start_sector),
        Some(&mut gpt_attr),
    ) {
        update_progress(10, "Drive is not a valid Ventoy drive");
        return -1;
    }

    update_progress(30, "Decompressing new Ventoy EFI image with lzma-rs...");
    let efi_xz_candidates = [
        "ventoy/ventoy.disk.img.xz",
        "ventoy\\ventoy.disk.img.xz",
        "ventoy.disk.img.xz",
    ];
    let mut efi_data = None;
    for cand in &efi_xz_candidates {
        if let Some(path) = find_asset_path(cand) {
            if let Ok(bytes) = xz::decompress_xz_file(path.to_str().unwrap_or(cand)) {
                efi_data = Some(bytes);
                break;
            }
        }
    }

    let mut efi_bytes = match efi_data {
        Some(b) => b,
        None => {
            update_progress(30, "Error: Could not find ventoy.disk.img.xz");
            return -2;
        }
    };

    if drive.secure_boot_support == 0 {
        update_progress(50, "Modifying EFI filesystem with fatfs...");
        let _ = fat_io::disable_secure_boot_in_image(&mut efi_bytes);
    }

    update_progress(70, "Writing updated EFI partition to disk...");
    let h_disk = match open_physical_drive(phy_drive_id, true) {
        Some(h) => h,
        None => {
            update_progress(70, "Failed to open physical drive with write access");
            return -3;
        }
    };

    let efi_offset = part2_start_sector * 512;
    if !write_data_to_phy_disk(&h_disk, efi_offset, &efi_bytes) {
        update_progress(70, "Failed to write EFI partition");
        return -4;
    }

    let mut bytes_ret: u32 = 0;
    unsafe {
        DeviceIoControl(
            h_disk.raw(),
            IOCTL_DISK_UPDATE_PROPERTIES,
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            0,
            &mut bytes_ret,
            std::ptr::null_mut(),
        );
    }
    drop(h_disk);

    update_progress(100, "rVentoy update completed successfully!");
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guid_generation() {
        let g1 = generate_random_guid();
        let g2 = generate_random_guid();
        assert_ne!(g1, [0u8; 16]);
        assert_ne!(g2, [0u8; 16]);
        assert_ne!(g1, g2);

        // Version 4 check
        assert_eq!(g1[6] & 0xF0, 0x40);
        assert_eq!(g2[6] & 0xF0, 0x40);

        // RFC 4122 variant check
        assert_eq!(g1[8] & 0xC0, 0x80);
        assert_eq!(g2[8] & 0xC0, 0x80);
    }

    #[test]
    fn test_ventoy_fill_gpt() {
        let size_bytes: u64 = 32 * 1024 * 1024 * 1024; // 32GB
        let mut gpt = VTOY_GPT_INFO::default();
        ventoy_fill_gpt(size_bytes, &mut gpt);

        let disk_guid = gpt.head.disk_guid;
        let p0 = gpt.part_tbl[0];
        let p1 = gpt.part_tbl[1];

        // Disk GUID and partition GUIDs must be valid non-zero
        assert_ne!(disk_guid, [0u8; 16]);
        assert_ne!(p0.part_guid, [0u8; 16]);
        assert_ne!(p1.part_guid, [0u8; 16]);
        assert_ne!(p0.part_guid, p1.part_guid);

        let name0 = p0.name;
        let name1 = p1.name;

        // Names
        assert_eq!(
            &name0[..6],
            &['V' as u16, 'e' as u16, 'n' as u16, 't' as u16, 'o' as u16, 'y' as u16]
        );
        assert_eq!(
            &name1[..7],
            &['V' as u16, 'T' as u16, 'O' as u16, 'Y' as u16, 'E' as u16, 'F' as u16, 'I' as u16]
        );

        // CRCs
        let crc = gpt.head.crc;
        let tbl_crc = gpt.head.part_tbl_crc;
        assert_ne!(crc, 0);
        assert_ne!(tbl_crc, 0);

        // LBA layout
        let start_lba = gpt.head.efi_start_lba;
        let backup_lba = gpt.head.efi_backup_lba;
        let tbl_start = gpt.head.part_tbl_start_lba;
        assert_eq!(start_lba, 1);
        assert_eq!(backup_lba, (size_bytes / 512) - 1);
        assert_eq!(tbl_start, 2);
    }

    #[test]
    fn test_ventoy_fill_mbr() {
        let size_bytes: u64 = 16 * 1024 * 1024 * 1024; // 16GB
        let mut mbr = MBR_HEAD::default();
        ventoy_fill_mbr(size_bytes, &mut mbr, 0, 0x07);

        let byte55 = mbr.byte55;
        let byte_aa = mbr.byte_aa;
        let part0 = mbr.part_tbl[0];
        let part1 = mbr.part_tbl[1];

        let part0_start = part0.start_sector_id;
        let part1_sectors = part1.sector_count;

        assert_eq!(byte55, 0x55);
        assert_eq!(byte_aa, 0xAA);
        assert_eq!(part0.fs_flag, 0x07);
        assert_eq!(part0_start, 2048);
        assert_eq!(part1.fs_flag, 0xEF);
        assert_eq!(part1_sectors, 65536);
        assert_eq!(part1.active, 0x80);
    }
}


