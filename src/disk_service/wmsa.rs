use std::os::windows::process::CommandExt;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn run_powershell_command(cmd: &str) -> bool {
    let windir = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
    let ps_exe = format!("{}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe", windir);

    crate::ventoy_log!("Running powershell: {}", cmd);
    let status = Command::new(ps_exe)
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-Command")
        .arg(cmd)
        .creation_flags(CREATE_NO_WINDOW)
        .status();

    match status {
        Ok(s) => s.success(),
        Err(e) => {
            crate::ventoy_log!("Failed to execute powershell: {}", e);
            false
        }
    }
}

pub fn clean_disk(drive_index: u32) -> bool {
    let cmd = format!("Clear-Disk -Number {} -RemoveData -RemoveOEM -Confirm:$false", drive_index);
    run_powershell_command(&cmd)
}

pub fn delete_efi_partition(drive_index: u32, partition_number: u32) -> bool {
    let cmd = format!(
        "Remove-Partition -DiskNumber {} -PartitionNumber {} -Confirm:$false",
        drive_index, partition_number
    );
    run_powershell_command(&cmd)
}

pub fn change_partition_type(drive_index: u32, partition_number: u32, type_guid: &str) -> bool {
    let cmd = format!(
        "Set-Partition -DiskNumber {} -PartitionNumber {} -GptType \"{{{}}}\"",
        drive_index, partition_number, type_guid
    );
    run_powershell_command(&cmd)
}

pub fn format_volume(drive_letter: char, fs_name: &str, cluster_size: u32) -> bool {
    let mut cmd = format!(
        "Format-Volume -DriveLetter {} -FileSystem {} -NewFileSystemLabel 'Ventoy' -Confirm:$false",
        drive_letter, fs_name
    );
    if cluster_size > 0 {
        cmd.push_str(&format!(" -AllocationUnitSize {}", cluster_size));
    }
    run_powershell_command(&cmd)
}

pub fn format_disk_partition(disk_number: u32, partition_number: u32, fs_name: &str, cluster_size: u32) -> bool {
    let mut cmd = format!(
        "Get-Partition -DiskNumber {} -PartitionNumber {} | Format-Volume -FileSystem {} -NewFileSystemLabel 'Ventoy' -Confirm:$false",
        disk_number, partition_number, fs_name
    );
    if cluster_size > 0 {
        cmd.push_str(&format!(" -AllocationUnitSize {}", cluster_size));
    }
    let success = run_powershell_command(&cmd);
    if success {
        let assign_cmd = format!(
            "Get-Partition -DiskNumber {} -PartitionNumber {} | Add-PartitionAccessPath -AssignDriveLetter -ErrorAction SilentlyContinue",
            disk_number, partition_number
        );
        let _ = run_powershell_command(&assign_cmd);
    }
    success
}
