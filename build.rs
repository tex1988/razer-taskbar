use std::fs;
use std::path::Path;

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

    // Generate embedded assets code from icon.properties
    generate_embedded_assets();
}

fn generate_embedded_assets() {
    let icon_properties_path = "src/assets/icon.properties";
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("embedded_assets.rs");

    // Parse icon.properties to find all referenced files
    let content = fs::read_to_string(icon_properties_path)
        .expect("Failed to read icon.properties");

    let mut filenames = std::collections::HashSet::new();

    // Parse icon.properties
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((_, filename)) = line.split_once('=') {
            filenames.insert(filename.trim().to_string());
        }
    }

    // Always include these special files
    filenames.insert("charging.png".to_string());
    filenames.insert("no_device.png".to_string());
    filenames.insert("headphones.png".to_string());
    filenames.insert("keyboard.png".to_string());
    filenames.insert("mouse.png".to_string());
    filenames.insert("unknown.png".to_string());

    // Generate Rust code
    let mut code = String::from("// Auto-generated from icon.properties\n");
    code.push_str("// Note: HashMap is imported in icon_manager.rs\n\n");
    code.push_str("pub fn get_embedded_assets() -> HashMap<&'static str, &'static [u8]> {\n");
    code.push_str("    let mut map = HashMap::new();\n");

    // Embed assets from dark/ and light/ subfolders, keyed as "dark/filename" and "light/filename"
    for theme in &["dark", "light"] {
        for filename in &filenames {
            let asset_path = format!("src/assets/{}/{}", theme, filename);
            if Path::new(&asset_path).exists() {
                code.push_str(&format!(
                    "    map.insert(\"{}/{}\", include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/src/assets/{}/{}\")) as &[u8]);\n",
                    theme, filename, theme, filename
                ));
            }
        }
    }

    code.push_str("    map\n");
    code.push_str("}\n");

    fs::write(&dest_path, code).expect("Failed to write embedded_assets.rs");

    // Tell Cargo to rerun if icon.properties or any asset changes
    println!("cargo:rerun-if-changed={}", icon_properties_path);
    println!("cargo:rerun-if-changed=src/assets/dark");
    println!("cargo:rerun-if-changed=src/assets/light");
}
