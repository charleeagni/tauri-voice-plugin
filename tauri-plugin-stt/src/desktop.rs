use serde::de::DeserializeOwned;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use tauri::{plugin::PluginApi, AppHandle, Emitter, Listener, Manager, Runtime};
use tauri_plugin_shell::ShellExt;

use crate::models::*;

const DEFAULT_MODEL: &str = "tiny.en";
const PROGRESS_CHANNEL: &str = "tauri-audio-plugin://model-progress";
const COMPLETE_CHANNEL: &str = "tauri-audio-plugin://complete";
const STATE_CHANNEL: &str = "tauri-audio-plugin://state";
const LIVE_CHANNEL: &str = "tauri-audio-plugin://live";

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
    // These repos exist without the -mlx suffix in mlx-community.
    const NO_MLX_SUFFIX: &[&str] = &["large-v3-turbo", "turbo"];

    if NO_MLX_SUFFIX.contains(&model_id) {
        format!("mlx-community/whisper-{}", model_id)
    } else {
        format!("mlx-community/whisper-{}-mlx", model_id)
    }
}

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<TauriPluginStt<R>> {
    Ok(TauriPluginStt {
        app: app.clone(),
        pipeline_state: Arc::new(Mutex::new(PipelineRuntimeState::default())),
        worker: Arc::new(Mutex::new(None)),
        download_in_progress: Arc::new(Mutex::new(false)),
    })
}

/// Access to the tauri-plugin-stt APIs.
pub struct TauriPluginStt<R: Runtime> {
    app: AppHandle<R>,
    pipeline_state: Arc<Mutex<PipelineRuntimeState>>,

    // Long-lived worker process; populated by bootstrap_stt.
    worker: Arc<Mutex<Option<SttWorker>>>,

    // Guards concurrent download_model / bootstrap_stt calls.
    download_in_progress: Arc<Mutex<bool>>,
}

/// Persistent Python worker process for model-loaded transcription.
struct SttWorker {
    // Owns the process; dropped when worker is replaced or plugin shuts down.
    #[allow(dead_code)]
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    rx: mpsc::Receiver<String>,

    // Loaded model identifier; retained for health diagnostics.
    #[allow(dead_code)]
    model_id: String,
}

#[derive(Debug, Clone)]
struct PipelineConfig {
    model_id: Option<String>,
    show_final_transcript: bool,
}

#[derive(Default)]
struct PipelineRuntimeState {
    configured_once: bool,
    listener_id: Option<tauri::EventId>,
    config: Option<PipelineConfig>,
}

impl<R: Runtime> TauriPluginStt<R> {
    pub async fn bootstrap_stt(
        &self,
        payload: BootstrapRequest,
    ) -> crate::Result<BootstrapResponse> {
        let response =
            crate::bootstrap_manager::BootstrapManager::bootstrap_stt(&self.app, payload).await?;

        // Spawn worker with the fixed default model if not already running.
        let mut worker_guard = self.worker.lock().expect("worker mutex poisoned");
        if worker_guard.is_none() {
            *worker_guard = Some(self.spawn_worker(DEFAULT_MODEL)?);
        }

        Ok(response)
    }

    pub fn download_model(
        &self,
        payload: DownloadModelRequest,
    ) -> crate::Result<DownloadModelResponse> {
        // Validate model against allowlist.
        if !ALLOWLISTED_MODELS.contains(&payload.model_id.as_str()) {
            return Err(crate::Error::invalid_input(
                "Invalid or unsupported model ID",
            ));
        }

        // Fail fast if a download is already running.
        {
            let mut in_progress = self
                .download_in_progress
                .lock()
                .expect("download_in_progress mutex poisoned");
            if *in_progress {
                return Err(crate::Error::not_ready(
                    "A model is already downloading, please wait",
                ));
            }
            *in_progress = true;
        }

        let result = self.download_model_inner(&payload.model_id);

        // Always clear the in-progress flag before returning.
        *self
            .download_in_progress
            .lock()
            .expect("download_in_progress mutex poisoned") = false;

        result
    }

