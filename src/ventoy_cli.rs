use std::sync::Mutex;

use crate::utility::*;
use crate::ventoy_log;

pub static CLI_RESERVE_SPACE_MB: Mutex<i32> = Mutex::new(0);

pub fn cli_set_reserve_space(mb: i32) {
    let mut space = CLI_RESERVE_SPACE_MB.lock().unwrap();
    *space = mb;
}

pub fn run_cli(args: &[String]) -> i32 {
    let mut cli_mode = CLI_MODE.lock().unwrap();
    *cli_mode = true;

    ventoy_log!("Running in CLI mode with args: {:?}", args);
    // CLI commands handling (e.g. -i for install, -u for update)
    0
}
