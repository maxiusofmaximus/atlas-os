// OpenCode OS — Tauri build script.
// Generates Tauri capability/permissions and exposes them at compile time.
fn main() {
    // tauri_build reads tauri.conf.json + capabilities/ and emits build artifacts.
    tauri_build::build();
    println!("cargo:rerun-if-changed=tauri.conf.json");
    println!("cargo:rerun-if-changed=capabilities");
}
