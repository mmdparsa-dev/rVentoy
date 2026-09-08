use std::sync::atomic::{AtomicBool, Ordering};

static FILTER_USB: AtomicBool = AtomicBool::new(true);

pub fn is_filter_usb() -> bool {
    FILTER_USB.load(Ordering::Relaxed)
}

pub fn set_filter_usb(filter: bool) {
    FILTER_USB.store(filter, Ordering::Relaxed)
}

pub fn get_ventoy_fs_name_by_type(fs: i32) -> &'static str {
    match fs {
        1 => "NTFS",
        2 => "FAT32",
        3 => "UDF",
        _ => "exFAT",
    }
}
