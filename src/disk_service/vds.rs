use super::{diskpart, wmsa};

pub fn clean_disk(drive_index: u32) -> bool {
    // Attempt diskpart / storage service
    if diskpart::clean_disk(drive_index) {
        return true;
    }
    wmsa::clean_disk(drive_index)
}

pub fn delete_all_partitions(drive_index: u32) -> bool {
    clean_disk(drive_index)
}

pub fn delete_vtoy_efi_partition(drive_index: u32, _offset: u64) -> bool {
    // Partition 2 is typically the EFI partition in Ventoy
    if wmsa::delete_efi_partition(drive_index, 2) {
        return true;
    }
    let script = format!("select disk {}\nselect partition 2\ndelete partition override\n", drive_index);
    diskpart::run_diskpart_script(&script)
}

pub fn change_vtoy_efi_attr(drive_index: u32, _offset: u64, attr: u64) -> bool {
    let script = format!("select disk {}\nselect partition 2\ngpt attributes={:#x}\n", drive_index, attr);
    diskpart::run_diskpart_script(&script)
}

pub fn change_vtoy_efi_to_esp(drive_index: u32, _offset: u64) -> bool {
    // ESP GUID: c12a7328-f81f-11d2-ba4b-00a0c93ec93b
    if wmsa::change_partition_type(drive_index, 2, "c12a7328-f81f-11d2-ba4b-00a0c93ec93b") {
        return true;
    }
    let script = format!("select disk {}\nselect partition 2\nset id=c12a7328-f81f-11d2-ba4b-00a0c93ec93b\n", drive_index);
    diskpart::run_diskpart_script(&script)
}

pub fn change_vtoy_efi_to_basic(drive_index: u32, _offset: u64) -> bool {
    // Basic data GUID: ebd0a0a2-b9e5-4433-87c0-68b6b72699c7
    if wmsa::change_partition_type(drive_index, 2, "ebd0a0a2-b9e5-4433-87c0-68b6b72699c7") {
        return true;
    }
    let script = format!("select disk {}\nselect partition 2\nset id=ebd0a0a2-b9e5-4433-87c0-68b6b72699c7\n", drive_index);
    diskpart::run_diskpart_script(&script)
}

pub fn format_volume(drive_letter: char, fs_name: &str, cluster_size: u32) -> bool {
    if diskpart::format_volume(drive_letter, fs_name, cluster_size) {
        return true;
    }
    wmsa::format_volume(drive_letter, fs_name, cluster_size)
}

pub fn format_disk_partition(disk_number: u32, partition_number: u32, fs_name: &str, cluster_size: u32) -> bool {
    if diskpart::format_disk_partition(disk_number, partition_number, fs_name, cluster_size) {
        return true;
    }
    wmsa::format_disk_partition(disk_number, partition_number, fs_name, cluster_size)
}
