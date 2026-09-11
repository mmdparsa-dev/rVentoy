#![windows_subsystem = "windows"]

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use windows_reactor::*;
use VentoyCore::dialogs::*;
use VentoyCore::phy_drive::*;
use VentoyCore::types::{PHY_DRIVE_INFO, STORAGE_BUS_TYPE};
use VentoyCore::utility::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OperationKind {
    Install,
    Update,
    Uninstall,
}

#[derive(Clone)]
struct OperationState {
    progress: Arc<AtomicI32>,
    status: Arc<Mutex<String>>,
    finished: Arc<AtomicBool>,
    result: Arc<Mutex<Option<(bool, String)>>>,
    kind: OperationKind,
}

#[derive(Clone)]
pub struct DriveItem {
    pub id: i32,
    pub phy_drive: i32,
    pub display_text: String,
    pub ventoy_version: String,
    pub size_in_bytes: u64,
    pub is_removable: bool,
    pub is_system_drive: bool,
    pub bus_type_str: &'static str,
    pub model_str: String,
    pub raw_drive: PHY_DRIVE_INFO,
}

impl std::fmt::Debug for DriveItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DriveItem")
            .field("id", &self.id)
            .field("phy_drive", &self.phy_drive)
            .field("display_text", &self.display_text)
            .field("ventoy_version", &self.ventoy_version)
            .field("size_in_bytes", &self.size_in_bytes)
            .field("is_removable", &self.is_removable)
            .field("is_system_drive", &self.is_system_drive)
            .finish()
    }
}

impl PartialEq for DriveItem {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.phy_drive == other.phy_drive
            && self.display_text == other.display_text
            && self.ventoy_version == other.ventoy_version
            && self.size_in_bytes == other.size_in_bytes
            && self.is_removable == other.is_removable
            && self.is_system_drive == other.is_system_drive
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppPage {
    Installer,
    Settings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavTransition {
    Idle,
    Exiting(AppPage),
    Entering,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveDialog {
    None,
    UnlockProtection,
    ConfirmInstall,
    ConfirmUninstall,
}

#[derive(Clone, Debug)]
pub enum Msg {
    NavigateTo(AppPage),
    NavSwitchPage(AppPage),
    NavFinishTransition,
    Refresh,
    SelectDevice(Option<usize>),
    Install,
    ConfirmInstall,
    CancelInstall,
    Update,
    Uninstall,
    ConfirmUninstall,
    CancelUninstall,
    InstallFinished(bool, String),
    UpdateFinished(bool, String),
    UninstallFinished(bool, String),
    OperationTick,
    DismissInfo,
    RestartAdmin,
    ToggleLockNonRemovable(bool),
    ConfirmUnlock,
    CancelUnlock,
    HoverActionBtn(Option<usize>),
}

pub struct RVentoyMainWindow {
    current_page: AppPage,
    nav_transition: NavTransition,
    lock_non_removable: bool,
    active_dialog: ActiveDialog,
    hovered_action_btn: Option<usize>,
    devices: Vec<DriveItem>,
    selected_device: Option<usize>,
    package_version: String,
    device_version: String,
    is_installing: bool,
    is_elevated: bool,
    progress: f64,
    status_text: String,
    info_message: Option<String>,
    info_severity: InfoBarSeverity,
    active_op: Option<OperationState>,
}

pub fn is_elevated() -> bool {
    VentoyCore::is_elevated()
}

pub fn restart_as_admin() {
    if let Ok(exe_path) = std::env::current_exe() {
        let _ = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!("Start-Process -FilePath '{}' -Verb RunAs", exe_path.display()),
            ])
            .spawn();
        std::process::exit(0);
    }
}

fn scan_usb_drives(lock_non_removable: bool) -> Vec<DriveItem> {
    let drives = scan_all_physical_drives();
    let mut list = Vec::new();
    for drive in drives {
        if drive.id >= 0 {
            let is_removable =
                drive.bus_type == STORAGE_BUS_TYPE::BusTypeUsb || drive.removable_media != 0;
            let is_sys = is_system_drive(drive.phy_drive);
            let bus_str = get_bus_type_string(drive.bus_type);
            let vendor = drive.vendor_str();
            let product = drive.product_str();
            let model = format!("{} {}", vendor, product).trim().to_string();
            let model_str = if model.is_empty() {
                if is_removable {
                    "USB Drive".to_string()
                } else if is_sys {
                    format!("Windows System Disk ({})", bus_str)
                } else {
                    format!("Fixed Disk ({})", bus_str)
                }
            } else {
                model
            };
            let size_gb = (drive.size_in_bytes as f64) / (1024.0 * 1024.0 * 1024.0);
            let ver = drive.version_str();

            let tag = if is_sys {
                "[System Disk - Protected]".to_string()
            } else if is_removable {
                "[Removable]".to_string()
            } else if lock_non_removable {
                "[Non-Removable - Locked]".to_string()
            } else {
                "[Non-Removable]".to_string()
            };

            let display = format!(
                "PhysicalDrive{} - {} ({:.1} GB) {}",
                drive.phy_drive, model_str, size_gb, tag
            );
            list.push(DriveItem {
                id: drive.id,
                phy_drive: drive.phy_drive,
                display_text: display,
                ventoy_version: ver,
                size_in_bytes: drive.size_in_bytes,
                is_removable,
                is_system_drive: is_sys,
                bus_type_str: bus_str,
                model_str,
                raw_drive: drive,
            });
        }
    }
    list
}

impl RVentoyMainWindow {
    fn finish_operation(&mut self, _kind: OperationKind, success: bool, msg: String) {
        self.is_installing = false;
        self.progress = if success { 100.0 } else { 0.0 };
        self.status_text = msg.clone();
        self.info_message = Some(msg);
        self.info_severity = if success {
            InfoBarSeverity::Success
        } else {
            InfoBarSeverity::Error
        };
        self.devices = scan_usb_drives(self.lock_non_removable);
        if let Some(idx) = self.selected_device {
            if let Some(dev) = self.devices.get(idx) {
                self.device_version = if dev.ventoy_version.is_empty() {
                    "--".to_string()
                } else {
                    dev.ventoy_version.clone()
                };
            }
        }
    }
}

impl Component for RVentoyMainWindow {
    type Input = ();
    type Message = Msg;

