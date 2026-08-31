use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

pub use models::*;

#[cfg(desktop)]
mod desktop;

mod bootstrap_manager;
mod commands;
mod error;
mod models;
mod sidecar_uv;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::TauriPluginVoice;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the tauri-plugin-voice APIs.
pub trait TauriPluginVoiceExt<R: Runtime> {
    fn tauri_plugin_voice(&self) -> &TauriPluginVoice<R>;
}

impl<R: Runtime, T: Manager<R>> crate::TauriPluginVoiceExt<R> for T {
    fn tauri_plugin_voice(&self) -> &TauriPluginVoice<R> {
        self.state::<TauriPluginVoice<R>>().inner()
    }
}

/// Initializes the plugin with configuration.
pub fn init<R: Runtime>() -> TauriPlugin<R, Config> {
    Builder::<R, Config>::new("voice")
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap_voice,
            commands::download_model,
            commands::transcribe_file,
            commands::synthesize_speech,
            commands::stream_speech,
            commands::cancel_speech,
            commands::voice_health,
            commands::setup_record_transcribe_pipeline,
            commands::initialize_recorder_runtime,
            commands::start_listening,
            commands::respond,
            commands::start_recording,
            commands::stop_recording,
            commands::set_hotkey_bindings,
            commands::capture_hotkey,
            commands::get_runtime_state,
            commands::set_output_destination,
            commands::get_output_destination,
            commands::set_overlay_mode,
            commands::get_overlay_mode,
            commands::list_declared_states,
            commands::register_state,
        ])
        .setup(|app, api| {
            #[cfg(desktop)]
            let config = api.config().clone();
            
            #[cfg(desktop)]
            let tauri_plugin_voice = desktop::init(app, config.clone())?;

            #[cfg(desktop)]
            {
                // Perform background automated model load at startup as per CODIN-269.
                let plugin_clone = tauri_plugin_voice.clone();
                tauri::async_runtime::spawn(async move {
                    plugin_clone.startup_preload(config).await;
                });
            }

            app.manage(tauri_plugin_voice);
            Ok(())
        })
        .build()
}
