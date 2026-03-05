const COMMANDS: &[&str] = &["bootstrap_stt", "transcribe_file", "stt_health"];

fn main() {
  tauri_plugin::Builder::new(COMMANDS)
    .android_path("android")
    .ios_path("ios")
    .build();
}
