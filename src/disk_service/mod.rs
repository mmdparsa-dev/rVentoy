pub mod diskpart;
pub mod vds;
pub mod wmsa;

pub fn disk_clean_disk(drive_index: u32) -> bool {
    vds::clean_disk(drive_index)
}

pub fn disk_delete_all_partitions(drive_index: u32) -> bool {
    vds::delete_all_partitions(drive_index)
}

pub fn disk_delete_vtoy_efi_partition(drive_index: u32, efi_part_offset: u64) -> bool {
    vds::delete_vtoy_efi_partition(drive_index, efi_part_offset)
}

pub fn disk_change_vtoy_efi_attr(drive_index: u32, offset: u64, attr: u64) -> bool {
    vds::change_vtoy_efi_attr(drive_index, offset, attr)
}

pub fn disk_change_vtoy_efi_to_esp(drive_index: u32, offset: u64) -> bool {
    vds::change_vtoy_efi_to_esp(drive_index, offset)
}

pub fn disk_change_vtoy_efi_to_basic(drive_index: u32, offset: u64) -> bool {
    vds::change_vtoy_efi_to_basic(drive_index, offset)
}

pub fn disk_format_volume(drive_letter: char, fs_name: &str, cluster_size: u32) -> bool {
    vds::format_volume(drive_letter, fs_name, cluster_size)
}

pub fn disk_format_partition(disk_number: u32, partition_number: u32, fs_name: &str, cluster_size: u32) -> bool {
    vds::format_disk_partition(disk_number, partition_number, fs_name, cluster_size)
}
