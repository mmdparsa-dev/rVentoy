use crate::phy_drive::{write_data_to_phy_disk, PhysicalDriveHandle};

static UPCASE_DATA: &[u8] = include_bytes!("upcase.bin");

pub fn format_exfat_partition_native(
    handle: &PhysicalDriveHandle,
    part_start_sector: u64,
    part_sectors: u64,
    volume_label: &str,
) -> bool {
    let chunks = match generate_exfat_metadata(part_start_sector, part_sectors, volume_label) {
        Some(c) => c,
        None => return false,
    };

    for (offset, data) in chunks {
        if !write_data_to_phy_disk(handle, offset, &data) {
            return false;
        }
    }
    true
}

pub fn generate_exfat_metadata(
    part_start_sector: u64,
    part_sectors: u64,
    volume_label: &str,
) -> Option<Vec<(u64, Vec<u8>)>> {
    if part_sectors < 65536 {
        return None;
    }

    let bytes_per_sector: u32 = 512;
    let bytes_per_sector_shift: u8 = 9;

    let sectors_per_cluster_shift: u8 = if part_sectors >= 67_108_864 {
        8
    } else {
        6
    };
    let sectors_per_cluster: u32 = 1 << sectors_per_cluster_shift;
    let cluster_size: u32 = sectors_per_cluster * bytes_per_sector;

    let fat_offset = 128u32;
    let cluster_count_est = (part_sectors.saturating_sub(fat_offset as u64) / sectors_per_cluster as u64) as u32;
    let fat_bytes = ((cluster_count_est as u64 + 2) * 4) as u32;
    let fat_length = (fat_bytes + bytes_per_sector - 1) / bytes_per_sector;
    let raw_heap_offset = fat_offset + fat_length;
    let cluster_heap_offset = ((raw_heap_offset + sectors_per_cluster - 1) / sectors_per_cluster) * sectors_per_cluster;
    let cluster_count = (part_sectors.saturating_sub(cluster_heap_offset as u64) / sectors_per_cluster as u64) as u32;

    if cluster_count < 16 {
        return None;
    }

    let bitmap_len = ((cluster_count as u64 + 7) / 8) as u32;
    let bitmap_clusters = (bitmap_len + cluster_size - 1) / cluster_size;
    let upcase_cluster = 2 + bitmap_clusters;
    let upcase_clusters = 1u32;
    let root_dir_cluster = upcase_cluster + upcase_clusters;
    let total_allocated_clusters = bitmap_clusters + upcase_clusters + 1;

    let mut vbr = [0u8; 512];
    vbr[0..3].copy_from_slice(&[0xEB, 0x76, 0x90]);
    vbr[3..11].copy_from_slice(b"EXFAT   ");
    vbr[0x40..0x48].copy_from_slice(&part_start_sector.to_le_bytes());
    vbr[0x48..0x50].copy_from_slice(&part_sectors.to_le_bytes());
    vbr[0x50..0x54].copy_from_slice(&fat_offset.to_le_bytes());
    vbr[0x54..0x58].copy_from_slice(&fat_length.to_le_bytes());
    vbr[0x58..0x5C].copy_from_slice(&cluster_heap_offset.to_le_bytes());
    vbr[0x5C..0x60].copy_from_slice(&cluster_count.to_le_bytes());
    vbr[0x60..0x64].copy_from_slice(&root_dir_cluster.to_le_bytes());

    let serial = 0x20260911u32
        ^ (part_start_sector as u32)
        ^ (part_sectors as u32)
        ^ ((part_sectors >> 32) as u32);
    vbr[0x64..0x68].copy_from_slice(&serial.to_le_bytes());
    vbr[0x68..0x6A].copy_from_slice(&[0x00, 0x01]);
    vbr[0x6A..0x6C].copy_from_slice(&[0x00, 0x00]);
    vbr[0x6C] = bytes_per_sector_shift;
    vbr[0x6D] = sectors_per_cluster_shift;
    vbr[0x6E] = 1;
    vbr[0x6F] = 0x80;
    vbr[0x70] = 0;
    vbr[510..512].copy_from_slice(&[0x55, 0xAA]);

    let mut boot_region = vec![0u8; 12 * 512];
    boot_region[0..512].copy_from_slice(&vbr);
    for i in 1..11 {
        let off = i * 512;
        boot_region[off + 510] = 0x55;
        boot_region[off + 511] = 0xAA;
    }

    let mut chk: u32 = 0;
    for (i, &b) in boot_region[..11 * 512].iter().enumerate() {
        if i == 0x6A || i == 0x6B || i == 0x70 {
            continue;
        }
        chk = ((chk << 31) | (chk >> 1)).wrapping_add(b as u32);
    }
    let chk_bytes = chk.to_le_bytes();
    for j in 0..128 {
        let off = 11 * 512 + j * 4;
        boot_region[off..off + 4].copy_from_slice(&chk_bytes);
    }

    let mut full_boot_region = vec![0u8; 24 * 512];
    full_boot_region[0..12 * 512].copy_from_slice(&boot_region);
    full_boot_region[12 * 512..24 * 512].copy_from_slice(&boot_region);

    let mut fat_sector = vec![0u8; 512];
    fat_sector[0..4].copy_from_slice(&0xFFFFFFF8u32.to_le_bytes());
    fat_sector[4..8].copy_from_slice(&0xFFFFFFFFu32.to_le_bytes());
    for c in 2..=(total_allocated_clusters + 1) {
        let entry_off = (c as usize) * 4;
        if entry_off + 4 <= 512 {
            fat_sector[entry_off..entry_off + 4].copy_from_slice(&0xFFFFFFFFu32.to_le_bytes());
        }
    }

    let mut bitmap_sector = vec![0u8; 512];
    let mut rem = total_allocated_clusters;
    let mut b_idx = 0;
    while rem > 0 && b_idx < 512 {
        if rem >= 8 {
            bitmap_sector[b_idx] = 0xFF;
            rem -= 8;
        } else {
            bitmap_sector[b_idx] = ((1u32 << rem) - 1) as u8;
            rem = 0;
        }
        b_idx += 1;
    }

    let upcase_sectors = (UPCASE_DATA.len() + 511) / 512;
    let mut upcase_buf = vec![0u8; upcase_sectors * 512];
    upcase_buf[..UPCASE_DATA.len()].copy_from_slice(UPCASE_DATA);

    let mut root_dir_sector = vec![0u8; 512];

    root_dir_sector[0] = 0x83;
    let label_utf16: Vec<u16> = volume_label.encode_utf16().take(11).collect();
    root_dir_sector[1] = label_utf16.len() as u8;
    for (i, &code) in label_utf16.iter().enumerate() {
        let b = code.to_le_bytes();
        root_dir_sector[2 + i * 2] = b[0];
        root_dir_sector[3 + i * 2] = b[1];
    }

    let e2_off = 32;
    root_dir_sector[e2_off] = 0x81;
    root_dir_sector[e2_off + 1] = 0x00;
    root_dir_sector[e2_off + 0x14..e2_off + 0x18].copy_from_slice(&2u32.to_le_bytes());
    root_dir_sector[e2_off + 0x18..e2_off + 0x20].copy_from_slice(&(bitmap_len as u64).to_le_bytes());

    let e3_off = 64;
    root_dir_sector[e3_off] = 0x82;
    root_dir_sector[e3_off + 0x04..e3_off + 0x08].copy_from_slice(&0xE619D30Du32.to_le_bytes());
    root_dir_sector[e3_off + 0x14..e3_off + 0x18].copy_from_slice(&upcase_cluster.to_le_bytes());
    root_dir_sector[e3_off + 0x18..e3_off + 0x20].copy_from_slice(&(UPCASE_DATA.len() as u64).to_le_bytes());

    let mut chunks = Vec::new();
    let boot_offset = part_start_sector * 512;
    chunks.push((boot_offset, full_boot_region));

    let fat_byte_offset = (part_start_sector + fat_offset as u64) * 512;
    chunks.push((fat_byte_offset, fat_sector));

    let bitmap_byte_offset = (part_start_sector + cluster_heap_offset as u64) * 512;
    chunks.push((bitmap_byte_offset, bitmap_sector));

    let upcase_byte_offset = (part_start_sector
        + cluster_heap_offset as u64
        + (upcase_cluster - 2) as u64 * sectors_per_cluster as u64)
        * 512;
    chunks.push((upcase_byte_offset, upcase_buf));

    let root_byte_offset = (part_start_sector
        + cluster_heap_offset as u64
        + (root_dir_cluster - 2) as u64 * sectors_per_cluster as u64)
        * 512;
    chunks.push((root_byte_offset, root_dir_sector));

    Some(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upcase_table_checksum() {
        assert_eq!(UPCASE_DATA.len(), 5836);
        let mut chk: u32 = 0;
        for &b in UPCASE_DATA {
            chk = ((chk << 31) | (chk >> 1)).wrapping_add(b as u32);
        }
        assert_eq!(chk, 0xE619D30D);
    }

    #[test]
    fn test_exfat_metadata_generation() {
        let part_start = 2048u64;
        let part_sectors = 60_000_000u64;
        let chunks = generate_exfat_metadata(part_start, part_sectors, "Ventoy").unwrap();
        assert_eq!(chunks.len(), 5);

        let (boot_off, ref boot_bytes) = chunks[0];
        assert_eq!(boot_off, part_start * 512);
        assert_eq!(boot_bytes.len(), 24 * 512);
        assert_eq!(&boot_bytes[3..11], b"EXFAT   ");
        assert_eq!(&boot_bytes[510..512], &[0x55, 0xAA]);

        let (root_off, ref root_bytes) = chunks[4];
        assert!(root_off > boot_off);
        assert_eq!(root_bytes[0], 0x83);
        assert_eq!(root_bytes[1], 6);
        assert_eq!(root_bytes[32], 0x81);
        assert_eq!(root_bytes[64], 0x82);
    }
}
