use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

use crate::models::*;

pub fn init<R: Runtime, C: DeserializeOwned>(
  app: &AppHandle<R>,
  _api: PluginApi<R, C>,
) -> crate::Result<TauriPluginStt<R>> {
  Ok(TauriPluginStt(app.clone()))
}

/// Access to the tauri-plugin-stt APIs.
pub struct TauriPluginStt<R: Runtime>(AppHandle<R>);

impl<R: Runtime> TauriPluginStt<R> {
  pub fn bootstrap_stt(&self, _payload: BootstrapRequest) -> crate::Result<BootstrapResponse> {
    Err(crate::Error::NotReady("Not implemented".into()))
  }

  pub fn transcribe_file(&self, _payload: TranscribeRequest) -> crate::Result<TranscribeResponse> {
    Err(crate::Error::NotReady("Not implemented".into()))
  }

  pub fn stt_health(&self, _payload: HealthRequest) -> crate::Result<HealthResponse> {
    Ok(HealthResponse {
      status: "not_ready".into(),
    })
  }
}
