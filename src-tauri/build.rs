fn main() {
    // tauri_build only watches tauri.conf.json, so a changed icon left the old
    // Win32 resource in the exe (window icon updated, Explorer icon didn't).
    println!("cargo:rerun-if-changed=icons");
    tauri_build::build()
}