    fn download_model_inner(&self, model_id: &str) -> crate::Result<DownloadModelResponse> {
        // Return early if worker already loaded with the requested model.
        {
            let worker_guard = self.worker.lock().expect("worker mutex poisoned");
            if let Some(ref w) = *worker_guard {
                if w.model_id == model_id {
                    return Ok(DownloadModelResponse {
                        model_id: model_id.to_string(),
                        already_active: true,
                    });
                }
            }
        }

        // Tear down the existing worker if loaded with a different model.
        *self.worker.lock().expect("worker mutex poisoned") = None;

        // Spawn new worker; progress events fire from reader thread during startup.
        let new_worker = self.spawn_worker(model_id)?;
        *self.worker.lock().expect("worker mutex poisoned") = Some(new_worker);

        Ok(DownloadModelResponse {
            model_id: model_id.to_string(),
            already_active: false,
        })
    }

    /// Spawns the persistent Python worker and waits for its ready signal.
    fn spawn_worker(&self, model_id: &str) -> crate::Result<SttWorker> {
        let app_data_dir = self.app.path().app_data_dir().map_err(|e| {
            crate::Error::not_ready(format!("Cannot resolve app data dir: {}", e))
        })?;
        let python_bin = app_data_dir
            .join("python")
            .join(".venv")
            .join("bin")
            .join("python");
        if !python_bin.exists() {
            return Err(crate::Error::not_ready(format!(
                "Python binary not found at {}",
                python_bin.display()
            )));
        }

        let script_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("scripts")
            .join("worker.py");
        if !script_path.exists() {
            return Err(crate::Error::generation_failed(format!(
                "Worker script not found at {}",
                script_path.display()
            )));
        }

        let model_repo = model_repo_for(model_id);
        let mut child = Command::new(&python_bin)
            .arg(&script_path)
            .arg("--model")
            .arg(&model_repo)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| {
                crate::Error::not_ready(format!("Failed to spawn STT worker: {e}"))
            })?;

        let stdout = child.stdout.take().expect("worker stdout not captured");
        let stdin = child.stdin.take().expect("worker stdin not captured");

        // Background thread reads worker stdout; progress lines become Tauri events.
        let (tx, rx) = mpsc::channel::<String>();
        let app_handle = self.app.clone();
        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        // Parse JSON; progress events go to Tauri, rest to rx.
                        let is_progress = serde_json::from_str::<serde_json::Value>(&l)
                            .ok()
                            .and_then(|v| {
                                v.get("status")
                                    .and_then(|s| s.as_str())
                                    .map(|s| s == "progress")
                            })
                            .unwrap_or(false);

