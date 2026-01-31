# Build the release version
cargo build --release

Write-Host "Build complete! Executable at: target\release\razer-taskbar.exe" -ForegroundColor Green
Write-Host "Size: $((Get-Item target\release\razer-taskbar.exe).Length / 1MB) MB" -ForegroundColor Cyan
