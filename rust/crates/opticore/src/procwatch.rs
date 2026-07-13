//! Lightweight process watching for playtime tracking: is any running
//! process's image located under a given directory? Direct kernel32 externs
//! (Toolhelp32 snapshot + QueryFullProcessImageNameW) — no extra crates, and
//! one sweep costs well under a millisecond, so a 10–15 s polling cadence is
//! effectively free.

#[cfg(windows)]
mod win {
    pub const TH32CS_SNAPPROCESS: u32 = 0x0000_0002;
    pub const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    pub const INVALID_HANDLE_VALUE: isize = -1;
    pub const MAX_PATH_W: usize = 32768;

    #[repr(C)]
    pub struct ProcessEntry32W {
        pub dw_size: u32,
        pub cnt_usage: u32,
        pub th32_process_id: u32,
        pub th32_default_heap_id: usize,
        pub th32_module_id: u32,
        pub cnt_threads: u32,
        pub th32_parent_process_id: u32,
        pub pc_pri_class_base: i32,
        pub dw_flags: u32,
        pub sz_exe_file: [u16; 260],
    }

    #[link(name = "kernel32")]
    extern "system" {
        pub fn CreateToolhelp32Snapshot(dwFlags: u32, th32ProcessID: u32) -> isize;
        pub fn Process32FirstW(hSnapshot: isize, lppe: *mut ProcessEntry32W) -> i32;
        pub fn Process32NextW(hSnapshot: isize, lppe: *mut ProcessEntry32W) -> i32;
        pub fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> isize;
        pub fn QueryFullProcessImageNameW(
            hProcess: isize,
            dwFlags: u32,
            lpExeName: *mut u16,
            lpdwSize: *mut u32,
        ) -> i32;
        pub fn CloseHandle(hObject: isize) -> i32;
    }

    /// Full image path of a process, or None (access denied on system procs).
    pub fn image_path(pid: u32) -> Option<String> {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle == 0 {
                return None;
            }
            let mut buf = vec![0u16; MAX_PATH_W];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut len);
            CloseHandle(handle);
            (ok != 0).then(|| String::from_utf16_lossy(&buf[..len as usize]))
        }
    }

    /// All process ids currently running.
    pub fn all_pids() -> Vec<u32> {
        let mut pids = Vec::with_capacity(256);
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot == INVALID_HANDLE_VALUE {
                return pids;
            }
            let mut entry: ProcessEntry32W = std::mem::zeroed();
            entry.dw_size = std::mem::size_of::<ProcessEntry32W>() as u32;
            if Process32FirstW(snapshot, &mut entry) != 0 {
                loop {
                    pids.push(entry.th32_process_id);
                    if Process32NextW(snapshot, &mut entry) == 0 {
                        break;
                    }
                }
            }
            CloseHandle(snapshot);
        }
        pids
    }
}

/// True when at least one running process's exe lives under `dir`
/// (case-insensitive, path-separator-normalized — matches the scanner's
/// normalization so Steam/GOG/Xbox layouts all compare cleanly).
pub fn any_process_under(dir: &std::path::Path) -> bool {
    #[cfg(windows)]
    {
        let needle = normalize(&dir.to_string_lossy());
        let needle_slash = format!("{needle}/");
        win::all_pids().into_iter().any(|pid| {
            win::image_path(pid)
                .map(|p| normalize(&p).starts_with(&needle_slash))
                .unwrap_or(false)
        })
    }
    #[cfg(not(windows))]
    {
        let _ = dir;
        false
    }
}

fn normalize(path: &str) -> String {
    path.replace('\\', "/").to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn finds_own_process_and_not_in_empty_dir() {
        let own_exe = std::env::current_exe().unwrap();
        let own_dir = own_exe.parent().unwrap();
        assert!(any_process_under(own_dir));

        let tmp = tempfile::tempdir().unwrap();
        assert!(!any_process_under(tmp.path()));
    }

    #[test]
    fn normalization_is_separator_and_case_insensitive() {
        assert_eq!(normalize(r"C:\Games\Foo"), "c:/games/foo");
    }
}
