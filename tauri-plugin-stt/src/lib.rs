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
use desktop::TauriPluginStt;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the tauri-plugin-stt APIs.
pub trait TauriPluginSttExt<R: Runtime> {
    fn tauri_plugin_stt(&self) -> &TauriPluginStt<R>;
}

impl<R: Runtime, T: Manager<R>> crate::TauriPluginSttExt<R> for T {
    fn tauri_plugin_stt(&self) -> &TauriPluginStt<R> {
        self.state::<TauriPluginStt<R>>().inner()
    }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("stt")
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap_stt,
            commands::transcribe_file,
            commands::stt_health,
            commands::setup_record_transcribe_pipeline,
            commands::initialize_recorder_runtime,
            commands::start_recording,
            commands::stop_recording,
            commands::set_hotkey_bindings,
            commands::capture_hotkey,
            commands::get_runtime_state,
            commands::set_output_destination,
            commands::get_output_destination,
            commands::set_overlay_mode,
            commands::get_overlay_mode,
        ])
        .setup(|app, api| {
            // #[cfg(feature = "recorder-bridge")]
            // {
            //     // Register recorder if bridge is enabled
            //     // app.plugin(tauri_plugin_recorder::init())?;
            // }

            #[cfg(desktop)]
            let tauri_plugin_stt = desktop::init(app, api)?;
            app.manage(tauri_plugin_stt);
            Ok(())
        })
        .build()
}