                        if is_progress {
                            if let Ok(line) =
                                serde_json::from_str::<WorkerProgressLine>(&l)
                            {
                                let now = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() as u64;
                                let event = ModelProgressEvent {
                                    contract_version: CONTRACT_VERSION.to_string(),
                                    event_id: format!("progress-{}", now),
                                    emitted_at_ms: now,
                                    model: line.model,
                                    phase: line.phase,
                                    state: line.state,
                                    percent: line.percent,
                                    filename: line.filename,
                                    error: line.error,
                                };
                                let _ = app_handle.emit(PROGRESS_CHANNEL, event);
                            }
                        } else if tx.send(l).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Wait for the ready signal; skip unrecognised stdout lines from libraries.
        let deadline = std::time::Instant::now() + Duration::from_secs(300);
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return Err(crate::Error::not_ready(
                    "STT worker startup timeout; model download may have failed",
                ));
            }
            let line = rx.recv_timeout(remaining).map_err(|_| {
                crate::Error::not_ready(
                    "STT worker startup timeout; model download may have failed",
                )
            })?;

            // Skip non-JSON and unrecognised lines; only stop on "ready".
            let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) else {
                continue;
            };
            match val.get("status").and_then(|s| s.as_str()) {
                Some("ready") => break,
                _ => continue,
            }
        }

        Ok(SttWorker {
            child,
            stdin,
            rx,
            model_id: model_id.to_string(),
        })
    }

    pub async fn transcribe_file(
        &self,
        payload: TranscribeRequest,
    ) -> crate::Result<TranscribeResponse> {
        self.transcribe_file_internal(payload, true).await
    }

    pub fn setup_record_transcribe_pipeline(
        &self,
        payload: SetupRecordTranscribePipelineRequest,
    ) -> crate::Result<SetupRecordTranscribePipelineResponse> {
        #[cfg(feature = "recorder-bridge")]
        {
            use tauri_plugin_recorder::RecorderExt;

            if payload.toggle_shortcut.trim().is_empty() {
                return Err(crate::Error::invalid_input(
                    "toggleShortcut is required for pipeline setup",
                ));
            }

            {
                let mut pipeline_state = self
                    .pipeline_state
                    .lock()
                    .expect("pipeline state mutex poisoned");
                if pipeline_state.configured_once {
                    return Err(crate::Error::invalid_input(
                        "Record-transcribe pipeline is already configured",
                    ));
                }
                pipeline_state.configured_once = true;
                pipeline_state.config = Some(PipelineConfig {
                    model_id: payload.model_id.clone(),
                    show_final_transcript: payload.show_final_transcript.unwrap_or(true),
                });
            }

            self.app
                .recorder()
                .initialize_recorder_runtime()
                .map_err(|e| crate::Error::not_ready(e.to_string()))?;

            let _ = self.stt_health(HealthRequest {})?;

            let recorder_config = payload.recorder_config.unwrap_or(RecorderConfig {
                output_dir: None,
                file_name_prefix: None,
            });

            let rec_bindings = tauri_plugin_recorder::HotkeyBindings {
                push_to_talk: None,
                toggle: Some(payload.toggle_shortcut),
            };
            let rec_config = tauri_plugin_recorder::RecorderConfig {
                output_dir: recorder_config.output_dir,
                file_name_prefix: recorder_config.file_name_prefix,
            };
            self.app
                .recorder()
                .set_hotkey_bindings(rec_bindings, rec_config)
                .map_err(|e| crate::Error::invalid_input(e.to_string()))?;

            let app = self.app.clone();
            let pipeline_state = self.pipeline_state.clone();
            let worker = self.worker.clone();
            let listener_id = self.app.listen_any(COMPLETE_CHANNEL, move |event| {
                let payload = event.payload().to_string();
                let app_for_task = app.clone();
                let state_for_task = pipeline_state.clone();
                let worker_for_task = worker.clone();

                tauri::async_runtime::spawn(async move {
                    process_complete_event(
                        app_for_task,
                        state_for_task,
                        worker_for_task,
                        payload,
                    )
                    .await;
                });
            });

            let mut state = self
                .pipeline_state
                .lock()
                .expect("pipeline state mutex poisoned");
            state.listener_id = Some(listener_id);

            Ok(SetupRecordTranscribePipelineResponse {
                contract_version: CONTRACT_VERSION.to_string(),
                active: true,
            })
        }
        #[cfg(not(feature = "recorder-bridge"))]
        {
            let _ = payload;
            Err(crate::Error::feature_disabled("Recorder bridge is disabled"))
        }
    }

    async fn transcribe_file_internal(
        &self,
        payload: TranscribeRequest,
        emit_live: bool,
    ) -> crate::Result<TranscribeResponse> {
        // 1. Enforce readiness gate (includes worker check).
        let health = self.stt_health(HealthRequest {})?;
        if let HealthResponse::NotReady { reason, .. } = health {
            return Err(crate::Error::not_ready(reason));
        }

        // 2. Reject invalid paths early.
        let audio_path = PathBuf::from(&payload.path);
        if !audio_path.exists() || !audio_path.is_file() {
            return Err(crate::Error::invalid_input(
                "Audio file does not exist or is not a file",
            ));
        }

        // 3. Send request to the persistent worker; receive transcript.
        let req_id = format!("req-{}", now_ms());
        let text = {
            let mut worker_guard = self.worker.lock().expect("worker mutex poisoned");
            let worker = worker_guard.as_mut().ok_or_else(|| {
                crate::Error::not_ready(
                    "STT worker not initialized; call bootstrapStt first",
                )
            })?;

            // Write JSON request line to worker stdin.
            let req_line = serde_json::json!({"id": req_id, "audio": payload.path}).to_string();

            // Clear slot on I/O failure so stt_health reflects degraded state.
            if let Err(e) = writeln!(worker.stdin, "{}", req_line) {
                *worker_guard = None;
                return Err(crate::Error::generation_failed(format!(
                    "Failed to write to worker stdin: {e}"
                )));
            }

            if let Err(e) = worker.stdin.flush() {
                *worker_guard = None;
                return Err(crate::Error::generation_failed(format!(
                    "Failed to flush worker stdin: {e}"
                )));
            }

            // Wait for response with a per-transcription timeout.
            let resp_line = match worker.rx.recv_timeout(Duration::from_secs(120)) {
                Ok(line) => line,
                Err(e) => {
                    *worker_guard = None;
                    return Err(crate::Error::generation_failed(format!(
                        "Transcription timeout or worker disconnect: {e}"
                    )));
                }
            };

            let resp: serde_json::Value =
                serde_json::from_str(&resp_line).map_err(|e| {
                    crate::Error::generation_failed(format!(
                        "Worker response parse failed: {e}"
                    ))
                })?;

            if let Some(err) = resp.get("error").and_then(|e| e.as_str()) {
                return Err(crate::Error::generation_failed(err.to_string()));
            }

            resp.get("text")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string()
        };

        if text.is_empty() {
            return Err(crate::Error::generation_failed(
                "Transcriber returned empty output",
            ));
        }

        let now = now_ms();
        if emit_live {
            let event = crate::models::LiveEvent {
                contract_version: crate::models::CONTRACT_VERSION.to_string(),
                event_id: format!("live-{}", now),
                session_id: None,
                seq: 0,
                emitted_at_ms: now,
                text: text.clone(),
            };
            let _ = self.app.emit(LIVE_CHANNEL, event);
        }

        Ok(TranscribeResponse { text })
    }

    pub fn stt_health(&self, _payload: HealthRequest) -> crate::Result<HealthResponse> {
        let mut diagnostics = Vec::new();

        // 1. App Data Dir check
        let app_data_dir = match self.app.path().app_data_dir() {
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
        let sidecar_ready = self.app.shell().sidecar("uv").is_ok();
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

        // 5. Worker check; model must be downloaded and loaded into memory.
        let worker_guard = self.worker.lock().expect("worker mutex poisoned");
        let worker_ready = worker_guard.is_some();
        diagnostics.push(DiagnosticEntry {
            name: "stt_worker".into(),
            ready: worker_ready,
            reason: if worker_ready {
                None
            } else {
                Some("STT worker not initialized; call bootstrapStt to load model".into())
            },
        });
        drop(worker_guard);

        // Aggregate results.
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

#[cfg(feature = "recorder-bridge")]
async fn process_complete_event<R: Runtime>(
    app: AppHandle<R>,
    pipeline_state: Arc<Mutex<PipelineRuntimeState>>,
    worker: Arc<Mutex<Option<SttWorker>>>,
    payload: String,
) {
    use tauri_plugin_recorder::RecorderExt;

    let complete_event: CompleteEvent = match serde_json::from_str(&payload) {
        Ok(event) => event,
        Err(_) => return,
    };

    let config = {
        let state = pipeline_state.lock().expect("pipeline state mutex poisoned");
        state.config.clone()
    };
    let Some(config) = config else {
        return;
    };

    emit_state_transition(&app, Phase::Transcribing);

    let service = TauriPluginStt {
        app: app.clone(),
        pipeline_state,
        worker,
        download_in_progress: Arc::new(Mutex::new(false)),
    };
    let transcribe_result = service
        .transcribe_file_internal(
            TranscribeRequest {
                path: complete_event.result.recording_wav_path.clone(),
                model_id: config.model_id.clone(),
            },
            false,
        )
        .await;

    match transcribe_result {
        Ok(response) => {
            println!("Pipeline transcription complete: \"{}\"", response.text);
            if config.show_final_transcript {
                let now = now_ms();
                let event = LiveEvent {
                    contract_version: CONTRACT_VERSION.to_string(),
                    event_id: format!("live-{}", now),
                    session_id: Some(complete_event.session_id),
                    seq: 0,
                    emitted_at_ms: now,
                    text: response.text,
                };
                let _ = app.emit(LIVE_CHANNEL, event);
                
                // Delay idle transition on a thread to avoid tokio dep
                let app_handle = app.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(3));
                    emit_state_transition(&app_handle, Phase::Idle);
                });
            } else {
                emit_state_transition(&app, Phase::Idle);
            }
        }
        Err(err) => {
            eprintln!("Pipeline transcription failed: {:?}", err);
            let error_event = ErrorEvent {
                contract_version: CONTRACT_VERSION.to_string(),
                event_id: format!("error-{}", now_ms()),
                emitted_at_ms: now_ms(),
                error: serde_json::to_value(err).unwrap_or(serde_json::json!({
                    "code": "internal_error",
                    "message": "Pipeline transcription failed",
                    "details": null
                })),
            };
            let _ = app.emit("tauri-audio-plugin://error", error_event);
            emit_state_transition(&app, Phase::Idle);
        }
    }

    let _ = app.recorder().set_overlay_mode(tauri_plugin_recorder::OverlayMode::Default);
}

