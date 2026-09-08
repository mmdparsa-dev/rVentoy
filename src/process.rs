#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, unused)]

use std::ffi::c_void;
use std::mem::size_of;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::ProcessStatus::*;
use windows_sys::Win32::System::Threading::*;

#[repr(C)]
struct IO_STATUS_BLOCK {
    status: NTSTATUS,
    information: usize,
}

const FILE_PROCESS_IDS_USING_FILE_INFORMATION: u32 = 47;

type NtQueryInformationFileFn = unsafe extern "system" fn(
    file_handle: HANDLE,
    io_status_block: *mut IO_STATUS_BLOCK,
    file_information: *mut c_void,
    length: u32,
    file_information_class: u32,
) -> NTSTATUS;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleA(lpModuleName: *const u8) -> HMODULE;
    fn GetProcAddress(hModule: HMODULE, lpProcName: *const u8) -> FARPROC;
}

pub fn find_processes_occupying_handle(handle: HANDLE) -> Vec<u32> {
    let mut pids = Vec::new();
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return pids;
    }

    unsafe {
        let ntdll = GetModuleHandleA(b"ntdll.dll\0".as_ptr());
        if ntdll.is_null() {
            return pids;
        }

        let proc_addr = GetProcAddress(ntdll, b"NtQueryInformationFile\0".as_ptr());
        if proc_addr.is_none() {
            return pids;
        }

        let nt_query_information_file: NtQueryInformationFileFn = std::mem::transmute(proc_addr);

        let mut buffer_size: u32 = 16 * 1024;
        let mut buffer: Vec<u8> = vec![0u8; buffer_size as usize];
        let mut isb = IO_STATUS_BLOCK {
            status: 0,
            information: 0,
        };

        loop {
            let status = nt_query_information_file(
                handle,
                &mut isb,
                buffer.as_mut_ptr() as *mut c_void,
                buffer_size,
                FILE_PROCESS_IDS_USING_FILE_INFORMATION,
            );

            // STATUS_INFO_LENGTH_MISMATCH is 0xC0000004
            if status == -1073741820 {
                buffer_size *= 2;
                if buffer_size > 32 * 1024 * 1024 {
                    return pids;
                }
                buffer.resize(buffer_size as usize, 0);
            } else if status == 0 {
                // Success: read number of process IDs
                if buffer.len() >= 8 {
                    let num_pids = *(buffer.as_ptr() as *const usize);
                    let pids_ptr = buffer.as_ptr().add(size_of::<usize>()) as *const usize;
                    for i in 0..num_pids {
                        pids.push(*pids_ptr.add(i) as u32);
                    }
                }
                break;
            } else {
                break;
            }
        }
    }

    pids
}

pub fn get_process_name(pid: u32) -> String {
    unsafe {
        let h_proc = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid);
        if !h_proc.is_null() && h_proc != INVALID_HANDLE_VALUE {
            let mut name_buf = [0u8; 260];
            let len = GetProcessImageFileNameA(h_proc, name_buf.as_mut_ptr(), 260);
            CloseHandle(h_proc);
            if len > 0 {
                return String::from_utf8_lossy(&name_buf[..len as usize]).to_string();
            }
        }
    }
    format!("PID:{}", pid)
}

pub fn find_process_occupy_disk(h_drive: HANDLE, p_phy_drive: &crate::types::PHY_DRIVE_INFO) -> i32 {
    let pids = find_processes_occupying_handle(h_drive);
    if !pids.is_empty() {
        crate::ventoy_log!("Found {} process(es) occupying disk PhysicalDrive{}:", pids.len(), p_phy_drive.phy_drive);
        for pid in &pids {
            let name = get_process_name(*pid);
            crate::ventoy_log!("  -> PID: {}, Image: {}", pid, name);
        }
        return pids.len() as i32;
    }
    0
}
