use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Mutex;
use windows_sys::Win32::Storage::FileSystem::GetLogicalDrives;

use crate::types::*;

static LOG_MUTEX: Mutex<()> = Mutex::new(());
pub static CLI_MODE: Mutex<bool> = Mutex::new(false);

/// Append a line to the log file (log.txt or cli_log.txt)
pub fn log_message(msg: &str) {
    let _lock = LOG_MUTEX.lock().unwrap();
    let is_cli = *CLI_MODE.lock().unwrap();
    let log_file_name = if is_cli { VENTOY_CLI_LOG } else { VENTOY_FILE_LOG };

    let now = std::time::SystemTime::now();
    let datetime = match now.duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => {
            let secs = d.as_secs();
            let millis = d.subsec_millis();
            format!("[{}.{:03}] {}", secs, millis, msg)
        }
        Err(_) => msg.to_string(),
    };

    println!("{}", datetime);

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_file_name) {
        let _ = writeln!(file, "{}", datetime);
    }
}

#[macro_export]
macro_rules! ventoy_log {
    ($($arg:tt)*) => {
        $crate::utility::log_message(&format!($($arg)*))
    };
}

pub fn is_path_exist(dir: bool, path: &str) -> bool {
    let p = Path::new(path);
    if dir {
        p.is_dir()
    } else {
        p.is_file()
    }
}

pub fn get_human_readable_gb_size(size_bytes: u64) -> u32 {
    let gb = (size_bytes as f64) / (1024.0 * 1024.0 * 1024.0);
    gb.round() as u32
}

pub fn read_whole_file(path: &str) -> std::io::Result<Vec<u8>> {
    let mut f = File::open(path)?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    Ok(buf)
}

pub fn save_buf_to_file(path: &str, buf: &[u8]) -> std::io::Result<()> {
    let mut f = File::create(path)?;
    f.write_all(buf)?;
    Ok(())
}

pub fn find_asset_path(rel_path: &str) -> Option<std::path::PathBuf> {
    let direct = Path::new(rel_path);
    if direct.exists() {
        return Some(direct.to_path_buf());
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let p1 = parent.join(rel_path);
            if p1.exists() {
                return Some(p1);
            }
            if let Some(p2_parent) = parent.parent().and_then(|p| p.parent()) {
                let p2 = p2_parent.join(rel_path);
                if p2.exists() {
                    return Some(p2);
                }
            }
        }
    }
    None
}

/// Reads version from `ventoy/version`
pub fn get_local_ventoy_version() -> String {
    let candidates = ["ventoy/version", "ventoy\\version", "version"];
    for cand in candidates {
        if let Some(path) = find_asset_path(cand) {
            if let Ok(content) = std::fs::read_to_string(path) {
                let trimmed = content.trim().to_string();
                if !trimmed.is_empty() {
                    return trimmed;
                }
            }
        }
    }
    "1.0.99".to_string()
}

pub fn get_bus_type_string(bus_type: STORAGE_BUS_TYPE) -> &'static str {
    match bus_type {
        STORAGE_BUS_TYPE::BusTypeUsb => "USB",
        STORAGE_BUS_TYPE::BusTypeScsi => "SCSI",
        STORAGE_BUS_TYPE::BusTypeAta => "ATA",
        STORAGE_BUS_TYPE::BusTypeSata => "SATA",
        STORAGE_BUS_TYPE::BusTypeNvme => "NVMe",
        STORAGE_BUS_TYPE::BusTypeSd => "SD",
        STORAGE_BUS_TYPE::BusTypeMmc => "MMC",
        STORAGE_BUS_TYPE::BusTypeVirtual => "VIRTUAL",
        STORAGE_BUS_TYPE::BusTypeRAID => "RAID",
        _ => "UNKNOWN",
    }
}