    fn create(_input: &(), _context: &ComponentContext<Self>) -> Self {
        let lock_non_removable = true;
        set_filter_usb(true);
        let devices = scan_usb_drives(lock_non_removable);
        let package_version = get_local_ventoy_version();

        // Safe auto-selection: only select the first removable, non-system drive if available
        let selected_device = devices.iter().position(|d| d.is_removable && !d.is_system_drive);
        let device_version = if let Some(idx) = selected_device {
            let ver = &devices[idx].ventoy_version;
            if ver.is_empty() {
                "--".to_string()
            } else {
                ver.clone()
            }
        } else {
            "--".to_string()
        };

        let is_elevated = is_elevated();

        Self {
            current_page: AppPage::Installer,
            nav_transition: NavTransition::Idle,
            lock_non_removable,
            active_dialog: ActiveDialog::None,
            hovered_action_btn: None,
            devices,
            selected_device,
            package_version,
            device_version,
            is_installing: false,
            is_elevated,
            progress: 0.0,
            status_text: String::new(),
            info_message: None,
            info_severity: InfoBarSeverity::Informational,
            active_op: None,
        }
    }

    fn update(&mut self, message: Self::Message, context: &ComponentContext<Self>) {
        match message {
            Msg::NavigateTo(target_page) => {
                if target_page != self.current_page && self.nav_transition == NavTransition::Idle {
                    self.nav_transition = NavTransition::Exiting(target_page);
                    context.spawn_background(move |_cancel| {
                        std::thread::sleep(Duration::from_millis(110));
                        Msg::NavSwitchPage(target_page)
                    });
                }
            }
            Msg::NavSwitchPage(target_page) => {
                self.current_page = target_page;
                self.nav_transition = NavTransition::Entering;
                context.spawn_background(move |_cancel| {
                    std::thread::sleep(Duration::from_millis(25));
                    Msg::NavFinishTransition
                });
            }
            Msg::NavFinishTransition => {
                self.nav_transition = NavTransition::Idle;
            }
            Msg::Refresh => {
                self.devices = scan_usb_drives(self.lock_non_removable);
                if let Some(idx) = self.selected_device {
                    if let Some(dev) = self.devices.get(idx) {
                        if self.lock_non_removable && !dev.is_removable {
                            self.selected_device = None;
                            self.device_version = "--".to_string();
                        } else {
                            self.device_version = if dev.ventoy_version.is_empty() {
                                "--".to_string()
                            } else {
                                dev.ventoy_version.clone()
                            };
                        }
                    } else {
                        self.selected_device = self
                            .devices
                            .iter()
                            .position(|d| !self.lock_non_removable || d.is_removable);
                        self.device_version = self
                            .selected_device
                            .and_then(|i| self.devices.get(i))
                            .map(|d| {
                                if d.ventoy_version.is_empty() {
                                    "--".to_string()
                                } else {
                                    d.ventoy_version.clone()
                                }
                            })
                            .unwrap_or_else(|| "--".to_string());
                    }
                } else {
                    self.selected_device = self
                        .devices
                        .iter()
                        .position(|d| !self.lock_non_removable || d.is_removable);
                    self.device_version = self
                        .selected_device
                        .and_then(|i| self.devices.get(i))
                        .map(|d| {
                            if d.ventoy_version.is_empty() {
                                "--".to_string()
                            } else {
                                d.ventoy_version.clone()
                            }
                        })
                        .unwrap_or_else(|| "--".to_string());
                }
            }
            Msg::SelectDevice(opt_idx) => {
                if let Some(idx) = opt_idx {
                    if let Some(dev) = self.devices.get(idx) {
                        if dev.is_system_drive {
                            // Absolute block on Windows system drive!
                            self.selected_device = None;
                            self.device_version = "--".to_string();
                            self.info_message = Some(format!(
                                "PhysicalDrive{} is the active Windows system disk (C:) and cannot be selected. Installing Ventoy here would destroy your Windows OS. Please connect a USB flash drive.",
                                dev.phy_drive
                            ));
                            self.info_severity = InfoBarSeverity::Error;
                            return;
                        }
                        if self.lock_non_removable && !dev.is_removable {
                            // Block selecting fixed disk while locked!
                            self.selected_device = None;
                            self.device_version = "--".to_string();
                            self.info_message = Some(format!(
                                "PhysicalDrive{} is a non-removable fixed disk and cannot be selected while protection is active. Turn off 'Lock Non-Removable Disks' in Settings if you really wish to use this disk.",
                                dev.phy_drive
                            ));
                            self.info_severity = InfoBarSeverity::Warning;
                            return;
                        }
                        self.selected_device = Some(idx);
                        self.device_version = if dev.ventoy_version.is_empty() {
                            "--".to_string()
                        } else {
                            dev.ventoy_version.clone()
                        };
                    }
                } else {
                    self.selected_device = None;
                    self.device_version = "--".to_string();
                }
            }
            Msg::ToggleLockNonRemovable(enabled) => {
                if !enabled {
                    // User is trying to turn off protection -> Open warning dialog!
                    self.active_dialog = ActiveDialog::UnlockProtection;
                } else {
                    // Re-enabling protection is safe
                    self.lock_non_removable = true;
                    set_filter_usb(true);
                    if let Some(idx) = self.selected_device {
                        if let Some(dev) = self.devices.get(idx) {
                            if !dev.is_removable {
                                self.selected_device = None;
                                self.device_version = "--".to_string();
                            }
                        }
                    }
                    self.devices = scan_usb_drives(self.lock_non_removable);
                    self.info_message = Some(
                        "Non-removable disk protection is active. Fixed internal drives are locked."
                            .to_string(),
                    );
                    self.info_severity = InfoBarSeverity::Informational;
                }
            }
            Msg::ConfirmUnlock => {
                self.active_dialog = ActiveDialog::None;
                self.lock_non_removable = false;
                set_filter_usb(false);
                self.devices = scan_usb_drives(self.lock_non_removable);
                self.info_message = Some(
                    "WARNING: Non-removable disk protection disabled! Fixed internal drives can now be selected. Exercise extreme caution!"
                        .to_string(),
                );
                self.info_severity = InfoBarSeverity::Warning;
            }
            Msg::CancelUnlock => {
                self.active_dialog = ActiveDialog::None;
            }
            Msg::Install => {
                if let Some(idx) = self.selected_device {
                    if let Some(dev) = self.devices.get(idx) {
                        if dev.is_system_drive {
                            self.info_message = Some(
                                "Cannot install: Selected device is the active Windows system disk (C:)."
                                    .to_string(),
                            );
                            self.info_severity = InfoBarSeverity::Error;
                            return;
                        }
                        if self.lock_non_removable && !dev.is_removable {
                            self.info_message = Some(
                                "Cannot install: Selected device is non-removable and locked."
                                    .to_string(),
                            );
                            self.info_severity = InfoBarSeverity::Error;
                            return;
                        }
                        // Prompt with warning confirmation dialog!
                        self.active_dialog = ActiveDialog::ConfirmInstall;
                    }
                }
            }
            Msg::CancelInstall => {
                self.active_dialog = ActiveDialog::None;
            }
            Msg::ConfirmInstall => {
                self.active_dialog = ActiveDialog::None;
                if let Some(idx) = self.selected_device {
                    if let Some(dev) = self.devices.get(idx) {
                        if dev.is_system_drive {
                            return;
                        }
                        if self.lock_non_removable && !dev.is_removable {
                            return;
                        }
                        let drive = dev.raw_drive;
                        let part_style = 0;
                        let secure_boot = true;
                        self.is_installing = true;
                        self.progress = 0.0;
                        self.status_text = format!(
                            "Installing rVentoy to PhysicalDrive{}...",
                            dev.phy_drive
                        );
                        self.info_message = None;

                        let progress_arc = Arc::new(AtomicI32::new(0));
                        let status_arc = Arc::new(Mutex::new(format!(
                            "Installing rVentoy to PhysicalDrive{}...",
                            dev.phy_drive
                        )));
                        let finished_arc = Arc::new(AtomicBool::new(false));
                        let result_arc = Arc::new(Mutex::new(None));

                        self.active_op = Some(OperationState {
                            progress: Arc::clone(&progress_arc),
                            status: Arc::clone(&status_arc),
                            finished: Arc::clone(&finished_arc),
                            result: Arc::clone(&result_arc),
                            kind: OperationKind::Install,
                        });

                        let _ = std::thread::Builder::new()
                            .stack_size(8 * 1024 * 1024)
                            .spawn({
                            let progress = Arc::clone(&progress_arc);
                            let status = Arc::clone(&status_arc);
                            let finished = Arc::clone(&finished_arc);
                            let result = Arc::clone(&result_arc);
                            move || {
                                let cb = |pct: i32, msg: &str| {
                                    progress.store(pct.clamp(0, 100), Ordering::SeqCst);
                                    if let Ok(mut lock) = status.lock() {
                                        *lock = msg.to_string();
                                    }
                                };
                                let res =
                                    install_ventoy_to_phy_drive(&drive, part_style, secure_boot, Some(&cb));
                                let outcome = if res == 0 {
                                    (true, "rVentoy installed successfully!".to_string())
                                } else {
                                    let err_msg = match res {
                                        -10 => "Operation blocked: Target is the active Windows system drive (C:).".to_string(),
                                        -1 => "Asset error: Could not find or decompress ventoy.disk.img.xz.".to_string(),
                                        -2 => "Partition cleanup failed. Make sure rVentoy is Run as Administrator and the USB drive is not open in Explorer.".to_string(),
                                        -3 => "Access denied: Failed to open drive with write permissions. Run as Administrator.".to_string(),
                                        -4 => "Write error: Failed to write EFI partition image to disk.".to_string(),
                                        -5 => "Write error: Failed to write partition table or boot sector.".to_string(),
                                        -6 | -7 => "Write error: Failed to write backup GPT header/tables.".to_string(),
                                        -8 => "Formatting error: Failed to format data partition (exFAT).".to_string(),
                                        _ => format!("rVentoy installation failed with error code {}", res),
                                    };
                                    (false, err_msg)
                                };
                                if let Ok(mut lock) = result.lock() {
                                    *lock = Some(outcome);
                                }
                                finished.store(true, Ordering::SeqCst);
                            }
                        });

                        context.spawn_background(|_cancel| {
                            std::thread::sleep(Duration::from_millis(50));
                            Msg::OperationTick
                        });
                    }
                }
            }
            Msg::Update => {
                if let Some(idx) = self.selected_device {
                    if let Some(dev) = self.devices.get(idx) {
                        if dev.is_system_drive {
                            self.info_message = Some(
                                "Cannot update: Selected device is the active Windows system disk (C:)."
                                    .to_string(),
                            );
                            self.info_severity = InfoBarSeverity::Error;
                            return;
                        }
                        if self.lock_non_removable && !dev.is_removable {
                            self.info_message = Some(
                                "Cannot update: Selected device is non-removable and locked."
                                    .to_string(),
                            );
                            self.info_severity = InfoBarSeverity::Error;
                            return;
                        }
                        let drive = dev.raw_drive;
                        self.is_installing = true;
                        self.progress = 0.0;
                        self.status_text =
                            format!("Updating rVentoy on PhysicalDrive{}...", dev.phy_drive);
                        self.info_message = None;

                        let progress_arc = Arc::new(AtomicI32::new(0));
                        let status_arc = Arc::new(Mutex::new(format!(
                            "Updating rVentoy on PhysicalDrive{}...",
                            dev.phy_drive
                        )));
                        let finished_arc = Arc::new(AtomicBool::new(false));
                        let result_arc = Arc::new(Mutex::new(None));

                        self.active_op = Some(OperationState {
                            progress: Arc::clone(&progress_arc),
                            status: Arc::clone(&status_arc),
                            finished: Arc::clone(&finished_arc),
                            result: Arc::clone(&result_arc),
                            kind: OperationKind::Update,
                        });

                        let _ = std::thread::Builder::new()
                            .stack_size(8 * 1024 * 1024)
                            .spawn({
                            let progress = Arc::clone(&progress_arc);
                            let status = Arc::clone(&status_arc);
                            let finished = Arc::clone(&finished_arc);
                            let result = Arc::clone(&result_arc);
                            move || {
                                let cb = |pct: i32, msg: &str| {
                                    progress.store(pct.clamp(0, 100), Ordering::SeqCst);
                                    if let Ok(mut lock) = status.lock() {
                                        *lock = msg.to_string();
                                    }
                                };
                                let res = update_ventoy_to_phy_drive(&drive, Some(&cb));
                                let outcome = if res == 0 {
                                    (true, "rVentoy updated successfully!".to_string())
                                } else {
                                    let err_msg = match res {
                                        -10 => "Operation blocked: Target is the active Windows system drive (C:).".to_string(),
                                        -1 => "The selected drive does not have an existing Ventoy installation.".to_string(),
                                        -2 => "Asset error: Could not find ventoy.disk.img.xz.".to_string(),
                                        -3 => "Access denied: Failed to open drive with write permissions. Run as Administrator.".to_string(),
                                        -4 => "Write error: Failed to write updated EFI partition.".to_string(),
                                        _ => format!("rVentoy update failed with error code {}", res),
                                    };
                                    (false, err_msg)
                                };
                                if let Ok(mut lock) = result.lock() {
                                    *lock = Some(outcome);
                                }
                                finished.store(true, Ordering::SeqCst);
                            }
                        });

                        context.spawn_background(|_cancel| {
                            std::thread::sleep(Duration::from_millis(50));
                            Msg::OperationTick
                        });
                    }
                }
            }
            Msg::Uninstall => {
                if let Some(idx) = self.selected_device {
                    if let Some(dev) = self.devices.get(idx) {
                        if dev.is_system_drive {
                            self.info_message = Some(
                                "Cannot uninstall: Selected device is the active Windows system disk (C:)."
                                    .to_string(),
                            );
                            self.info_severity = InfoBarSeverity::Error;
                            return;
                        }
                        if self.lock_non_removable && !dev.is_removable {
                            self.info_message = Some(
                                "Cannot uninstall: Selected device is non-removable and locked."
                                    .to_string(),
                            );
                            self.info_severity = InfoBarSeverity::Error;
                            return;
                        }
                        self.active_dialog = ActiveDialog::ConfirmUninstall;
                    }
                }
            }
            Msg::CancelUninstall => {
                self.active_dialog = ActiveDialog::None;
            }
            Msg::ConfirmUninstall => {
                self.active_dialog = ActiveDialog::None;
                self.current_page = AppPage::Installer;
                if let Some(idx) = self.selected_device {
                    if let Some(dev) = self.devices.get(idx) {
                        if dev.is_system_drive {
                            return;
                        }
                        if self.lock_non_removable && !dev.is_removable {
                            return;
                        }
                        let drive = dev.raw_drive;
                        self.is_installing = true;
                        self.progress = 0.0;
                        self.status_text = format!(
                            "Uninstalling rVentoy from PhysicalDrive{}...",
                            dev.phy_drive
                        );
                        self.info_message = None;

                        let progress_arc = Arc::new(AtomicI32::new(0));
                        let status_arc = Arc::new(Mutex::new(format!(
                            "Uninstalling rVentoy from PhysicalDrive{}...",
                            dev.phy_drive
                        )));
                        let finished_arc = Arc::new(AtomicBool::new(false));
                        let result_arc = Arc::new(Mutex::new(None));

                        self.active_op = Some(OperationState {
                            progress: Arc::clone(&progress_arc),
                            status: Arc::clone(&status_arc),
                            finished: Arc::clone(&finished_arc),
                            result: Arc::clone(&result_arc),
                            kind: OperationKind::Uninstall,
                        });

                        let _ = std::thread::Builder::new()
                            .stack_size(8 * 1024 * 1024)
                            .spawn({
                            let progress = Arc::clone(&progress_arc);
                            let status = Arc::clone(&status_arc);
                            let finished = Arc::clone(&finished_arc);
                            let result = Arc::clone(&result_arc);
                            move || {
                                let cb = |pct: i32, msg: &str| {
                                    progress.store(pct.clamp(0, 100), Ordering::SeqCst);
                                    if let Ok(mut lock) = status.lock() {
                                        *lock = msg.to_string();
                                    }
                                };
                                let res = uninstall_ventoy_from_phy_drive(&drive, Some(&cb));
                                let outcome = if res == 0 {
                                    (true, "rVentoy uninstalled successfully! Drive restored as standard storage.".to_string())
                                } else {
                                    let err_msg = match res {
                                        -10 => "Operation blocked: Target is the active Windows system drive (C:).".to_string(),
                                        -3 => "Access denied: Failed to open drive with write permissions. Run as Administrator.".to_string(),
                                        -5 => "Partition error: Failed to create clean primary partition.".to_string(),
                                        -8 => "Formatting error: Failed to format restored partition (exFAT).".to_string(),
                                        _ => format!("rVentoy uninstall failed with error code {}", res),
                                    };
                                    (false, err_msg)
                                };
                                if let Ok(mut lock) = result.lock() {
                                    *lock = Some(outcome);
                                }
                                finished.store(true, Ordering::SeqCst);
                            }
                        });

                        context.spawn_background(|_cancel| {
                            std::thread::sleep(Duration::from_millis(50));
                            Msg::OperationTick
                        });
                    }
                }
            }
            Msg::OperationTick => {
                if let Some(ref op) = self.active_op {
                    let pct = op.progress.load(Ordering::SeqCst) as f64;
                    if let Ok(lock) = op.status.lock() {
                        self.status_text = lock.clone();
                    }
                    self.progress = pct;

                    if op.finished.load(Ordering::SeqCst) {
                        let kind = op.kind;
                        let outcome = op.result.lock().ok().and_then(|mut r| r.take());
                        self.active_op = None;
                        if let Some((success, msg)) = outcome {
                            self.finish_operation(kind, success, msg);
                        }
                    } else {
                        context.spawn_background(|_cancel| {
                            std::thread::sleep(Duration::from_millis(50));
                            Msg::OperationTick
                        });
                    }
                }
            }
            Msg::InstallFinished(success, msg) => {
                self.finish_operation(OperationKind::Install, success, msg);
            }
            Msg::UpdateFinished(success, msg) => {
                self.finish_operation(OperationKind::Update, success, msg);
            }
            Msg::UninstallFinished(success, msg) => {
                self.finish_operation(OperationKind::Uninstall, success, msg);
            }
            Msg::HoverActionBtn(opt) => {
                self.hovered_action_btn = opt;
            }
            Msg::DismissInfo => {
                self.info_message = None;
            }
            Msg::RestartAdmin => {
                restart_as_admin();
            }
        }
    }

