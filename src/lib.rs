#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, unused)]

pub mod crc32;
pub mod dialogs;
pub mod disk_service;
pub mod fat_io;
pub mod language;
pub mod phy_drive;
pub mod process;
pub mod types;
pub mod utility;
pub mod ventoy_cli;
pub mod ventoy_json;
pub mod xz;

pub use phy_drive::{
    install_ventoy_to_phy_drive, is_ventoy_phy_drive, open_physical_drive,
    scan_all_physical_drives, update_ventoy_to_phy_drive, PhysicalDriveHandle,
};
pub use types::*;
pub use utility::*;
