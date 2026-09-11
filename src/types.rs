#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, unused)]

use std::os::raw::{c_char, c_int};

pub type BOOL = i32;

pub const FAT32_MAX_LIMIT: u64 = 32 * 1024 * 1024 * 1024;
pub const VENTOY_EFI_PART_ATTR: u64 = 0x8000_0000_0000_0000;

pub const SIZE_1KB: usize = 1024;
pub const SIZE_1MB: usize = 1024 * 1024;
pub const SIZE_2MB: usize = 2048 * 1024;
pub const SIZE_1GB: u64 = 1024 * 1024 * 1024;
pub const SIZE_1TB: u64 = 1024 * 1024 * 1024 * 1024;
pub const VENTOY_EFI_PART_SIZE: usize = 32 * SIZE_1MB;
pub const VENTOY_PART1_START_SECTOR: u64 = 2048;

pub const VENTOY_FILE_BOOT_IMG: &str = "boot\\boot.img";
pub const VENTOY_FILE_STG1_IMG: &str = "boot\\core.img.xz";
pub const VENTOY_FILE_DISK_IMG: &str = "ventoy\\ventoy.disk.img.xz";
pub const VENTOY_FILE_LOG: &str = "log.txt";
pub const VENTOY_FILE_VERSION: &str = "ventoy\\version";

pub const VENTOY_CLI_LOG: &str = "cli_log.txt";
pub const VENTOY_CLI_PERCENT: &str = "cli_percent.txt";
pub const VENTOY_CLI_DONE: &str = "cli_done.txt";

pub const DRIVE_ACCESS_TIMEOUT: u32 = 15000;
pub const DRIVE_ACCESS_RETRIES: u32 = 150;
pub const VENTOY_MAX_PHY_DRIVE: usize = 128;
pub const VTSI_IMG_MAX_SEG: usize = 128;
pub const VTSI_IMG_MAGIC: u64 = 0x0000_594F_544E_4556; // "VENTOY\0\0"

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum STORAGE_BUS_TYPE {
    BusTypeUnknown = 0x00,
    BusTypeScsi,
    BusTypeAtapi,
    BusTypeAta,
    BusType1394,
    BusTypeSsa,
    BusTypeFibre,
    BusTypeUsb,
    BusTypeRAID,
    BusTypeiScsi,
    BusTypeSas,
    BusTypeSata,
    BusTypeSd,
    BusTypeMmc,
    BusTypeVirtual,
    BusTypeFileBackedVirtual,
    BusTypeSpaces,
    BusTypeNvme,
    BusTypeSCM,
    BusTypeUfs,
    BusTypeMax,
    BusTypeMaxReserved = 0x7F,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum VTOY_FS {
    VTOY_FS_EXFAT = 0,
    VTOY_FS_NTFS = 1,
    VTOY_FS_FAT32 = 2,
    VTOY_FS_UDF = 3,
    VTOY_FS_BUTT = 4,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PROGRESS_POINT {
    PT_START = 0,
    PT_LOCK_FOR_CLEAN = 8,
    PT_DEL_ALL_PART = 9,
    PT_LOCK_FOR_WRITE = 10,
    PT_FORMAT_PART1 = 11,
    PT_LOCK_VOLUME = 12,
    PT_FORMAT_PART2 = 13,
    PT_WRITE_VENTOY_START = 14,
    PT_WRITE_VENTOY_FINISH = 47,
    PT_WRITE_STG1_IMG = 48,
    PT_WRITE_PART_TABLE = 49,
    PT_MOUNT_VOLUME = 50,
    PT_REFORMAT_START = 51,
    PT_REFORMAT_FINISH = 68,
    PT_FINISH = 69,
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PART_TABLE {
    pub active: u8,
    pub start_head: u8,
    pub start_sector_cyl: u16,
    pub fs_flag: u8,
    pub end_head: u8,
    pub end_sector_cyl: u16,
    pub start_sector_id: u32,
    pub sector_count: u32,
}

impl Default for PART_TABLE {
    fn default() -> Self {
        Self {
            active: 0,
            start_head: 0,
            start_sector_cyl: 0,
            fs_flag: 0,
            end_head: 0,
            end_sector_cyl: 0,
            start_sector_id: 0,
            sector_count: 0,
        }
    }
}

impl PART_TABLE {
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut buf = [0u8; 16];
        buf[0] = self.active;
        buf[1] = self.start_head;
        buf[2..4].copy_from_slice(&self.start_sector_cyl.to_le_bytes());
        buf[4] = self.fs_flag;
        buf[5] = self.end_head;
        buf[6..8].copy_from_slice(&self.end_sector_cyl.to_le_bytes());
        buf[8..12].copy_from_slice(&self.start_sector_id.to_le_bytes());
        buf[12..16].copy_from_slice(&self.sector_count.to_le_bytes());
        buf
    }

    pub fn from_bytes(buf: &[u8; 16]) -> Self {
        Self {
            active: buf[0],
            start_head: buf[1],
            start_sector_cyl: u16::from_le_bytes([buf[2], buf[3]]),
            fs_flag: buf[4],
            end_head: buf[5],
            end_sector_cyl: u16::from_le_bytes([buf[6], buf[7]]),
            start_sector_id: u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]),
            sector_count: u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]),
        }
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct MBR_HEAD {
    pub boot_code: [u8; 446],
    pub part_tbl: [PART_TABLE; 4],
    pub byte55: u8,
    pub byte_aa: u8,
}

