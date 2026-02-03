fn main() {
    // Only compile the icon resource on Windows
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("src/assets/app_icon.ico");
        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to set application icon: {}", e);
        }
    }
}
