use anyhow::Result;
use std::env;
use std::fs;
use std::path::PathBuf;

const APP_NAME: &str = "RazerTaskbar";

/// Get the Windows Startup folder path for current user
fn get_startup_folder() -> Result<PathBuf> {
    // Get the user's Startup folder: %APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup
    let appdata = env::var("APPDATA")
        .map_err(|_| anyhow::anyhow!("Failed to get APPDATA environment variable"))?;

    let mut startup_path = PathBuf::from(appdata);
    startup_path.push("Microsoft");
    startup_path.push("Windows");
    startup_path.push("Start Menu");
    startup_path.push("Programs");
    startup_path.push("Startup");

    Ok(startup_path)
}

/// Get the shortcut file path in the Startup folder
fn get_shortcut_path() -> Result<PathBuf> {
    let mut shortcut_path = get_startup_folder()?;
    shortcut_path.push(format!("{}.lnk", APP_NAME));
    Ok(shortcut_path)
}

/// Enable or disable app autostart using Windows Startup folder
pub fn set_startup(enable: bool) -> Result<()> {
    let shortcut_path = get_shortcut_path()?;

    if enable {
        // Create shortcut in Startup folder
        create_shortcut(&shortcut_path)?;
    } else {
        // Remove shortcut from Startup folder
        if shortcut_path.exists() {
            fs::remove_file(&shortcut_path)?;
        }
    }

    Ok(())
}

/// Create a Windows shortcut (.lnk file) using COM
fn create_shortcut(shortcut_path: &PathBuf) -> Result<()> {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile,
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::IShellLinkW;
    use windows::core::{GUID, Interface};

    unsafe {
        // Initialize COM
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        // CLSID for ShellLink
        let clsid = GUID::from_values(
            0x00021401, 0x0000, 0x0000,
            [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46]
        );

        // Create ShellLink instance
        let shell_link: IShellLinkW = CoCreateInstance(&clsid, None, CLSCTX_INPROC_SERVER)?;

        // Get current executable path
        let exe_path = env::current_exe()?;
        let exe_path_str: Vec<u16> = exe_path.to_string_lossy().encode_utf16().chain(Some(0)).collect();

        // Set the target path
        shell_link.SetPath(windows::core::PCWSTR(exe_path_str.as_ptr()))?;

        // Set working directory to exe directory
        if let Some(parent) = exe_path.parent() {
            let work_dir: Vec<u16> = parent.to_string_lossy().encode_utf16().chain(Some(0)).collect();
            shell_link.SetWorkingDirectory(windows::core::PCWSTR(work_dir.as_ptr()))?;
        }

        // Set description
        let description: Vec<u16> = "Razer Taskbar - Battery Monitor".encode_utf16().chain(Some(0)).collect();
        shell_link.SetDescription(windows::core::PCWSTR(description.as_ptr()))?;

        // Save the shortcut
        let persist_file: IPersistFile = shell_link.cast()?;
        let shortcut_path_str: Vec<u16> = shortcut_path.to_string_lossy().encode_utf16().chain(Some(0)).collect();
        persist_file.Save(windows::core::PCWSTR(shortcut_path_str.as_ptr()), true)?;

        // Cleanup COM
        CoUninitialize();

        Ok(())
    }
}
