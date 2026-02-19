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
            let _ = writeln!(
                file, "[{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), msg
            );
        }
    }
}

pub fn log(msg: &str, debug: bool) {
    if debug { println!("{}", msg); }
}

