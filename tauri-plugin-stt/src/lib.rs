use tauri::{
  plugin::{Builder, TauriPlugin},
  Manager, Runtime,
};

pub use models::*;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod commands;
mod error;
mod models;
mod bootstrap_manager;
mod sidecar_uv;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::TauriPluginStt;
#[cfg(mobile)]
use mobile::TauriPluginStt;

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
      commands::stt_health
    ])
    .setup(|app, api| {
      #[cfg(mobile)]
      let tauri_plugin_stt = mobile::init(app, api)?;
      #[cfg(desktop)]
      let tauri_plugin_stt = desktop::init(app, api)?;
      app.manage(tauri_plugin_stt);
      Ok(())
    })
    .build()
}