impl Default for MBR_HEAD {
    fn default() -> Self {
        Self {
            boot_code: [0u8; 446],
            part_tbl: [PART_TABLE::default(); 4],
            byte55: 0x55,
            byte_aa: 0xAA,
        }
    }
}

impl MBR_HEAD {
    pub fn to_bytes(&self) -> [u8; 512] {
        let mut buf = [0u8; 512];
        buf[..446].copy_from_slice(&self.boot_code);
        let part_tbl = self.part_tbl;
        for (i, part) in part_tbl.iter().enumerate() {
            let offset = 446 + i * 16;
            buf[offset..offset + 16].copy_from_slice(&part.to_bytes());
        }
        buf[510] = self.byte55;
        buf[511] = self.byte_aa;
        buf
    }

    pub fn from_bytes(buf: &[u8]) -> Option<Self> {
        if buf.len() < 512 {
            return None;
        }
        let mut boot_code = [0u8; 446];
        boot_code.copy_from_slice(&buf[..446]);
        let mut part_tbl = [PART_TABLE::default(); 4];
        for i in 0..4 {
            let offset = 446 + i * 16;
            let mut part_buf = [0u8; 16];
            part_buf.copy_from_slice(&buf[offset..offset + 16]);
            part_tbl[i] = PART_TABLE::from_bytes(&part_buf);
        }
        Some(Self {
            boot_code,
            part_tbl,
            byte55: buf[510],
            byte_aa: buf[511],
        })
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct VTOY_GPT_HDR {
    pub signature: [u8; 8],
    pub version: [u8; 4],
    pub length: u32,
    pub crc: u32,
    pub reserved1: [u8; 4],
    pub efi_start_lba: u64,
    pub efi_backup_lba: u64,
    pub part_area_start_lba: u64,
    pub part_area_end_lba: u64,
    pub disk_guid: [u8; 16],
    pub part_tbl_start_lba: u64,
    pub part_tbl_tot_num: u32,
    pub part_tbl_entry_len: u32,
    pub part_tbl_crc: u32,
    pub reserved2: [u8; 420],
}

impl Default for VTOY_GPT_HDR {
    fn default() -> Self {
        let mut sig = [0u8; 8];
        sig.copy_from_slice(b"EFI PART");
        Self {
            signature: sig,
            version: [0x00, 0x00, 0x01, 0x00],
            length: 92,
            crc: 0,
            reserved1: [0u8; 4],
            efi_start_lba: 0,
            efi_backup_lba: 0,
            part_area_start_lba: 0,
            part_area_end_lba: 0,
            disk_guid: [0u8; 16],
            part_tbl_start_lba: 0,
            part_tbl_tot_num: 128,
            part_tbl_entry_len: 128,
            part_tbl_crc: 0,
            reserved2: [0u8; 420],
        }
    }
}

impl VTOY_GPT_HDR {
    pub fn to_bytes_92(&self) -> [u8; 92] {
        let mut buf = [0u8; 92];
        buf[0..8].copy_from_slice(&self.signature);
        buf[8..12].copy_from_slice(&self.version);
        buf[12..16].copy_from_slice(&self.length.to_le_bytes());
        buf[16..20].copy_from_slice(&self.crc.to_le_bytes());
        buf[20..24].copy_from_slice(&self.reserved1);
        buf[24..32].copy_from_slice(&self.efi_start_lba.to_le_bytes());
        buf[32..40].copy_from_slice(&self.efi_backup_lba.to_le_bytes());
        buf[40..48].copy_from_slice(&self.part_area_start_lba.to_le_bytes());
        buf[48..56].copy_from_slice(&self.part_area_end_lba.to_le_bytes());
        buf[56..72].copy_from_slice(&self.disk_guid);
        buf[72..80].copy_from_slice(&self.part_tbl_start_lba.to_le_bytes());
        buf[80..84].copy_from_slice(&self.part_tbl_tot_num.to_le_bytes());
        buf[84..88].copy_from_slice(&self.part_tbl_entry_len.to_le_bytes());
        buf[88..92].copy_from_slice(&self.part_tbl_crc.to_le_bytes());
        buf
    }

    pub fn to_sector_bytes(&self) -> [u8; 512] {
        let mut buf = [0u8; 512];
        buf[..92].copy_from_slice(&self.to_bytes_92());
        buf[92..512].copy_from_slice(&self.reserved2);
        buf
    }

    pub fn from_bytes(buf: &[u8]) -> Option<Self> {
        if buf.len() < 92 {
            return None;
        }
        let mut sig = [0u8; 8];
        sig.copy_from_slice(&buf[0..8]);
        let mut ver = [0u8; 4];
        ver.copy_from_slice(&buf[8..12]);
        let length = u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]);
        let crc = u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]);
        let mut reserved1 = [0u8; 4];
        reserved1.copy_from_slice(&buf[20..24]);
        let efi_start_lba = u64::from_le_bytes(buf[24..32].try_into().ok()?);
        let efi_backup_lba = u64::from_le_bytes(buf[32..40].try_into().ok()?);
        let part_area_start_lba = u64::from_le_bytes(buf[40..48].try_into().ok()?);
        let part_area_end_lba = u64::from_le_bytes(buf[48..56].try_into().ok()?);
        let mut disk_guid = [0u8; 16];
        disk_guid.copy_from_slice(&buf[56..72]);
        let part_tbl_start_lba = u64::from_le_bytes(buf[72..80].try_into().ok()?);
        let part_tbl_tot_num = u32::from_le_bytes([buf[80], buf[81], buf[82], buf[83]]);
        let part_tbl_entry_len = u32::from_le_bytes([buf[84], buf[85], buf[86], buf[87]]);
        let part_tbl_crc = u32::from_le_bytes([buf[88], buf[89], buf[90], buf[91]]);
        let mut reserved2 = [0u8; 420];
        if buf.len() >= 512 {
            reserved2.copy_from_slice(&buf[92..512]);
        }
        Some(Self {
            signature: sig,
            version: ver,
            length,
            crc,
            reserved1,
            efi_start_lba,
            efi_backup_lba,
            part_area_start_lba,
            part_area_end_lba,
            disk_guid,
            part_tbl_start_lba,
            part_tbl_tot_num,
            part_tbl_entry_len,
            part_tbl_crc,
            reserved2,
        })
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct VTOY_GPT_PART_TBL {
    pub part_type: [u8; 16],
    pub part_guid: [u8; 16],
    pub start_lba: u64,
    pub last_lba: u64,
    pub attr: u64,
    pub name: [u16; 36],
}

