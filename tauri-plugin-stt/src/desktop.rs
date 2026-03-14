use serde::de::DeserializeOwned;
use std::process::Command;
use std::path::PathBuf;
use tauri::{plugin::PluginApi, AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_shell::ShellExt;

use crate::models::*;

const DEFAULT_MODEL: &str = "tiny";

const ALLOWLISTED_MODELS: &[&str] = &[
    "tiny",
    "tiny.en",
    "base",
    "base.en",
    "small",
    "small.en",
    "medium",
    "medium.en",
    "large",
    "large-v1",
    "large-v2",
    "large-v3",
    "large-v3-turbo",
    "turbo",
];

fn model_repo_for(model_id: &str) -> String {
    format!("mlx-community/whisper-{}", model_id)
}

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<TauriPluginStt<R>> {
    Ok(TauriPluginStt(app.clone()))
}

/// Access to the tauri-plugin-stt APIs.
pub struct TauriPluginStt<R: Runtime>(AppHandle<R>);

impl<R: Runtime> TauriPluginStt<R> {
    pub async fn bootstrap_stt(
        &self,
        payload: BootstrapRequest,
    ) -> crate::Result<BootstrapResponse> {
        crate::bootstrap_manager::BootstrapManager::bootstrap_stt(&self.0, payload).await
    }

    pub async fn transcribe_file(
        &self,
        payload: TranscribeRequest,
    ) -> crate::Result<TranscribeResponse> {
        // 1. Enforce readiness gate
        let health = self.stt_health(HealthRequest {})?;
        if let HealthResponse::NotReady { reason, .. } = health {
            return Err(crate::Error::not_ready(reason));
        }

        // 2. Reject invalid paths early
        let audio_path = PathBuf::from(&payload.path);
        if !audio_path.exists() || !audio_path.is_file() {
            return Err(crate::Error::invalid_input(
                "Audio file does not exist or is not a file",
            ));
        }

        // 3. Resolve effective model ID
        let model_id = payload.model_id.as_deref().unwrap_or(DEFAULT_MODEL);

        // 4. Validate effective model ID against allowlist
        if !ALLOWLISTED_MODELS.contains(&model_id) {
            return Err(crate::Error::invalid_input(
                "Invalid or unsupported model ID",
            ));
        }

        // 5. Execute transcription using app-local python venv + mlx_whisper.
        let app_data_dir = self.0.path().app_data_dir().map_err(|e| {
            crate::Error::not_ready(format!("Cannot resolve app data dir: {}", e))
        })?;
        let python_bin = app_data_dir.join("python").join(".venv").join("bin").join("python");
        if !python_bin.exists() {
            return Err(crate::Error::not_ready(format!(
                "Python binary not found at {}",
                python_bin.display()
            )));
        }

        let script_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("scripts")
            .join("transcribe.py");
        if !script_path.exists() {
            return Err(crate::Error::generation_failed(format!(
                "Transcription script not found at {}",
                script_path.display()
            )));
        }

        let output = Command::new(&python_bin)
            .arg(&script_path)
            .arg("--audio")
            .arg(audio_path.as_os_str())
            .arg("--model")
            .arg(model_repo_for(model_id))
            .output()
            .map_err(|e| {
                crate::Error::generation_failed(format!("Failed to execute python transcriber: {e}"))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let message = if stderr.is_empty() {
                format!("Transcriber failed with status {}", output.status)
            } else {
                format!("Transcriber failed: {}", stderr)
            };
            return Err(crate::Error::generation_failed(message));
        }

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if text.is_empty() {
            return Err(crate::Error::generation_failed(
                "Transcriber returned empty output",
            ));
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // Publish overlay text payload using existing overlay pathway
        let event = crate::models::LiveEvent {
            contract_version: crate::models::CONTRACT_VERSION.to_string(),
            event_id: format!("live-{}", now),
            session_id: None,
            seq: 0,
            emitted_at_ms: now,
            text: text.clone(),
        };
        let _ = self.0.emit("tauri-audio-plugin://live", event);

        Ok(TranscribeResponse {
            text,
        })
    }

    pub fn stt_health(&self, _payload: HealthRequest) -> crate::Result<HealthResponse> {
        let mut diagnostics = Vec::new();

        // 1. App Data Dir check
        let app_data_dir = match self.0.path().app_data_dir() {
            Ok(dir) => {
                diagnostics.push(DiagnosticEntry {
                    name: "app_data_dir".into(),
                    ready: true,
                    reason: None,
                });
                dir
            }
            Err(e) => {
                let reason = format!("Cannot resolve app data dir: {}", e);
                diagnostics.push(DiagnosticEntry {
                    name: "app_data_dir".into(),
                    ready: false,
                    reason: Some(reason.clone()),
                });
                return Ok(HealthResponse::NotReady {
                    reason,
                    diagnostics,
                });
            }
        };

        // 2. sidecar check
        let sidecar_ready = self.0.shell().sidecar("uv").is_ok();
        diagnostics.push(DiagnosticEntry {
            name: "uv_sidecar".into(),
            ready: sidecar_ready,
            reason: if sidecar_ready {
                None
            } else {
                Some("uv sidecar not available".into())
            },
        });

        // 3. venv check
        let python_dir = app_data_dir.join("python");
        let venv_dir = python_dir.join(".venv");
        let venv_ready = venv_dir.exists();
        diagnostics.push(DiagnosticEntry {
            name: "python_venv".into(),
            ready: venv_ready,
            reason: if venv_ready {
                None
            } else {
                Some("Virtual environment not found".into())
            },
        });

        // 4. python bin check
        let python_bin = venv_dir.join("bin").join("python");
        let python_bin_ready = python_bin.exists();
        diagnostics.push(DiagnosticEntry {
            name: "python_binary".into(),
            ready: python_bin_ready,
            reason: if python_bin_ready {
                None
            } else {
                Some("Python binary not found".into())
            },
        });

        // Aggregate results
        if diagnostics.iter().all(|d| d.ready) {
            Ok(HealthResponse::Ready { diagnostics })
        } else {
            let reason = diagnostics
                .iter()
                .find(|d| !d.ready)
                .and_then(|d| d.reason.clone())
                .unwrap_or_else(|| "Unknown readiness failure".into());

            Ok(HealthResponse::NotReady {
                reason,
                diagnostics,
            })
        }
    }

}
