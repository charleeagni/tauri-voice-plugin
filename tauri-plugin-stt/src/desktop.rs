use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Manager, Runtime};
use tauri_plugin_shell::ShellExt;
use std::path::PathBuf;

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
  pub async fn bootstrap_stt(&self, payload: BootstrapRequest) -> crate::Result<BootstrapResponse> {
    crate::bootstrap_manager::BootstrapManager::bootstrap_stt(&self.0, payload).await
  }

  pub async fn transcribe_file(&self, payload: TranscribeRequest) -> crate::Result<TranscribeResponse> {
    // 1. Enforce readiness gate
    let health = self.stt_health(HealthRequest {})?;
    if let HealthResponse::NotReady { reason } = health {
        return Err(crate::Error::NotReady(reason));
    }

    // 2. Reject invalid paths early
    let audio_path = PathBuf::from(&payload.path);
    if !audio_path.exists() || !audio_path.is_file() {
        return Err(crate::Error::InvalidInput("Audio file does not exist or is not a file".into()));
    }

    // 3. Execute transcription path based on command-specific runtime path
    // Placeholder execution until real python script is introduced in subsequent tickets.
    // Real execution would spawn python sidecar and wait for response.
    
    Ok(TranscribeResponse {
      text: "Simulated transcription payload".into(),
    })
  }

  pub fn stt_health(&self, _payload: HealthRequest) -> crate::Result<HealthResponse> {
    let app_data_dir = match self.0.path().app_data_dir() {
        Ok(dir) => dir,
        Err(_) => return Ok(HealthResponse::NotReady { reason: "Cannot resolve app data dir".into() }),
    };
    
    // Check venv path
    let python_dir = app_data_dir.join("python");
    let venv_dir = python_dir.join(".venv");
    let python_bin = venv_dir.join("bin").join("python");

    if !venv_dir.exists() {
        return Ok(HealthResponse::NotReady { reason: "Virtual environment not found".into() });
    }
    if !python_bin.exists() {
        return Ok(HealthResponse::NotReady { reason: "Python binary not found".into() });
    }

    // Verify uv sidecar is resolvable and executable (by checking if we can spawn it or just resolve it)
    if self.0.shell().sidecar("uv").is_err() {
        return Ok(HealthResponse::NotReady { reason: "uv sidecar not available".into() });
    }

    Ok(HealthResponse::Ready)
  }
}
