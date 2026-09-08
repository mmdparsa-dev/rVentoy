#![windows_subsystem = "windows"]

use windows_reactor::*;
use VentoyCore::phy_drive::*;
use VentoyCore::types::PHY_DRIVE_INFO;
use VentoyCore::utility::*;

#[derive(Clone)]
pub struct DriveItem {
    pub id: i32,
    pub phy_drive: i32,
    pub display_text: String,
    pub ventoy_version: String,
    pub size_in_bytes: u64,
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
    }
}

#[derive(Clone, Debug)]
pub enum Msg {
    Refresh,
    SelectDevice(Option<usize>),
    SelectPartitionStyle(Option<usize>),
    ToggleSecureBoot(bool),
    Install,
    Update,
    InstallFinished(bool, String),
    UpdateFinished(bool, String),
    DismissInfo,
    RestartAdmin,
}

pub struct RVentoyMainWindow {
    devices: Vec<DriveItem>,
    selected_device: Option<usize>,
    selected_partition_style: usize,
    secure_boot_enabled: bool,
    package_version: String,
    device_version: String,
    is_installing: bool,
    is_elevated: bool,
    progress: f64,
    status_text: String,
    info_message: Option<String>,
    info_severity: InfoBarSeverity,
}

pub fn is_elevated() -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Security::*;
    use windows_sys::Win32::System::Threading::*;
    let mut token: HANDLE = std::ptr::null_mut();
    let open_ok = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) };
    if open_ok == 0 {
        return false;
    }
    let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
    let mut ret_len: u32 = 0;
    let res = unsafe {
        GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        )
    };
    unsafe { CloseHandle(token) };
    res != 0 && elevation.TokenIsElevated != 0
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

fn scan_usb_drives() -> Vec<DriveItem> {
    let drives = scan_all_physical_drives();
    let mut list = Vec::new();
    for drive in drives {
        if drive.id >= 0 {
            let vendor = drive.vendor_str();
            let product = drive.product_str();
            let model = format!("{} {}", vendor, product).trim().to_string();
            let model_str = if model.is_empty() {
                "USB Drive".to_string()
            } else {
                model
            };
            let size_gb = (drive.size_in_bytes as f64) / (1024.0 * 1024.0 * 1024.0);
            let ver = drive.version_str();
            let display = format!(
                "PhysicalDrive{} - {} ({:.1} GB)",
                drive.phy_drive, model_str, size_gb
            );
            list.push(DriveItem {
                id: drive.id,
                phy_drive: drive.phy_drive,
                display_text: display,
                ventoy_version: ver,
                size_in_bytes: drive.size_in_bytes,
                raw_drive: drive,
            });
        }
    }
    list
}

impl Component for RVentoyMainWindow {
    type Input = ();
    type Message = Msg;

    fn create(_input: &(), _context: &ComponentContext<Self>) -> Self {
        let devices = scan_usb_drives();
        let package_version = get_local_ventoy_version();
        let (selected_device, device_version) = if !devices.is_empty() {
            let ver = if devices[0].ventoy_version.is_empty() {
                "--".to_string()
            } else {
                devices[0].ventoy_version.clone()
            };
            (Some(0), ver)
        } else {
            (None, "--".to_string())
        };

        let is_elevated = is_elevated();

        Self {
            devices,
            selected_device,
            selected_partition_style: 0,
            secure_boot_enabled: true,
            package_version,
            device_version,
            is_installing: false,
            is_elevated,
            progress: 0.0,
            status_text: String::new(),
            info_message: None,
            info_severity: InfoBarSeverity::Informational,
        }
    }