    fn view(&self, _input: &(), context: &mut ViewContext<Self>) -> View {
        context.window_title("rVentoy");
        context.window_visuals(
            WindowVisuals::new()
                .backdrop(WindowBackdrop::Mica)
                .icon("assets/app.ico")
                .client_size(560.0, 720.0),
        );

        // --- Content Header ---
        let (header_icon, header_action, title_text, subtitle_text) = match self.current_page {
            AppPage::Installer => (
                Symbol::Setting,
                Msg::NavigateTo(AppPage::Settings),
                "rVentoy",
                "A modern multiboot tool in Rust & WinUI 3",
            ),
            AppPage::Settings => (
                Symbol::Back,
                Msg::NavigateTo(AppPage::Installer),
                "Settings & Safety",
                "Configure safety protection and application options",
            ),
        };

        let content_header = Grid::new()
            .grid_row(0)
            .columns([
                GridLength::Auto,
                GridLength::Star(1.0),
            ])
            .column_spacing(12.0)
            .margin(Thickness::new(0.0, 0.0, 0.0, 16.0))
            .children((
                Button::new()
                    .grid_column(0)
                    .style(ButtonStyle::Subtle)
                    .width(48.0)
                    .height(40.0)
                    .horizontal_content_alignment(HorizontalAlignment::Center)
                    .vertical_content_alignment(VerticalAlignment::Center)
                    .vertical_alignment(VerticalAlignment::Center)
                    .is_enabled(!self.is_installing && self.nav_transition == NavTransition::Idle)
                    .on_click(context.message(header_action))
                    .content(
                        SymbolIcon::new()
                            .symbol(header_icon)
                            .horizontal_alignment(HorizontalAlignment::Center)
                            .vertical_alignment(VerticalAlignment::Center),
                    ),
                StackPanel::new()
                    .grid_column(1)
                    .spacing(2.0)
                    .vertical_alignment(VerticalAlignment::Center)
                    .children((
                        TextBlock::new()
                            .text(title_text)
                            .font_size(22.0)
                            .font_weight(FontWeight::SEMI_BOLD),
                        TextBlock::new()
                            .text(subtitle_text)
                            .font_size(12.0)
                            .opacity(0.7),
                    )),
            ));

        let progress_section: View = if self.is_installing {
            StackPanel::new()
                .spacing(8.0)
                .children((
                    Grid::new()
                        .columns([GridLength::Star(1.0), GridLength::Auto])
                        .children((
                            TextBlock::new()
                                .grid_column(0)
                                .text(&self.status_text)
                                .font_size(12.0),
                            TextBlock::new()
                                .grid_column(1)
                                .text(format!("{:.0}%", self.progress))
                                .font_size(12.0),
                        )),
                    ProgressBar::new()
                        .minimum(0.0)
                        .maximum(100.0)
                        .value(self.progress)
                        .is_indeterminate(false),
                ))
                .into()
        } else if let Some(ref msg) = self.info_message {
            InfoBar::new()
                .title("Notice")
                .message(msg)
                .severity(self.info_severity)
                .is_open(true)
                .on_closed(context.message(Msg::DismissInfo))
                .into()
        } else {
            View::empty()
        };

        let admin_banner: View = if !self.is_elevated {
            StackPanel::new()
                .spacing(8.0)
                .children((
                    InfoBar::new()
                        .title("Administrator Privileges Required")
                        .message("Accessing physical USB drives on Windows requires Administrator privileges. Please restart rVentoy as Administrator.")
                        .severity(InfoBarSeverity::Warning)
                        .is_open(true),
                    Button::new()
                        .horizontal_alignment(HorizontalAlignment::Left)
                        .on_click(context.message(Msg::RestartAdmin))
                        .content("Restart as Administrator"),
                ))
                .into()
        } else {
            View::empty()
        };

        let selected_is_safe = self.selected_device.map_or(false, |idx| {
            if let Some(dev) = self.devices.get(idx) {
                !dev.is_system_drive && (!self.lock_non_removable || dev.is_removable)
            } else {
                false
            }
        });

        let has_device = self.selected_device.is_some()
            && !self.devices.is_empty()
            && selected_is_safe;

        // --- PAGE 1: Installer Page ---
        let page_installer: View = {
            let device_labels = self
                .devices
                .iter()
                .map(|d| d.display_text.clone())
                .collect::<Vec<_>>();

            let placeholder = if self.devices.is_empty() {
                "No storage drives found (click refresh)"
            } else {
                "Select target drive..."
            };

            let combo = ComboBox::new()
                .grid_column(0)
                .placeholder_text(placeholder)
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .is_enabled(!self.is_installing && !self.devices.is_empty())
                .items_source(device_labels)
                .selected_index(self.selected_device)
                .on_selection_changed(context.callback(|idx| Msg::SelectDevice(idx)));

            let refresh_btn = Button::new()
                .grid_column(1)
                .is_enabled(!self.is_installing)
                .on_click(context.message(Msg::Refresh))
                .content(SymbolIcon::new().symbol(Symbol::Refresh));

            let has_removable = self.devices.iter().any(|d| d.is_removable);
            let lock_hint: View = if !has_removable {
                InfoBar::new()
                    .severity(InfoBarSeverity::Informational)
                    .message("No removable USB flash drive detected. Please connect a USB drive to install Ventoy.")
                    .is_open(true)
                    .is_closable(false)
                    .into()
            } else if self.lock_non_removable {
                TextBlock::new()
                    .text("Fixed/internal drives are locked to prevent data loss. Open Settings to unlock if needed.")
                    .font_size(11.0)
                    .opacity(0.7)
                    .into()
            } else {
                InfoBar::new()
                    .severity(InfoBarSeverity::Warning)
                    .message("Protection disabled: Fixed internal drives can be selected! Windows system drive (C:) remains protected.")
                    .is_open(true)
                    .is_closable(false)
                    .into()
            };

            let device_card = Border::new()
                .background(ThemeBrush::CardBackground)
                .border_brush(ThemeBrush::CardStroke)
                .border_thickness(1.0)
                .corner_radius(8.0)
                .padding(16.0)
                .content(
                    StackPanel::new()
                        .spacing(12.0)
                        .children((
                            TextBlock::new()
                                .text("Target Device")
                                .font_size(14.0)
                                .font_weight(FontWeight::SEMI_BOLD),
                            Grid::new()
                                .columns([GridLength::Star(1.0), GridLength::Auto])
                                .column_spacing(12.0)
                                .children((combo, refresh_btn)),
                            lock_hint,
                        )),
                );

            let version_card = Border::new()
                .background(ThemeBrush::CardBackground)
                .border_brush(ThemeBrush::CardStroke)
                .border_thickness(1.0)
                .corner_radius(8.0)
                .padding(16.0)
                .content(
                    Grid::new()
                        .columns([
                            GridLength::Star(1.0),
                            GridLength::Pixel(1.0),
                            GridLength::Star(1.0),
                        ])
                        .column_spacing(16.0)
                        .children((
                            StackPanel::new()
                                .grid_column(0)
                                .spacing(6.0)
                                .children((
                                    TextBlock::new()
                                        .text("rVentoy In Package")
                                        .font_size(12.0)
                                        .opacity(0.7),
                                    TextBlock::new()
                                        .text(&self.package_version)
                                        .font_size(18.0)
                                        .font_weight(FontWeight::BOLD),
                                 )),
                            Rectangle::new()
                                .grid_column(1)
                                .fill(ThemeBrush::CardStroke)
                                .width(1.0)
                                .horizontal_alignment(HorizontalAlignment::Center),
                            StackPanel::new()
                                .grid_column(2)
                                .spacing(6.0)
                                .children((
                                    TextBlock::new()
                                        .text("rVentoy In Device")
                                        .font_size(12.0)
                                        .opacity(0.7),
                                    TextBlock::new()
                                        .text(&self.device_version)
                                        .font_size(18.0)
                                        .font_weight(FontWeight::BOLD),
                                )),
                        )),
                );

            let install_btn = Button::new()
                .grid_column(0)
                .style(ButtonStyle::Accent)
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .height(42.0)
                .is_enabled(has_device && !self.is_installing)
                .on_click(context.message(Msg::Install))
                .content("Install");

            let update_btn = Button::new()
                .grid_column(1)
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .height(42.0)
                .is_enabled(has_device && !self.is_installing)
                .on_click(context.message(Msg::Update))
                .content("Update");

            let uninstall_btn = Button::new()
                .grid_column(2)
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .height(42.0)
                .is_enabled(has_device && !self.is_installing)
                .on_click(context.message(Msg::Uninstall))
                .content("Uninstall");

            let (s0, s1, s2) = match self.hovered_action_btn {
                Some(0) => (1.03, 0.98, 0.98),
                Some(1) => (0.98, 1.03, 0.98),
                Some(2) => (0.98, 0.98, 1.03),
                _ => (1.0, 1.0, 1.0),
            };

            let install_border = Border::new()
                .grid_column(0)
                .scale(s0)
                .scale_transition(Some(Duration::from_millis(150)))
                .on_pointer_entered(context.callback(|_| Msg::HoverActionBtn(Some(0))))
                .on_pointer_exited(context.callback(|_| Msg::HoverActionBtn(None)))
                .content(install_btn);

            let update_border = Border::new()
                .grid_column(1)
                .scale(s1)
                .scale_transition(Some(Duration::from_millis(150)))
                .on_pointer_entered(context.callback(|_| Msg::HoverActionBtn(Some(1))))
                .on_pointer_exited(context.callback(|_| Msg::HoverActionBtn(None)))
                .content(update_btn);

            let uninstall_border = Border::new()
                .grid_column(2)
                .scale(s2)
                .scale_transition(Some(Duration::from_millis(150)))
                .on_pointer_entered(context.callback(|_| Msg::HoverActionBtn(Some(2))))
                .on_pointer_exited(context.callback(|_| Msg::HoverActionBtn(None)))
                .content(uninstall_btn);

            let action_bar = Grid::new()
                .columns([GridLength::Star(1.0), GridLength::Star(1.0), GridLength::Star(1.0)])
                .column_spacing(10.0)
                .children((install_border, update_border, uninstall_border));

            let bottom_panel = StackPanel::new()
                .grid_row(1)
                .spacing(12.0)
                .margin(Thickness::new(0.0, 16.0, 0.0, 0.0))
                .children((progress_section, action_bar));

            Grid::new()
                .rows([GridLength::Star(1.0), GridLength::Auto])
                .children((
                    ScrollViewer::new()
                        .grid_row(0)
                        .content(
                            StackPanel::new()
                                .spacing(16.0)
                                .children((
                                    admin_banner,
                                    device_card,
                                    version_card,
                                )),
                        ),
                    bottom_panel,
                ))
                .into()
        };

        // --- PAGE 2: Settings Page ---
        let page_settings: View = {
            let safety_alert: View = if !self.lock_non_removable {
                InfoBar::new()
                    .title("High Risk: Protection Disabled!")
                    .message("Non-removable internal drives can now be selected. Writing to an internal drive will permanently erase its partitions and data.")
                    .severity(InfoBarSeverity::Error)
                    .is_open(true)
                    .into()
            } else {
                View::empty()
            };

            let safety_card = Border::new()
                .background(ThemeBrush::CardBackground)
                .border_brush(ThemeBrush::CardStroke)
                .border_thickness(1.0)
                .corner_radius(8.0)
                .padding(16.0)
                .content(
                    StackPanel::new()
                        .spacing(14.0)
                        .children((
                            TextBlock::new()
                                .text("Disk Selection Safety")
                                .font_size(15.0)
                                .font_weight(FontWeight::SEMI_BOLD),
                            TextBlock::new()
                                .text("Prevent accidental selection and formatting of internal storage drives (NVMe SSDs, SATA drives, Windows C: partition). When locked, only removable USB drives can be selected.")
                                .font_size(12.0)
                                .text_wrapping(TextWrapping::Wrap)
                                .opacity(0.8),
                            Grid::new()
                                .columns([GridLength::Star(1.0), GridLength::Auto])
                                .children((
                                    StackPanel::new()
                                        .grid_column(0)
                                        .spacing(2.0)
                                        .vertical_alignment(VerticalAlignment::Center)
                                        .children((
                                            TextBlock::new()
                                                .text("Lock Non-Removable Disks")
                                                .font_weight(FontWeight::SEMI_BOLD),
                                            TextBlock::new()
                                                .text("Turning this off requires security confirmation")
                                                .font_size(11.0)
                                                .opacity(0.7),
                                        )),
                                    ToggleSwitch::new()
                                        .grid_column(1)
                                        .is_on(self.lock_non_removable)
                                        .on_toggled(context.callback(|on| Msg::ToggleLockNonRemovable(on)))
                                        .slots([
                                            SlotView::new(ToggleSwitchSlot::OnContent, "Locked"),
                                            SlotView::new(ToggleSwitchSlot::OffContent, "Unlocked"),
                                        ]),
                                )),
                            safety_alert,
                        )),
                );

            let about_card = Border::new()
                .background(ThemeBrush::CardBackground)
                .border_brush(ThemeBrush::CardStroke)
                .border_thickness(1.0)
                .corner_radius(8.0)
                .padding(16.0)
                .content(
                    StackPanel::new()
                        .spacing(10.0)
                        .children((
                            TextBlock::new()
                                .text("About rVentoy")
                                .font_size(15.0)
                                .font_weight(FontWeight::SEMI_BOLD),
                            TextBlock::new()
                                .text("rVentoy is a declarative WinUI 3 desktop application and core library written in 100% Rust.")
                                .font_size(12.0)
                                .opacity(0.8)
                                .text_wrapping(TextWrapping::Wrap),
                            Grid::new()
                                .columns([GridLength::Star(1.0), GridLength::Star(1.0)])
                                .children((
                                    TextBlock::new()
                                        .grid_column(0)
                                        .text("Application Version:")
                                        .font_size(12.0)
                                        .opacity(0.7),
                                    TextBlock::new()
                                        .grid_column(1)
                                        .text("v0.7.0")
                                        .font_size(12.0)
                                        .font_weight(FontWeight::SEMI_BOLD),
                                    )),
                            Grid::new()
                                .columns([GridLength::Star(1.0), GridLength::Star(1.0)])
                                .children((
                                    TextBlock::new()
                                        .grid_column(0)
                                        .text("Ventoy Package Core:")
                                        .font_size(12.0)
                                        .opacity(0.7),
                                    TextBlock::new()
                                        .grid_column(1)
                                        .text(&self.package_version)
                                        .font_size(12.0)
                                        .font_weight(FontWeight::SEMI_BOLD),
                                    )),
                        )),
                );

            let uninstall_card = Border::new()
                .background(ThemeBrush::CardBackground)
                .border_brush(ThemeBrush::CardStroke)
                .border_thickness(1.0)
                .corner_radius(8.0)
                .padding(16.0)
                .content(
                    StackPanel::new()
                        .spacing(12.0)
                        .children((
                            TextBlock::new()
                                .text("Uninstall & Restore Drive")
                                .font_size(15.0)
                                .font_weight(FontWeight::SEMI_BOLD),
                            TextBlock::new()
                                .text("Completely remove Ventoy bootloader and partitions from the selected drive, restoring the USB drive back to a clean, single standard storage volume.")
                                .font_size(12.0)
                                .text_wrapping(TextWrapping::Wrap)
                                .opacity(0.8),
                            Button::new()
                                .horizontal_alignment(HorizontalAlignment::Left)
                                .height(36.0)
                                .is_enabled(has_device && !self.is_installing)
                                .on_click(context.message(Msg::Uninstall))
                                .content("Uninstall Ventoy from Drive"),
                        )),
                );

            ScrollViewer::new()
                .content(
                    StackPanel::new()
                        .spacing(16.0)
                        .children((safety_card, uninstall_card, about_card)),
                )
                .into()
        };

        let active_page_content: View = match self.current_page {
            AppPage::Installer => page_installer,
            AppPage::Settings => page_settings,
        };

        let (page_opacity, page_scale) = match self.nav_transition {
            NavTransition::Idle => (1.0, 1.0),
            NavTransition::Exiting(_) => (0.0, 0.97),
            NavTransition::Entering => (0.0, 0.97),
        };

        let transition_duration = Duration::from_millis(130);

        let active_page_view = Border::new()
            .grid_row(1)
            .opacity(page_opacity)
            .opacity_transition(Some(transition_duration))
            .scale(page_scale)
            .scale_transition(Some(transition_duration))
            .content(active_page_content);

        let main_content_area = Grid::new()
            .rows([GridLength::Auto, GridLength::Star(1.0)])
            .margin(Thickness::new(20.0, 16.0, 20.0, 20.0))
            .children((
                content_header,
                active_page_view,
            ));

        // --- Modal Dialogs (Protection Unlock Warning & Install Erase Confirmation) ---
        let active_dialog: View = match self.active_dialog {
            ActiveDialog::UnlockProtection => ContentDialog::new()
                .title("Disable Non-Removable Disk Protection?")
                .primary_button_text("Disable Protection")
                .close_button_text("Cancel (Keep Safe)")
                .is_open(true)
                .on_closed(context.callback(|res| {
                    if res == ContentDialogResult::Primary {
                        Msg::ConfirmUnlock
                    } else {
                        Msg::CancelUnlock
                    }
                }))
                .content(
                    StackPanel::new()
                        .spacing(12.0)
                        .children((
                            TextBlock::new()
                                .text("DANGER: High risk of permanent data loss!")
                                .font_weight(FontWeight::BOLD)
                                .foreground(ThemeBrush::SystemCritical)
                                .text_wrapping(TextWrapping::Wrap),
                            TextBlock::new()
                                .text("Non-removable disks are your computer's internal storage drives (NVMe SSDs, SATA drives, and your Windows system drive C:).")
                                .text_wrapping(TextWrapping::Wrap),
                            TextBlock::new()
                                .text("If you select an internal drive and install Ventoy, ALL existing partitions, operating systems, and files on that drive will be PERMANENTLY ERASED.")
                                .font_weight(FontWeight::SEMI_BOLD)
                                .text_wrapping(TextWrapping::Wrap),
                            TextBlock::new()
                                .text("Are you absolutely sure you want to turn off this protection?")
                                .text_wrapping(TextWrapping::Wrap)
                                .opacity(0.85),
                        )),
                )
                .into(),

            ActiveDialog::ConfirmInstall => {
                let target_info: View = if let Some(idx) = self.selected_device {
                    if let Some(dev) = self.devices.get(idx) {
                        let size_gb = (dev.size_in_bytes as f64) / (1024.0 * 1024.0 * 1024.0);
                        let drive_type = if dev.is_removable {
                            "Removable USB Drive"
                        } else {
                            "Internal Fixed Disk (High Risk!)"
                        };
                        Border::new()
                            .background(ThemeBrush::CardBackground)
                            .border_brush(ThemeBrush::CardStroke)
                            .border_thickness(1.0)
                            .corner_radius(6.0)
                            .padding(12.0)
                            .content(
                                StackPanel::new()
                                    .spacing(4.0)
                                    .children((
                                        TextBlock::new()
                                            .text(&dev.model_str)
                                            .font_weight(FontWeight::SEMI_BOLD)
                                            .font_size(13.0),
                                        TextBlock::new()
                                            .text(format!(
                                                "PhysicalDrive{} • {:.1} GB • {}",
                                                dev.phy_drive, size_gb, drive_type
                                            ))
                                            .font_size(11.5)
                                            .opacity(0.75),
                                    )),
                            )
                            .into()
                    } else {
                        View::empty()
                    }
                } else {
                    View::empty()
                };

                ContentDialog::new()
                    .title("Format & Install rVentoy?")
                    .primary_button_text("Format and Install")
                    .close_button_text("Cancel")
                    .is_open(true)
                    .on_closed(context.callback(|res| {
                        if res == ContentDialogResult::Primary {
                            Msg::ConfirmInstall
                        } else {
                            Msg::CancelInstall
                        }
                    }))
                    .content(
                        StackPanel::new()
                            .spacing(12.0)
                            .children((
                                TextBlock::new()
                                    .text("WARNING: The device will be formatted and ALL existing data will be PERMANENTLY ERASED!")
                                    .font_weight(FontWeight::BOLD)
                                    .foreground(ThemeBrush::SystemCritical)
                                    .text_wrapping(TextWrapping::Wrap),
                                target_info,
                                TextBlock::new()
                                    .text("rVentoy will partition this drive into a 32 MB VTOYEFI boot partition and an exFAT data partition for your bootable ISO files.")
                                    .text_wrapping(TextWrapping::Wrap)
                                    .font_size(12.0)
                                    .opacity(0.85),
                                TextBlock::new()
                                    .text("Are you sure you want to continue?")
                                    .font_weight(FontWeight::SEMI_BOLD)
                                    .text_wrapping(TextWrapping::Wrap),
                            )),
                    )
                    .into()
            }

            ActiveDialog::ConfirmUninstall => {
                let target_info: View = if let Some(idx) = self.selected_device {
                    if let Some(dev) = self.devices.get(idx) {
                        let size_gb = (dev.size_in_bytes as f64) / (1024.0 * 1024.0 * 1024.0);
                        let drive_type = if dev.is_removable {
                            "Removable USB Drive"
                        } else {
                            "Internal Fixed Disk (High Risk!)"
                        };
                        Border::new()
                            .background(ThemeBrush::CardBackground)
                            .border_brush(ThemeBrush::CardStroke)
                            .border_thickness(1.0)
                            .corner_radius(6.0)
                            .padding(12.0)
                            .content(
                                StackPanel::new()
                                    .spacing(4.0)
                                    .children((
                                        TextBlock::new()
                                            .text(&dev.model_str)
                                            .font_weight(FontWeight::SEMI_BOLD)
                                            .font_size(13.0),
                                        TextBlock::new()
                                            .text(format!(
                                                "PhysicalDrive{} • {:.1} GB • {}",
                                                dev.phy_drive, size_gb, drive_type
                                            ))
                                            .font_size(11.5)
                                            .opacity(0.75),
                                    )),
                            )
                            .into()
                    } else {
                        View::empty()
                    }
                } else {
                    View::empty()
                };

                ContentDialog::new()
                    .title("Uninstall rVentoy?")
                    .primary_button_text("Uninstall and Format")
                    .close_button_text("Cancel")
                    .is_open(true)
                    .on_closed(context.callback(|res| {
                        if res == ContentDialogResult::Primary {
                            Msg::ConfirmUninstall
                        } else {
                            Msg::CancelUninstall
                        }
                    }))
                    .content(
                        StackPanel::new()
                            .spacing(12.0)
                            .children((
                                TextBlock::new()
                                    .text("WARNING: This will wipe Ventoy bootloader and partitions from the selected drive!")
                                    .font_weight(FontWeight::BOLD)
                                    .foreground(ThemeBrush::SystemCritical)
                                    .text_wrapping(TextWrapping::Wrap),
                                target_info,
                                TextBlock::new()
                                    .text("The drive will be cleaned, repartitioned into a single standard primary partition, and formatted as exFAT for regular file storage.")
                                    .text_wrapping(TextWrapping::Wrap)
                                    .font_size(12.0)
                                    .opacity(0.85),
                                TextBlock::new()
                                    .text("Are you sure you want to continue?")
                                    .font_weight(FontWeight::SEMI_BOLD)
                                    .text_wrapping(TextWrapping::Wrap),
                            )),
                    )
                    .into()
            }

            ActiveDialog::None => View::empty(),
        };

        Grid::new()
            .children((main_content_area, active_dialog))
            .into()
    }
}

fn main() {
    VentoyCore::preload_assets_in_background();
    App::run_component::<RVentoyMainWindow>(()).unwrap();
}