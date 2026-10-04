// Atlas OS — build script.
// Tauri capability/permission generation only runs when the `tauri` feature is
// on. A CLI-only build (`--no-default-features --features cli`, e.g. the static
// binary shipped to Linux task containers) skips it entirely: no GTK/glib, no
// tauri-build, so the binary is portable.
fn main() {
    if std::env::var_os("CARGO_FEATURE_TAURI").is_some() {
        // tauri_build reads tauri.conf.json + capabilities/ and emits build artifacts.
        tauri_build::build();
        println!("cargo:rerun-if-changed=tauri.conf.json");
        println!("cargo:rerun-if-changed=capabilities");
    }
}
