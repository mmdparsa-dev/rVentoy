use std::fs::File;
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn is_diskpart_exist() -> bool {
    let path = get_system32_path("diskpart.exe");
    path.exists()
}

pub fn get_system32_path(binary: &str) -> PathBuf {
    let windir = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
    PathBuf::from(windir).join("System32").join(binary)
}

pub fn run_diskpart_script(script: &str) -> bool {
    let temp_dir = std::env::temp_dir();
    let script_file = temp_dir.join(format!("ventoy_dp_{}.txt", std::process::id()));

    if let Ok(mut f) = File::create(&script_file) {
        if f.write_all(script.as_bytes()).is_err() {
            return false;
        }
    } else {
        return false;
    }

    let diskpart_exe = get_system32_path("diskpart.exe");
    crate::ventoy_log!("Running diskpart script:\n{}", script);

    let output = Command::new(diskpart_exe)
        .arg("/s")
        .arg(&script_file)
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    let _ = std::fs::remove_file(script_file);

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            if !stdout.trim().is_empty() {
                crate::ventoy_log!("diskpart stdout:\n{}", stdout.trim());
            }
            if !stderr.trim().is_empty() {
                crate::ventoy_log!("diskpart stderr:\n{}", stderr.trim());
            }
            let stdout_lower = stdout.to_lowercase();
            let has_error = stdout_lower.contains("diskpart has encountered an error")
                || stdout_lower.contains("error:")
                || stdout_lower.contains("clean is not allowed");
            out.status.success() && !has_error
        }
        Err(e) => {
            crate::ventoy_log!("Failed to run diskpart: {}", e);
            false
        }
    }
}

pub fn clean_disk(drive_index: u32) -> bool {
    let script = format!("select disk {}\nclean\n", drive_index);
    run_diskpart_script(&script)
}

pub fn format_volume(drive_letter: char, fs_name: &str, cluster_size: u32) -> bool {
    let mut script = format!("rescan\nselect volume {}\nformat fs={} quick label=Ventoy", drive_letter, fs_name);
    if cluster_size > 0 {
        script.push_str(&format!(" unit={}", cluster_size));
    }
    script.push('\n');
    run_diskpart_script(&script)
}

pub fn format_disk_partition(disk_number: u32, partition_number: u32, fs_name: &str, cluster_size: u32) -> bool {
    let mut script = format!(
        "rescan\nselect disk {}\nselect partition {}\nformat fs={} quick label=Ventoy",
        disk_number, partition_number, fs_name
    );
    if cluster_size > 0 {
        script.push_str(&format!(" unit={}", cluster_size));
    }
    script.push_str("\nassign\n");
    run_diskpart_script(&script)
}
