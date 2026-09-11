#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, unused)]

pub mod crc32;
pub mod dialogs;
pub mod disk_service;
pub mod exfat;
pub mod fat_io;
pub mod language;
pub mod phy_drive;
pub mod types;
pub mod utility;
pub mod ventoy_cli;
pub mod ventoy_json;
pub mod xz;

pub use phy_drive::{
    get_system_drive_disk_number, install_ventoy_to_phy_drive, is_elevated, is_system_drive,
    is_ventoy_phy_drive, open_physical_drive, preload_assets_in_background,
    scan_all_physical_drives, uninstall_ventoy_from_phy_drive, update_ventoy_to_phy_drive,
    PhysicalDriveHandle,
};
pub use types::*;
pub use utility::*;