    fn update(&mut self, message: Self::Message, context: &ComponentContext<Self>) {
        match message {
            Msg::Refresh => {
                self.devices = scan_usb_drives();
                if let Some(idx) = self.selected_device {
                    if idx < self.devices.len() {
                        let ver = &self.devices[idx].ventoy_version;
                        self.device_version = if ver.is_empty() {
                            "--".to_string()
                        } else {
                            ver.clone()
                        };
                    } else if !self.devices.is_empty() {
                        self.selected_device = Some(0);
                        let ver = &self.devices[0].ventoy_version;
                        self.device_version = if ver.is_empty() {
                            "--".to_string()
                        } else {
                            ver.clone()
                        };
                    } else {
                        self.selected_device = None;
                        self.device_version = "--".to_string();
                    }
                } else if !self.devices.is_empty() {
                    self.selected_device = Some(0);
                    let ver = &self.devices[0].ventoy_version;
                    self.device_version = if ver.is_empty() {
                        "--".to_string()
                    } else {
                        ver.clone()
                    };
                } else {
                    self.selected_device = None;
                    self.device_version = "--".to_string();
                }
            }
            Msg::SelectDevice(opt_idx) => {
                self.selected_device = opt_idx;
                if let Some(idx) = opt_idx {
                    if let Some(dev) = self.devices.get(idx) {
                        self.device_version = if dev.ventoy_version.is_empty() {
                            "--".to_string()
                        } else {
                            dev.ventoy_version.clone()
                        };
                    }
                } else {
                    self.device_version = "--".to_string();
                }
            }
            Msg::SelectPartitionStyle(opt_idx) => {
                if let Some(idx) = opt_idx {
                    self.selected_partition_style = idx;
                }
            }
            Msg::ToggleSecureBoot(enabled) => {
                self.secure_boot_enabled = enabled;
            }
            Msg::Install => {
                if let Some(idx) = self.selected_device {
                    if let Some(dev) = self.devices.get(idx) {
                        let drive = dev.raw_drive;
                        let part_style = self.selected_partition_style as i32;
                        let secure_boot = self.secure_boot_enabled;
                        self.is_installing = true;
                        self.progress = 0.0;
                        self.status_text = format!(
                            "Installing rVentoy ({}) to PhysicalDrive{}...",
                            if part_style == 1 { "GPT" } else { "MBR" },
                            dev.phy_drive
                        );
                        self.info_message = None;

                        context.spawn_background(move |_cancel| {
                            let res = install_ventoy_to_phy_drive(&drive, part_style, secure_boot, None);
                            if res == 0 {
                                Msg::InstallFinished(
                                    true,
                                    "rVentoy installed successfully!".to_string(),
                                )
                            } else {
                                Msg::InstallFinished(
                                    false,
                                    format!("rVentoy installation failed with error code {}", res),
                                )
                            }
                        });
                    }
                }
            }
            Msg::Update => {
                if let Some(idx) = self.selected_device {
                    if let Some(dev) = self.devices.get(idx) {
                        let drive = dev.raw_drive;
                        self.is_installing = true;
                        self.progress = 0.0;
                        self.status_text = format!(
                            "Updating rVentoy on PhysicalDrive{}...",
                            dev.phy_drive
                        );
                        self.info_message = None;

                        context.spawn_background(move |_cancel| {
                            let res = update_ventoy_to_phy_drive(&drive, None);
                            if res == 0 {
                                Msg::UpdateFinished(
                                    true,
                                    "rVentoy updated successfully!".to_string(),
                                )
                            } else {
                                Msg::UpdateFinished(
                                    false,
                                    format!("rVentoy update failed with error code {}", res),
                                )
                            }
                        });
                    }
                }
            }
            Msg::InstallFinished(success, msg) => {
                self.is_installing = false;
                self.progress = if success { 100.0 } else { 0.0 };
                self.status_text = msg.clone();
                self.info_message = Some(msg);
                self.info_severity = if success {
                    InfoBarSeverity::Success
                } else {
                    InfoBarSeverity::Error
                };
                self.devices = scan_usb_drives();
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
            Msg::UpdateFinished(success, msg) => {
                self.is_installing = false;
                self.progress = if success { 100.0 } else { 0.0 };
                self.status_text = msg.clone();
                self.info_message = Some(msg);
                self.info_severity = if success {
                    InfoBarSeverity::Success
                } else {
                    InfoBarSeverity::Error
                };
                self.devices = scan_usb_drives();
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
                .client_size(520.0, 640.0),
        );

        let header = StackPanel::new()
            .grid_row(0)
            .spacing(4.0)
            .children((
                TextBlock::new()
                    .text("rVentoy")
                    .font_size(24.0)
                    .font_weight(FontWeight::SEMI_BOLD),
                TextBlock::new()
                    .text("A modern multiboot tool in Rust & WinUI 3")
                    .font_size(12.0)
                    .opacity(0.7),
            ));

        let device_labels = self
            .devices
            .iter()
            .map(|d| d.display_text.clone())
            .collect::<Vec<_>>();

        let placeholder = if self.devices.is_empty() {
            "No USB drives found (click refresh)"
        } else {
            "Select USB drive..."
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

        let part_style_labels = vec!["MBR (Default)".to_string(), "GPT".to_string()];
        let part_combo = ComboBox::new()
            .grid_column(0)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .is_enabled(!self.is_installing)
            .items_source(part_style_labels)
            .selected_index(Some(self.selected_partition_style))
            .on_selection_changed(context.callback(|idx| Msg::SelectPartitionStyle(idx)));

        let sb_checkbox = CheckBox::new()
            .grid_column(1)
            .is_checked(self.secure_boot_enabled)
            .is_enabled(!self.is_installing)
            .vertical_alignment(VerticalAlignment::Center)
            .on_is_checked_changed(context.callback(|checked| Msg::ToggleSecureBoot(checked)))
            .content("Secure Boot Support");

        let options_card = Border::new()
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
                            .text("Installation Options")
                            .font_size(14.0)
                            .font_weight(FontWeight::SEMI_BOLD),
                        Grid::new()
                            .columns([GridLength::Star(1.0), GridLength::Star(1.0)])
                            .column_spacing(16.0)
                            .children((part_combo, sb_checkbox)),
                    )),
            );

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
                                .text(if self.progress > 0.0 {
                                    format!("{:.0}%", self.progress)
                                } else {
                                    String::new()
                                })
                                .font_size(12.0),
                        )),
                    ProgressBar::new()
                        .minimum(0.0)
                        .maximum(100.0)
                        .value(self.progress)
                        .is_indeterminate(self.progress == 0.0),
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

        let body = StackPanel::new()
            .grid_row(1)
            .spacing(16.0)
            .children((admin_banner, device_card, version_card, options_card, progress_section));

        let has_device = self.selected_device.is_some() && !self.devices.is_empty();

        let install_btn = Button::new()
            .grid_column(0)
            .style(ButtonStyle::Accent)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .height(40.0)
            .is_enabled(has_device && !self.is_installing)
            .on_click(context.message(Msg::Install))
            .content("Install");

        let update_btn = Button::new()
            .grid_column(1)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .height(40.0)
            .is_enabled(has_device && !self.is_installing)
            .on_click(context.message(Msg::Update))
            .content("Update");

        let action_bar = Grid::new()
            .grid_row(2)
            .columns([GridLength::Star(1.0), GridLength::Star(1.0)])
            .column_spacing(12.0)
            .children((install_btn, update_btn));

        Grid::new()
            .rows([GridLength::Auto, GridLength::Star(1.0), GridLength::Auto])
            .columns([GridLength::Star(1.0)])
            .row_spacing(20.0)
            .margin(24.0)
            .children((header, body, action_bar))
            .into()
    }
}

fn main() {
    App::run_component::<RVentoyMainWindow>(()).unwrap();
}