#[cfg(feature = "recorder-bridge")]
fn emit_state_transition<R: Runtime>(app: &AppHandle<R>, phase: Phase) {
    use tauri_plugin_recorder::RecorderExt;

    let state_snapshot: Option<GetRuntimeStateResponse> = app
        .recorder()
        .get_runtime_state()
        .ok()
        .and_then(|bridge_state| convert_bridge_response(bridge_state).ok());

    let state = state_snapshot
        .as_ref()
        .map(|snapshot| RuntimeState {
            phase: phase.clone(),
            mic_permission: snapshot.state.mic_permission.clone(),
        })
        .unwrap_or(RuntimeState {
            phase,
            mic_permission: PermissionState::Unknown,
        });

    let readiness = state_snapshot
        .map(|snapshot| snapshot.readiness)
        .unwrap_or(Readiness {
            aggregate_status: AggregateStatus::Degraded,
            recording_ready: false,
            shortcut_ready: false,
            checks: vec![],
            issues: vec![],
        });

    let event = StateEvent {
        contract_version: CONTRACT_VERSION.to_string(),
        event_id: format!("state-{}", now_ms()),
        emitted_at_ms: now_ms(),
        state,
        readiness,
    };
    let _ = app.emit(STATE_CHANNEL, event);
}

#[cfg(feature = "recorder-bridge")]
fn convert_bridge_response<T, U>(res: T) -> crate::Result<U>
where
    T: serde::Serialize,
    U: serde::de::DeserializeOwned,
{
    let value = serde_json::to_value(res)
        .map_err(|e| crate::Error::generation_failed(format!("Bridge serialization failed: {e}")))?;
    serde_json::from_value(value)
        .map_err(|e| crate::Error::generation_failed(format!("Bridge schema conversion failed: {e}")))
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