impl Default for VTOY_GPT_PART_TBL {
    fn default() -> Self {
        Self {
            part_type: [0u8; 16],
            part_guid: [0u8; 16],
            start_lba: 0,
            last_lba: 0,
            attr: 0,
            name: [0u16; 36],
        }
    }
}

impl VTOY_GPT_PART_TBL {
    pub fn to_bytes(&self) -> [u8; 128] {
        let mut buf = [0u8; 128];
        buf[0..16].copy_from_slice(&self.part_type);
        buf[16..32].copy_from_slice(&self.part_guid);
        buf[32..40].copy_from_slice(&self.start_lba.to_le_bytes());
        buf[40..48].copy_from_slice(&self.last_lba.to_le_bytes());
        buf[48..56].copy_from_slice(&self.attr.to_le_bytes());
        let name = self.name;
        for (i, c) in name.iter().enumerate() {
            let offset = 56 + i * 2;
            buf[offset..offset + 2].copy_from_slice(&c.to_le_bytes());
        }
        buf
    }

    pub fn from_bytes(buf: &[u8]) -> Option<Self> {
        if buf.len() < 128 {
            return None;
        }
        let mut part_type = [0u8; 16];
        part_type.copy_from_slice(&buf[0..16]);
        let mut part_guid = [0u8; 16];
        part_guid.copy_from_slice(&buf[16..32]);
        let start_lba = u64::from_le_bytes(buf[32..40].try_into().ok()?);
        let last_lba = u64::from_le_bytes(buf[40..48].try_into().ok()?);
        let attr = u64::from_le_bytes(buf[48..56].try_into().ok()?);
        let mut name = [0u16; 36];
        for i in 0..36 {
            let offset = 56 + i * 2;
            name[i] = u16::from_le_bytes([buf[offset], buf[offset + 1]]);
        }
        Some(Self {
            part_type,
            part_guid,
            start_lba,
            last_lba,
            attr,
            name,
        })
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VTOY_GPT_INFO {
    pub mbr: MBR_HEAD,
    pub head: VTOY_GPT_HDR,
    pub part_tbl: [VTOY_GPT_PART_TBL; 128],
}

impl Default for VTOY_GPT_INFO {
    fn default() -> Self {
        Self {
            mbr: MBR_HEAD::default(),
            head: VTOY_GPT_HDR::default(),
            part_tbl: [VTOY_GPT_PART_TBL::default(); 128],
        }
    }
}

impl VTOY_GPT_INFO {
    pub fn primary_to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(34 * 512);
        let mbr = self.mbr;
        let head = self.head;
        buf.extend_from_slice(&mbr.to_bytes());
        buf.extend_from_slice(&head.to_sector_bytes());
        let part_tbl = self.part_tbl;
        for entry in part_tbl.iter() {
            buf.extend_from_slice(&entry.to_bytes());
        }
        buf
    }

    pub fn part_table_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(128 * 128);
        let part_tbl = self.part_tbl;
        for entry in part_tbl.iter() {
            buf.extend_from_slice(&entry.to_bytes());
        }
        buf
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct ventoy_secure_data {
    pub magic1: [u8; 16],
    pub diskuuid: [u8; 16],
    pub checksum: [u8; 16],
    pub admin_sha256: [u8; 32],
    pub reserved: [u8; 4000],
    pub magic2: [u8; 16],
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, Default)]
pub struct VTSI_SEGMENT {
    pub disk_start_sector: u64,
    pub sector_num: u64,
    pub data_offset: u64,
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VTSI_FOOTER {
    pub magic: u64,
    pub version: u32,
    pub disk_size: u64,
    pub disk_signature: u32,
    pub foot_chksum: u32,
    pub segment_num: u32,
    pub segment_chksum: u32,
    pub segment_offset: u64,
    pub reserved: [u8; 512 - 44],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct PHY_DRIVE_INFO {
    pub id: c_int,
    pub phy_drive: c_int,
    pub part_style: c_int, // 0: MBR, 1: GPT
    pub size_in_bytes: u64,
    pub device_type: u8,
    pub removable_media: u32, // BOOL
    pub vendor_id: [c_char; 128],
    pub product_id: [c_char; 128],
    pub product_rev: [c_char; 128],
    pub serial_number: [c_char; 128],
    pub bus_type: STORAGE_BUS_TYPE,

    pub bytes_per_logical_sector: u32,
    pub bytes_per_physical_sector: u32,

    pub drive_letters: [c_char; 64],
    pub ventoy_fs_cluster_size: c_int,
    pub ventoy_fs_type: [c_char; 16],
    pub ventoy_version: [c_char; 32],

    pub secure_boot_support: u32, // BOOL
    pub mbr: MBR_HEAD,
    pub part2_gpt_attr: u64,

    pub resize_no_shrink: u32, // BOOL
    pub resize_old_part1_size: u64,
    pub part1_drive_letter: c_char,
    pub resize_volume_guid: [c_char; 64],
    pub fs_name: [c_char; 64],
    pub resize_part2_start_sector: u64,
    pub gpt: VTOY_GPT_INFO,
}

impl Default for PHY_DRIVE_INFO {
    fn default() -> Self {
        Self {
            id: -1,
            phy_drive: -1,
            part_style: 0,
            size_in_bytes: 0,
            device_type: 0,
            removable_media: 0,
            vendor_id: [0; 128],
            product_id: [0; 128],
            product_rev: [0; 128],
            serial_number: [0; 128],
            bus_type: STORAGE_BUS_TYPE::BusTypeUnknown,
            bytes_per_logical_sector: 512,
            bytes_per_physical_sector: 512,
            drive_letters: [0; 64],
            ventoy_fs_cluster_size: 0,
            ventoy_fs_type: [0; 16],
            ventoy_version: [0; 32],
            secure_boot_support: 1,
            mbr: MBR_HEAD::default(),
            part2_gpt_attr: 0,
            resize_no_shrink: 0,
            resize_old_part1_size: 0,
            part1_drive_letter: 0,
            resize_volume_guid: [0; 64],
            fs_name: [0; 64],
            resize_part2_start_sector: 0,
            gpt: VTOY_GPT_INFO::default(),
        }
    }
}

impl PHY_DRIVE_INFO {
    pub fn vendor_str(&self) -> String {
        c_chars_to_string(&self.vendor_id)
    }

    pub fn product_str(&self) -> String {
        c_chars_to_string(&self.product_id)
    }

    pub fn version_str(&self) -> String {
        c_chars_to_string(&self.ventoy_version)
    }
}

fn c_chars_to_string(chars: &[c_char]) -> String {
    let mut bytes = Vec::with_capacity(chars.len());
    for &c in chars {
        if c == 0 {
            break;
        }
        bytes.push(c as u8);
    }
    String::from_utf8_lossy(&bytes).trim().to_string()
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VTOY_EXTERNAL_DRIVE {
    pub id: c_int,
    pub phy_drive_id: c_int,
    pub model: [c_char; 128],
    pub size_in_bytes: u64,
    pub ventoy_version: [c_char; 32],
    pub part_style: c_int,
    pub fs_type: [c_char; 16],
}

impl Default for VTOY_EXTERNAL_DRIVE {
    fn default() -> Self {
        Self {
            id: 0,
            phy_drive_id: 0,
            model: [0; 128],
            size_in_bytes: 0,
            ventoy_version: [0; 32],
            part_style: 0,
            fs_type: [0; 16],
        }
    }
}

pub type ProgressCallback<'a> = &'a dyn Fn(i32, &str);
pub type ProgressCallbackFunc = fn(percent: i32, status: &str);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mbr_serialization_roundtrip() {
        let mut mbr = MBR_HEAD::default();
        mbr.boot_code[0] = 0xEB;
        mbr.boot_code[1] = 0x48;
        mbr.part_tbl[0].active = 0x80;
        mbr.part_tbl[0].fs_flag = 0x07;
        mbr.part_tbl[0].start_sector_id = 2048;
        mbr.part_tbl[0].sector_count = 1000000;

        let bytes = mbr.to_bytes();
        assert_eq!(bytes.len(), 512);
        assert_eq!(bytes[510], 0x55);
        assert_eq!(bytes[511], 0xAA);

        let parsed = MBR_HEAD::from_bytes(&bytes).expect("Failed to parse MBR bytes");
        let p0 = parsed.part_tbl[0];
        assert_eq!(parsed.boot_code[0], 0xEB);
        assert_eq!(p0.active, 0x80);
        assert_eq!(p0.fs_flag, 0x07);
        let start_sector = p0.start_sector_id;
        let sector_count = p0.sector_count;
        assert_eq!(start_sector, 2048);
        assert_eq!(sector_count, 1000000);
        assert_eq!(parsed.byte55, 0x55);
        assert_eq!(parsed.byte_aa, 0xAA);
    }

    #[test]
    fn test_gpt_header_roundtrip() {
        let mut hdr = VTOY_GPT_HDR::default();
        hdr.efi_start_lba = 1;
        hdr.efi_backup_lba = 500000;
        hdr.part_tbl_crc = 0x12345678;

        let sector = hdr.to_sector_bytes();
        assert_eq!(sector.len(), 512);

        let parsed = VTOY_GPT_HDR::from_bytes(&sector).expect("Failed to parse GPT header");
        assert_eq!(&parsed.signature, b"EFI PART");
        let start_lba = parsed.efi_start_lba;
        let backup_lba = parsed.efi_backup_lba;
        let tbl_crc = parsed.part_tbl_crc;
        assert_eq!(start_lba, 1);
        assert_eq!(backup_lba, 500000);
        assert_eq!(tbl_crc, 0x12345678);
    }
}
