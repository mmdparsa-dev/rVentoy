use std::sync::RwLock;

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum STR_ID {
    STR_ERROR = 0,
    STR_WARNING,
    STR_INFO,
    STR_INCORRECT_DIR,
    STR_INCORRECT_TREE_DIR,
    STR_DEVICE,
    STR_LOCAL_VER,
    STR_DISK_VER,
    STR_STATUS,
    STR_INSTALL,
    STR_UPDATE,
    STR_UPDATE_TIP,
    STR_INSTALL_TIP,
    STR_INSTALL_TIP2,
    STR_INSTALL_SUCCESS,
    STR_INSTALL_FAILED,
    STR_UPDATE_SUCCESS,
    STR_UPDATE_FAILED,
    STR_WAIT_PROCESS,
    STR_MENU_OPTION,
    STR_MENU_SECURE_BOOT,
    STR_MENU_PART_CFG,
    STR_BTN_OK,
    STR_BTN_CANCEL,
    STR_PRESERVE_SPACE,
    STR_SPACE_VAL_INVALID,
    STR_MENU_CLEAR,
    STR_CLEAR_SUCCESS,
    STR_CLEAR_FAILED,
    STR_MENU_PART_STYLE,
    STR_DISK_2TB_MBR_ERROR,
    STR_SHOW_ALL_DEV,
    STR_PART_ALIGN_4KB,
    STR_WEB_COMMUNICATION_ERR,
    STR_WEB_REMOTE_ABNORMAL,
    STR_WEB_REQUEST_TIMEOUT,
    STR_WEB_SERVICE_UNAVAILABLE,
    STR_WEB_TOKEN_MISMATCH,
    STR_WEB_SERVICE_BUSY,
    STR_MENU_VTSI_CREATE,
    STR_VTSI_CREATE_TIP,
    STR_VTSI_CREATE_SUCCESS,
    STR_VTSI_CREATE_FAILED,
    STR_MENU_PART_RESIZE,
    STR_PART_RESIZE_TIP,
    STR_PART_RESIZE_SUCCESS,
    STR_PART_RESIZE_FAILED,
    STR_PART_RESIZE_UNSUPPORTED,
    STR_INSTALL_YES_TIP1,
    STR_INSTALL_YES_TIP2,
    STR_PART_VENTOY_FS,
    STR_PART_FS,
    STR_PART_CLUSTER,
    STR_PART_CLUSTER_DEFAULT,
    STR_DONATE,
    STR_4KN_UNSUPPORTED,
    STR_ID_MAX,
}

pub static CURRENT_LANGUAGE: RwLock<Option<VentoyLanguage>> = RwLock::new(None);

pub struct VentoyLanguage {
    pub name: String,
    pub font_family: String,
    pub font_size: i32,
    pub messages: Vec<String>,
}

impl Default for VentoyLanguage {
    fn default() -> Self {
        let mut messages = vec![String::new(); STR_ID::STR_ID_MAX as usize];
        messages[STR_ID::STR_ERROR as usize] = "Error".to_string();
        messages[STR_ID::STR_WARNING as usize] = "Warning".to_string();
        messages[STR_ID::STR_INFO as usize] = "Info".to_string();
        messages[STR_ID::STR_DEVICE as usize] = "Device".to_string();
        messages[STR_ID::STR_LOCAL_VER as usize] = "Ventoy In Package".to_string();
        messages[STR_ID::STR_DISK_VER as usize] = "Ventoy In Device".to_string();
        messages[STR_ID::STR_STATUS as usize] = "Status".to_string();
        messages[STR_ID::STR_INSTALL as usize] = "Install".to_string();
        messages[STR_ID::STR_UPDATE as usize] = "Update".to_string();
        messages[STR_ID::STR_INSTALL_SUCCESS as usize] = "Congratulations! Ventoy has been successfully installed to the device.".to_string();
        messages[STR_ID::STR_INSTALL_FAILED as usize] = "An error occurred during the installation. You can replug the USB and try again. Check log.txt for detail.".to_string();
        messages[STR_ID::STR_UPDATE_SUCCESS as usize] = "Congratulations! Ventoy has been successfully updated on the device.".to_string();
        messages[STR_ID::STR_UPDATE_FAILED as usize] = "An error occurred during the update. Check log.txt for detail.".to_string();

        Self {
            name: "English".to_string(),
            font_family: "Segoe UI".to_string(),
            font_size: 9,
            messages,
        }
    }
}

pub fn get_str(id: STR_ID) -> String {
    if let Ok(guard) = CURRENT_LANGUAGE.read() {
        if let Some(ref lang) = *guard {
            let idx = id as usize;
            if idx < lang.messages.len() && !lang.messages[idx].is_empty() {
                return lang.messages[idx].clone();
            }
        }
    }
    let default_lang = VentoyLanguage::default();
    let idx = id as usize;
    if idx < default_lang.messages.len() {
        default_lang.messages[idx].clone()
    } else {
        String::new()
    }
}
