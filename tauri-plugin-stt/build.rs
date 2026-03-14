const COMMANDS: &[&str] = &[
    "bootstrap_stt",
    "transcribe_file",
    "stt_health",
    "initialize_recorder_runtime",
    "start_recording",
    "stop_recording",
    "set_hotkey_bindings",
    "capture_hotkey",
    "get_runtime_state",
    "set_output_destination",
    "get_output_destination",
    "set_overlay_mode",
    "get_overlay_mode",
];
fn main() {
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=permissions");
    println!("cargo:rerun-if-changed=scripts");
    println!("cargo:rerun-if-changed=requirements");

    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .ios_path("ios")
        .build();
}
