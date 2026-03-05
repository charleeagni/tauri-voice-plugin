use serde::de::DeserializeOwned;
use tauri::{
  plugin::{PluginApi, PluginHandle},
  AppHandle, Runtime,
};

use crate::models::*;

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_tauri_plugin_stt);

// initializes the Kotlin or Swift plugin classes
pub fn init<R: Runtime, C: DeserializeOwned>(
  _app: &AppHandle<R>,
  api: PluginApi<R, C>,
) -> crate::Result<TauriPluginStt<R>> {
  #[cfg(target_os = "android")]
  let handle = api.register_android_plugin("", "ExamplePlugin")?;
  #[cfg(target_os = "ios")]
  let handle = api.register_ios_plugin(init_plugin_tauri_plugin_stt)?;
  Ok(TauriPluginStt(handle))
}

/// Access to the tauri-plugin-stt APIs.
pub struct TauriPluginStt<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> TauriPluginStt<R> {
  pub async fn bootstrap_stt(&self, payload: BootstrapRequest) -> crate::Result<BootstrapResponse> {
    self
      .0
      .run_mobile_plugin("bootstrap_stt", payload)
      .map_err(Into::into)
  }

  pub async fn transcribe_file(&self, payload: TranscribeRequest) -> crate::Result<TranscribeResponse> {
    self
      .0
      .run_mobile_plugin("transcribe_file", payload)
      .map_err(Into::into)
  }

  pub fn stt_health(&self, payload: HealthRequest) -> crate::Result<HealthResponse> {
    self
      .0
      .run_mobile_plugin("stt_health", payload)
      .map_err(Into::into)
  }
}
