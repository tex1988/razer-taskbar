use std::fs::OpenOptions;
use std::io::Write;

pub fn write_error_log(msg: &str) {
    if let Some(mut log_path) = std::env::temp_dir().parent().map(|p| p.to_path_buf()) {
        log_path.push("razer_taskbar_errors.log");
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let _ = writeln!(file, "[{}] {}", timestamp, msg);
        }
    }
}

pub fn log(msg: &str, debug: bool) {
    if debug { println!("{}", msg); }
}

#[cfg(target_os = "windows")]
pub fn log_memory_usage(label: &str, debug: bool) {
    use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows::Win32::System::Threading::GetCurrentProcess;

    unsafe {
        let mut pmc = PROCESS_MEMORY_COUNTERS::default();
        pmc.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;

        if GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut pmc,
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32
        ).is_ok() {
            // PagefileUsage is the Private Bytes (actual RAM used by process)
            let private_kb = pmc.PagefileUsage / 1024;
            let working_set_kb = pmc.WorkingSetSize / 1024;
            let msg = format!("{}: Private={} KB, WorkingSet={} KB", label, private_kb, working_set_kb);
            log(&msg, debug);
            write_error_log(&msg);
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn log_memory_usage(_label: &str, _debug: bool) {
    // No-op on non-Windows platforms
}

