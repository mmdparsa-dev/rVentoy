use std::io::{Cursor, Read, Seek, SeekFrom, Write};

/// Inspects the EFI partition image in memory to check if Secure Boot is enabled.
/// If `grubx64_real.efi` exists, Secure Boot is supported.
pub fn is_secure_boot_enabled_in_image(image_bytes: &mut [u8]) -> bool {
    let cursor = Cursor::new(image_bytes);
    if let Ok(fs) = fatfs::FileSystem::new(cursor, fatfs::FsOptions::new()) {
        let root = fs.root_dir();
        if let Ok(boot_dir) = root.open_dir("EFI/BOOT") {
            for entry in boot_dir.iter().flatten() {
                if entry.file_name().eq_ignore_ascii_case("grubx64_real.efi") {
                    return true;
                }
            }
        }
    }
    false
}

/// Reads a file from a FAT image buffer into a Vec<u8>
pub fn read_file_from_image(image_bytes: &mut [u8], path: &str) -> std::io::Result<Vec<u8>> {
    let cursor = Cursor::new(image_bytes);
    let fs = fatfs::FileSystem::new(cursor, fatfs::FsOptions::new())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    let root = fs.root_dir();
    let mut file = root
        .open_file(path)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::NotFound, e.to_string()))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

/// Disables Secure Boot in the EFI partition image in-place.
/// Replaces BOOTX64.EFI and BOOTIA32.EFI with the real (non-signed) grub binaries,
/// and removes shim/MokManager files.
pub fn disable_secure_boot_in_image(image_bytes: &mut [u8]) -> std::io::Result<()> {
    let mut cursor = Cursor::new(image_bytes);
    let fs = fatfs::FileSystem::new(&mut cursor, fatfs::FsOptions::new())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    let root = fs.root_dir();

    // 1. Process x64
    let x64_real_data = {
        if let Ok(mut f) = root.open_file("EFI/BOOT/grubx64_real.efi") {
            let mut data = Vec::new();
            f.read_to_end(&mut data)?;
            Some(data)
        } else {
            None
        }
    };

    if let Some(data) = x64_real_data {
        let _ = root.remove("EFI/BOOT/BOOTX64.EFI");
        let _ = root.remove("EFI/BOOT/grubx64.efi");
        let _ = root.remove("EFI/BOOT/grubx64_real.efi");
        let _ = root.remove("EFI/BOOT/MokManager.efi");
        let _ = root.remove("EFI/BOOT/mmx64.efi");
        let _ = root.remove("ENROLL_THIS_KEY_IN_MOKMANAGER.cer");
        let _ = root.remove("EFI/BOOT/grub.efi");

        if let Ok(mut dest) = root.create_file("EFI/BOOT/BOOTX64.EFI") {
            dest.write_all(&data)?;
            dest.flush()?;
        }
    }

    // 2. Process ia32
    let ia32_real_data = {
        if let Ok(mut f) = root.open_file("EFI/BOOT/grubia32_real.efi") {
            let mut data = Vec::new();
            f.read_to_end(&mut data)?;
            Some(data)
        } else {
            None
        }
    };

    if let Some(data) = ia32_real_data {
        let _ = root.remove("EFI/BOOT/BOOTIA32.EFI");
        let _ = root.remove("EFI/BOOT/grubia32.efi");
        let _ = root.remove("EFI/BOOT/grubia32_real.efi");
        let _ = root.remove("EFI/BOOT/mmia32.efi");

        if let Ok(mut dest) = root.create_file("EFI/BOOT/BOOTIA32.EFI") {
            dest.write_all(&data)?;
            dest.flush()?;
        }
    }

    Ok(())
}

/// Formats a block device/stream as FAT with label "Ventoy"
pub fn format_fat_volume<T: Read + Write + Seek>(stream: &mut T, label: &str) -> std::io::Result<()> {
    let mut options = fatfs::FormatVolumeOptions::new();
    let mut vol_label = [0u8; 11];
    let label_bytes = label.as_bytes();
    let len = label_bytes.len().min(11);
    vol_label[..len].copy_from_slice(&label_bytes[..len]);
    for b in &mut vol_label[len..] {
        *b = b' ';
    }
    options = options.volume_label(vol_label);

    fatfs::format_volume(stream, options)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    Ok(())
}
