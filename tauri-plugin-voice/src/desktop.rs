use std::fs;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Listener, Manager, Runtime};
use tauri_plugin_shell::ShellExt;

use crate::models::*;

#[derive(Debug, Clone, PartialEq)]
pub enum StartupPreloadStatus {
    Idle,
    Running,
    Ready,
    Failed,
}

const DEFAULT_MODEL: &str = "tiny.en";
const DEFAULT_TTS_MODEL: &str = "mlx-community/Kokoro-82M-bf16";
const DEFAULT_TTS_VOICE: &str = "af_heart";
const DEFAULT_TTS_LANGUAGE: &str = "a";
const DEFAULT_TTS_SPEED: f32 = 1.0;
const TTS_MAX_TEXT_CHARS: usize = 5000;
const TTS_OUTPUT_SUBDIR: &str = "tts/output";
const TTS_FORMAT: &str = "wav";
const PROGRESS_CHANNEL: &str = "tauri-voice-plugin://model-progress";
const TTS_STREAM_CHANNEL: &str = "tauri-voice-plugin://tts-stream";
const COMPLETE_CHANNEL: &str = "tauri-audio-plugin://complete";
const STATE_CHANNEL: &str = "tauri-audio-plugin://state";
const LIVE_CHANNEL: &str = "tauri-audio-plugin://live";

const DEFAULT_STREAM_FORMAT: &str = "pcm_s16le";
const DEFAULT_STREAM_SAMPLE_RATE_HZ: u32 = 24000;
const DEFAULT_STREAM_CHANNELS: u16 = 1;
const DEFAULT_CHUNK_DURATION_MS: u32 = 200;
const MIN_CHUNK_DURATION_MS: u32 = 50;
const MAX_CHUNK_DURATION_MS: u32 = 1000;

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
const ALLOWLISTED_TTS_MODELS: &[&str] = &[DEFAULT_TTS_MODEL];
const ALLOWLISTED_TTS_VOICES: &[&str] = &[
    "af_heart",
    "af_bella",
    "af_nova",
    "af_sky",
    "am_adam",
    "am_echo",
    "bf_alice",
    "bf_emma",
    "bm_daniel",
    "bm_george",
];
const ALLOWLISTED_TTS_LANGUAGES: &[&str] = &["a", "b"];

fn model_repo_for(model_id: &str) -> String {
    // These repos exist without the -mlx suffix in mlx-community.
    const NO_MLX_SUFFIX: &[&str] = &["large-v3-turbo", "turbo"];

    if NO_MLX_SUFFIX.contains(&model_id) {
        format!("mlx-community/whisper-{}", model_id)
    } else {
        format!("mlx-community/whisper-{}-mlx", model_id)
    }
}

pub fn init<R: Runtime>(app: &AppHandle<R>) -> crate::Result<TauriPluginVoice<R>> {
    Ok(TauriPluginVoice {
        app: app.clone(),
        pipeline_state: Arc::new(Mutex::new(PipelineRuntimeState::default())),
        stt: SttEngineState::default(),
        tts: TtsEngineState::default(),
    })
}

#[derive(Clone)]
struct SttEngineState {
    worker: Arc<Mutex<Option<SttWorker>>>,
    download_in_progress: Arc<Mutex<bool>>,
    startup_error: Arc<Mutex<Option<String>>>,
    startup_preload_status: Arc<Mutex<StartupPreloadStatus>>,
}

impl Default for SttEngineState {
    fn default() -> Self {
        Self {
            worker: Arc::new(Mutex::new(None)),
            download_in_progress: Arc::new(Mutex::new(false)),
            startup_error: Arc::new(Mutex::new(None)),
            startup_preload_status: Arc::new(Mutex::new(StartupPreloadStatus::Idle)),
        }
    }
}

#[derive(Clone)]
struct TtsEngineState {
    worker: Arc<Mutex<Option<TtsWorker>>>,
    lifecycle_state: Arc<Mutex<LifecycleState>>,
    startup_error: Arc<Mutex<Option<String>>>,
    startup_preload_status: Arc<Mutex<StartupPreloadStatus>>,
    active_model_id: Arc<Mutex<Option<String>>>,

    // Active streaming synthesis ID; None when idle.
    active_stream_id: Arc<Mutex<Option<String>>>,

    // Set to true when cancel_speech is called for the active stream.
    stream_cancelled: Arc<Mutex<bool>>,

    // Sender for routing stream_* worker lines to the stream-reader thread.
    stream_line_tx: Arc<Mutex<Option<mpsc::Sender<String>>>>,
}

impl Default for TtsEngineState {
    fn default() -> Self {
        Self {
            worker: Arc::new(Mutex::new(None)),
            lifecycle_state: Arc::new(Mutex::new(LifecycleState::Uninitialized)),
            startup_error: Arc::new(Mutex::new(None)),
            startup_preload_status: Arc::new(Mutex::new(StartupPreloadStatus::Idle)),
            active_model_id: Arc::new(Mutex::new(None)),
            active_stream_id: Arc::new(Mutex::new(None)),
            stream_cancelled: Arc::new(Mutex::new(false)),
            stream_line_tx: Arc::new(Mutex::new(None)),
        }
    }
}

impl TtsEngineState {
    fn health(&self) -> EngineHealth {
        let lifecycle_state = self.lifecycle_state.lock().unwrap().clone();
        let startup_status = self.startup_preload_status.lock().unwrap().clone();
        let startup_error = self.startup_error.lock().unwrap().clone();
        let active_model_id = self.active_model_id.lock().unwrap().clone();
        let worker_ready = self
            .worker
            .lock()
            .expect("tts worker mutex poisoned")
            .is_some();
        let reason = startup_error
            .or_else(|| active_model_id.map(|model| format!("TTS model active: {model}")))
            .unwrap_or_else(|| "TTS engine is not configured".into());

        EngineHealth {
            engine: Engine::Tts,
            lifecycle_state,
            ready: worker_ready,
            required: false,
            diagnostics: vec![
                DiagnosticEntry {
                    name: "tts_startup_preload".into(),
                    ready: startup_status != StartupPreloadStatus::Failed,
                    reason: Some(match startup_status {
                        StartupPreloadStatus::Idle => "TTS startup preload is disabled".into(),
                        StartupPreloadStatus::Running => "TTS startup preload is running".into(),
                        StartupPreloadStatus::Ready => "TTS startup preload is ready".into(),
                        StartupPreloadStatus::Failed => "TTS startup preload failed".into(),
                    }),
                },
                DiagnosticEntry {
                    name: "tts_worker".into(),
                    ready: worker_ready,
                    reason: Some(reason),
                },
            ],
        }
    }
}

impl SttEngineState {
    fn health(&self) -> EngineHealth {
        let preload_status = self.startup_preload_status.lock().unwrap().clone();
        let is_downloading = *self.download_in_progress.lock().unwrap();
        let startup_error = self.startup_error.lock().unwrap().clone();
        let worker_ready = self.worker.lock().expect("worker mutex poisoned").is_some();

        let preload_diagnostic = match preload_status {
            StartupPreloadStatus::Running => DiagnosticEntry {
                name: "stt_startup_preload".into(),
                ready: false,
                reason: Some("STT startup preload is still in progress".into()),
            },
            StartupPreloadStatus::Failed => DiagnosticEntry {
                name: "stt_startup_preload".into(),
                ready: false,
                reason: Some(startup_error.unwrap_or_else(|| "STT startup preload failed".into())),
            },
            StartupPreloadStatus::Idle => DiagnosticEntry {
                name: "stt_startup_preload".into(),
                ready: false,
                reason: Some("STT startup preload has not produced a ready worker yet".into()),
            },
            StartupPreloadStatus::Ready => DiagnosticEntry {
                name: "stt_startup_preload".into(),
                ready: true,
                reason: None,
            },
        };
        let worker_diagnostic = DiagnosticEntry {
            name: "stt_worker".into(),
            ready: worker_ready,
            reason: if worker_ready {
                None
            } else {
                Some("STT worker not initialized; startup preload has not completed".into())
            },
        };

        let lifecycle_state = if preload_status == StartupPreloadStatus::Running || is_downloading {
            LifecycleState::Initializing
        } else if preload_status == StartupPreloadStatus::Failed {
            LifecycleState::Failed
        } else if worker_ready && preload_status == StartupPreloadStatus::Ready {
            LifecycleState::Ready
        } else {
            LifecycleState::Uninitialized
        };
        let diagnostics = vec![preload_diagnostic, worker_diagnostic];
        let ready = diagnostics.iter().all(|d| d.ready);

        EngineHealth {
            engine: Engine::Stt,
            lifecycle_state,
            ready,
            required: true,
            diagnostics,
        }
    }
}

/// Access to the tauri-plugin-voice APIs.
pub struct TauriPluginVoice<R: Runtime> {
    app: AppHandle<R>,
    pipeline_state: Arc<Mutex<PipelineRuntimeState>>,
    stt: SttEngineState,
    tts: TtsEngineState,
}

impl<R: Runtime> Clone for TauriPluginVoice<R> {
    fn clone(&self) -> Self {
        Self {
            app: self.app.clone(),
            pipeline_state: self.pipeline_state.clone(),
            stt: self.stt.clone(),
            tts: self.tts.clone(),
        }
    }
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

/// Persistent Python worker process for local speech synthesis.
struct TtsWorker {
    #[allow(dead_code)]
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    rx: mpsc::Receiver<String>,
    model_id: String,
}

struct ResolvedSynthesisRequest {
    text: String,
    model_id: String,
    voice_id: String,
    language_code: String,
    speed: f32,
    output_path: PathBuf,
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

impl<R: Runtime> TauriPluginVoice<R> {
    pub async fn startup_preload(&self, config: Config) {
        let model_id = config.model_id.unwrap_or_else(|| DEFAULT_MODEL.to_string());

        {
            let mut status = self.stt.startup_preload_status.lock().unwrap();
            match *status {
                StartupPreloadStatus::Running | StartupPreloadStatus::Ready => return,
                _ => {
                    *status = StartupPreloadStatus::Running;
                    *self.stt.startup_error.lock().unwrap() = None;
                }
            }
        }

        let bootstrap_result = crate::bootstrap_manager::BootstrapManager::bootstrap_voice(
            &self.app,
            BootstrapRequest {},
        )
        .await;

        if let Err(e) = bootstrap_result {
            let error_msg = format!("Startup bootstrap failed: {e}");
            eprintln!("{error_msg}");
            *self.stt.startup_error.lock().unwrap() = Some(error_msg);
            *self.stt.startup_preload_status.lock().unwrap() = StartupPreloadStatus::Failed;
            return;
        }

        match self.spawn_worker(&model_id) {
            Ok(worker) => {
                *self.stt.worker.lock().unwrap() = Some(worker);
                *self.stt.startup_error.lock().unwrap() = None;
                *self.stt.startup_preload_status.lock().unwrap() = StartupPreloadStatus::Ready;
            }
            Err(e) => {
                let error_msg = format!("Startup model load failed for {model_id}: {e}");
                eprintln!("{error_msg}");
                *self.stt.startup_error.lock().unwrap() = Some(error_msg);
                *self.stt.startup_preload_status.lock().unwrap() = StartupPreloadStatus::Failed;
            }
        }
    }

    pub async fn bootstrap_voice(
        &self,
        payload: BootstrapRequest,
    ) -> crate::Result<BootstrapResponse> {
        let response =
            crate::bootstrap_manager::BootstrapManager::bootstrap_voice(&self.app, payload).await?;

        // Spawn worker with the fixed default model if not already running.
        let mut worker_guard = self.stt.worker.lock().expect("worker mutex poisoned");
        if worker_guard.is_none() {
            *worker_guard = Some(self.spawn_worker(DEFAULT_MODEL)?);
        }

        *self.stt.startup_error.lock().unwrap() = None;
        *self.stt.startup_preload_status.lock().unwrap() = StartupPreloadStatus::Ready;

        Ok(response)
    }

    pub fn download_model(
        &self,
        payload: DownloadModelRequest,
    ) -> crate::Result<DownloadModelResponse> {
        if payload.engine != Engine::Stt {
            return Err(crate::Error::invalid_input(
                "Only the STT engine supports model downloads",
            ));
        }

        // Validate model against allowlist.
        if !ALLOWLISTED_MODELS.contains(&payload.model_id.as_str()) {
            return Err(crate::Error::invalid_input(
                "Invalid or unsupported model ID",
            ));
        }

        // Fail fast if a download is already running.
        {
            let mut in_progress = self
                .stt
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
            .stt
            .download_in_progress
            .lock()
            .expect("download_in_progress mutex poisoned") = false;

        result
    }

    fn download_model_inner(&self, model_id: &str) -> crate::Result<DownloadModelResponse> {
        // Return early if worker already loaded with the requested model.
        {
            let worker_guard = self.stt.worker.lock().expect("worker mutex poisoned");
            if let Some(ref w) = *worker_guard {
                if w.model_id == model_id {
                    *self.stt.startup_error.lock().unwrap() = None;
                    *self.stt.startup_preload_status.lock().unwrap() = StartupPreloadStatus::Ready;
                    return Ok(DownloadModelResponse {
                        engine: Engine::Stt,
                        model_id: model_id.to_string(),
                        already_active: true,
                    });
                }
            }
        }

        // Tear down the existing worker if loaded with a different model.
        *self.stt.worker.lock().expect("worker mutex poisoned") = None;

        // Spawn new worker; progress events fire from reader thread during startup.
        let new_worker = self.spawn_worker(model_id)?;
        *self.stt.worker.lock().expect("worker mutex poisoned") = Some(new_worker);

        *self.stt.startup_error.lock().unwrap() = None;
        *self.stt.startup_preload_status.lock().unwrap() = StartupPreloadStatus::Ready;

        Ok(DownloadModelResponse {
            engine: Engine::Stt,
            model_id: model_id.to_string(),
            already_active: false,
        })
    }

    /// Spawns the persistent Python worker and waits for its ready signal.
    fn spawn_worker(&self, model_id: &str) -> crate::Result<SttWorker> {
        let app_data_dir =
            self.app.path().app_data_dir().map_err(|e| {
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
            .map_err(|e| crate::Error::not_ready(format!("Failed to spawn STT worker: {e}")))?;

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
                            if let Ok(line) = serde_json::from_str::<WorkerProgressLine>(&l) {
                                let now = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() as u64;
                                let event = ModelProgressEvent {
                                    contract_version: CONTRACT_VERSION.to_string(),
                                    event_id: format!("progress-{}", now),
                                    emitted_at_ms: now,
                                    engine: Engine::Stt,
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

    /// Spawns the persistent TTS worker and waits for readiness.
    fn spawn_tts_worker(&self, model_id: &str) -> crate::Result<TtsWorker> {
        let app_data_dir =
            self.app.path().app_data_dir().map_err(|e| {
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
            .join("tts_worker.py");
        if !script_path.exists() {
            return Err(crate::Error::generation_failed(format!(
                "TTS worker script not found at {}",
                script_path.display()
            )));
        }

        let mut child = Command::new(&python_bin)
            .arg(&script_path)
            .arg("--model")
            .arg(model_id)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| crate::Error::not_ready(format!("Failed to spawn TTS worker: {e}")))?;

        let stdout = child.stdout.take().expect("tts worker stdout not captured");
        let stdin = child.stdin.take().expect("tts worker stdin not captured");

        let (tx, rx) = mpsc::channel::<String>();
        let app_handle = self.app.clone();

        // Route stream_* lines to stream_line_tx; all others to tx.
        let stream_line_tx_arc = Arc::clone(&self.tts.stream_line_tx);
        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        let status = serde_json::from_str::<serde_json::Value>(&line)
                            .ok()
                            .and_then(|v| {
                                v.get("status").and_then(|s| s.as_str()).map(|s| s.to_string())
                            });

                        match status.as_deref() {
                            Some("progress") => {
                                if let Ok(progress) =
                                    serde_json::from_str::<WorkerProgressLine>(&line)
                                {
                                    let now = now_ms();
                                    let event = ModelProgressEvent {
                                        contract_version: CONTRACT_VERSION.to_string(),
                                        event_id: format!("progress-{}", now),
                                        emitted_at_ms: now,
                                        engine: Engine::Tts,
                                        model: progress.model,
                                        phase: progress.phase,
                                        state: progress.state,
                                        percent: progress.percent,
                                        filename: progress.filename,
                                        error: progress.error,
                                    };
                                    let _ = app_handle.emit(PROGRESS_CHANNEL, event);
                                }
                            }
                            Some(s) if s.starts_with("stream_") => {
                                // Route to stream reader thread if active.
                                let guard = stream_line_tx_arc.lock().unwrap();
                                if let Some(ref stream_tx) = *guard {
                                    let _ = stream_tx.send(line);
                                }
                            }
                            _ => {
                                if tx.send(line).is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let deadline = std::time::Instant::now() + Duration::from_secs(300);
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return Err(crate::Error::not_ready(
                    "TTS worker startup timeout; model download may have failed",
                ));
            }
            let line = rx.recv_timeout(remaining).map_err(|_| {
                crate::Error::not_ready(
                    "TTS worker startup timeout; model download may have failed",
                )
            })?;

            let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
                continue;
            };
            if value.get("status").and_then(|status| status.as_str()) == Some("ready") {
                break;
            }
        }

        Ok(TtsWorker {
            child,
            stdin,
            rx,
            model_id: model_id.to_string(),
        })
    }

    fn resolve_synthesis_request(
        &self,
        payload: SynthesizeSpeechRequest,
    ) -> crate::Result<ResolvedSynthesisRequest> {
        let text = payload.text.trim().to_string();
        if text.is_empty() {
            return Err(crate::Error::invalid_input("text is required"));
        }
        if text.chars().count() > TTS_MAX_TEXT_CHARS {
            return Err(crate::Error::invalid_input(
                "text exceeds the maximum length",
            ));
        }

        let model_id = payload
            .model_id
            .unwrap_or_else(|| DEFAULT_TTS_MODEL.to_string());
        if !ALLOWLISTED_TTS_MODELS.contains(&model_id.as_str()) {
            return Err(crate::Error::invalid_input(
                "Invalid or unsupported TTS model ID",
            ));
        }

        let voice_id = payload
            .voice_id
            .unwrap_or_else(|| DEFAULT_TTS_VOICE.to_string());
        if !ALLOWLISTED_TTS_VOICES.contains(&voice_id.as_str()) {
            return Err(crate::Error::invalid_input(
                "Invalid or unsupported TTS voice ID",
            ));
        }

        let language_code = payload
            .language_code
            .unwrap_or_else(|| DEFAULT_TTS_LANGUAGE.to_string());
        if !ALLOWLISTED_TTS_LANGUAGES.contains(&language_code.as_str()) {
            return Err(crate::Error::invalid_input(
                "Invalid or unsupported TTS language code",
            ));
        }

        let speed = payload.speed.unwrap_or(DEFAULT_TTS_SPEED);
        if !(0.5..=2.0).contains(&speed) {
            return Err(crate::Error::invalid_input(
                "speed must be between 0.5 and 2.0",
            ));
        }

        if payload.output_path.is_some() && payload.output_dir.is_some() {
            return Err(crate::Error::invalid_input(
                "outputPath and outputDir are mutually exclusive",
            ));
        }

        let output_path = self.resolve_tts_output_path(
            payload.output_path,
            payload.output_dir,
            payload.file_name_prefix,
        )?;

        Ok(ResolvedSynthesisRequest {
            text,
            model_id,
            voice_id,
            language_code,
            speed,
            output_path,
        })
    }

    fn resolve_tts_output_path(
        &self,
        output_path: Option<String>,
        output_dir: Option<String>,
        file_name_prefix: Option<String>,
    ) -> crate::Result<PathBuf> {
        let path = if let Some(output_path) = output_path {
            PathBuf::from(output_path)
        } else {
            let base_dir = if let Some(output_dir) = output_dir {
                PathBuf::from(output_dir)
            } else {
                self.app
                    .path()
                    .app_data_dir()
                    .map_err(|e| {
                        crate::Error::not_ready(format!("Cannot resolve app data dir: {}", e))
                    })?
                    .join(TTS_OUTPUT_SUBDIR)
            };
            let prefix = sanitize_file_prefix(file_name_prefix.as_deref().unwrap_or("tts"));
            base_dir.join(format!(
                "{}-{}-{}.wav",
                prefix,
                now_ms(),
                short_request_id()
            ))
        };

        if path.extension().and_then(|ext| ext.to_str()) != Some(TTS_FORMAT) {
            return Err(crate::Error::invalid_input(
                "TTS output path must use a .wav extension",
            ));
        }
        if path.exists() {
            return Err(crate::Error::invalid_input(
                "TTS output path already exists",
            ));
        }

        let parent = path.parent().ok_or_else(|| {
            crate::Error::invalid_input("TTS output path must include a parent directory")
        })?;
        fs::create_dir_all(parent).map_err(|e| {
            crate::Error::invalid_input(format!("Cannot create TTS output directory: {e}"))
        })?;

        Ok(path)
    }

    fn send_tts_request(
        &self,
        request: ResolvedSynthesisRequest,
    ) -> crate::Result<SynthesizeSpeechResponse> {
        let req_id = format!("tts-{}", now_ms());
        let output_path = request.output_path.to_string_lossy().to_string();
        let response_line = {
            let mut worker_guard = self.tts.worker.lock().expect("tts worker mutex poisoned");
            let worker = worker_guard.as_mut().ok_or_else(|| {
                crate::Error::not_ready("TTS worker not initialized; call synthesizeSpeech first")
            })?;

            let req_line = serde_json::json!({
                "id": req_id,
                "text": request.text,
                "output": output_path,
                "model": request.model_id,
                "voice": request.voice_id,
                "language": request.language_code,
                "speed": request.speed,
            })
            .to_string();

            if let Err(e) = writeln!(worker.stdin, "{}", req_line) {
                *worker_guard = None;
                *self.tts.lifecycle_state.lock().unwrap() = LifecycleState::Failed;
                return Err(crate::Error::generation_failed(format!(
                    "Failed to write to TTS worker stdin: {e}"
                )));
            }
            if let Err(e) = worker.stdin.flush() {
                *worker_guard = None;
                *self.tts.lifecycle_state.lock().unwrap() = LifecycleState::Failed;
                return Err(crate::Error::generation_failed(format!(
                    "Failed to flush TTS worker stdin: {e}"
                )));
            }

            match worker.rx.recv_timeout(Duration::from_secs(120)) {
                Ok(line) => line,
                Err(e) => {
                    *worker_guard = None;
                    *self.tts.lifecycle_state.lock().unwrap() = LifecycleState::Failed;
                    return Err(crate::Error::generation_failed(format!(
                        "TTS timeout or worker disconnect: {e}"
                    )));
                }
            }
        };

        let value: serde_json::Value = serde_json::from_str(&response_line).map_err(|e| {
            crate::Error::generation_failed(format!("TTS worker response parse failed: {e}. Output: {response_line}"))
        })?;

        if let Some(err) = value.get("error").and_then(|error| error.as_str()) {
            if !err.is_empty() {
                return Err(crate::Error::generation_failed(err.to_string()));
            }
        }

        let audio_path = value
            .get("audio")
            .and_then(|audio| audio.as_str())
            .ok_or_else(|| crate::Error::generation_failed("TTS worker returned no audio path"))?;
        if audio_path != output_path {
            return Err(crate::Error::generation_failed(
                "TTS worker returned an unexpected output path",
            ));
        }
        if !PathBuf::from(audio_path).is_file() {
            return Err(crate::Error::generation_failed(
                "TTS worker did not create an output file",
            ));
        }

        Ok(SynthesizeSpeechResponse {
            audio_path: audio_path.to_string(),
            model_id: value
                .get("model")
                .and_then(|model| model.as_str())
                .unwrap_or(DEFAULT_TTS_MODEL)
                .to_string(),
            voice_id: value
                .get("voice")
                .and_then(|voice| voice.as_str())
                .unwrap_or(DEFAULT_TTS_VOICE)
                .to_string(),
            language_code: value
                .get("language")
                .and_then(|language| language.as_str())
                .map(|language| language.to_string()),
            duration_ms: value
                .get("duration_ms")
                .and_then(|duration| duration.as_u64()),
            format: value
                .get("format")
                .and_then(|format| format.as_str())
                .unwrap_or(TTS_FORMAT)
                .to_string(),
            sample_rate_hz: value
                .get("sample_rate_hz")
                .and_then(|sample_rate| sample_rate.as_u64())
                .map(|sample_rate| sample_rate as u32),
            created_at_ms: now_ms(),
        })
    }

    pub async fn transcribe_file(
        &self,
        payload: TranscribeRequest,
    ) -> crate::Result<TranscribeResponse> {
        self.transcribe_file_internal(payload, true).await
    }

    pub async fn synthesize_speech(
        &self,
        payload: SynthesizeSpeechRequest,
    ) -> crate::Result<SynthesizeSpeechResponse> {
        // Reject if a streaming synthesis is active.
        if self.tts.active_stream_id.lock().unwrap().is_some() {
            return Err(crate::Error::not_ready(
                "A streaming synthesis is active; cancel it before calling synthesizeSpeech",
            ));
        }

        let request = self.resolve_synthesis_request(payload)?;

        {
            let mut worker_guard = self.tts.worker.lock().expect("tts worker mutex poisoned");
            let needs_spawn = worker_guard
                .as_ref()
                .map(|worker| worker.model_id != request.model_id)
                .unwrap_or(true);

            if needs_spawn {
                *worker_guard = None;
                *self.tts.lifecycle_state.lock().unwrap() = LifecycleState::Initializing;
                match self.spawn_tts_worker(&request.model_id) {
                    Ok(worker) => {
                        *worker_guard = Some(worker);
                        *self.tts.active_model_id.lock().unwrap() = Some(request.model_id.clone());
                        *self.tts.startup_error.lock().unwrap() = None;
                        *self.tts.startup_preload_status.lock().unwrap() =
                            StartupPreloadStatus::Ready;
                        *self.tts.lifecycle_state.lock().unwrap() = LifecycleState::Ready;
                    }
                    Err(err) => {
                        let message = err.to_string();
                        *self.tts.startup_error.lock().unwrap() = Some(message);
                        *self.tts.startup_preload_status.lock().unwrap() =
                            StartupPreloadStatus::Failed;
                        *self.tts.lifecycle_state.lock().unwrap() = LifecycleState::Failed;
                        return Err(err);
                    }
                }
            }
        }

        self.send_tts_request(request)
    }

    pub async fn stream_speech(
        &self,
        payload: StreamSpeechRequest,
    ) -> crate::Result<StreamSpeechResponse> {
        // Validate common TTS fields.
        let text = payload.text.trim().to_string();
        if text.is_empty() {
            return Err(crate::Error::invalid_input("text is required"));
        }
        if text.chars().count() > TTS_MAX_TEXT_CHARS {
            return Err(crate::Error::invalid_input("text exceeds the maximum length"));
        }

        let model_id = payload
            .model_id
            .unwrap_or_else(|| DEFAULT_TTS_MODEL.to_string());
        if !ALLOWLISTED_TTS_MODELS.contains(&model_id.as_str()) {
            return Err(crate::Error::invalid_input(
                "Invalid or unsupported TTS model ID",
            ));
        }

        let voice_id = payload
            .voice_id
            .unwrap_or_else(|| DEFAULT_TTS_VOICE.to_string());
        if !ALLOWLISTED_TTS_VOICES.contains(&voice_id.as_str()) {
            return Err(crate::Error::invalid_input(
                "Invalid or unsupported TTS voice ID",
            ));
        }

        let language_code = payload
            .language_code
            .unwrap_or_else(|| DEFAULT_TTS_LANGUAGE.to_string());
        if !ALLOWLISTED_TTS_LANGUAGES.contains(&language_code.as_str()) {
            return Err(crate::Error::invalid_input(
                "Invalid or unsupported TTS language code",
            ));
        }

        let speed = payload.speed.unwrap_or(DEFAULT_TTS_SPEED);
        if !(0.5..=2.0).contains(&speed) {
            return Err(crate::Error::invalid_input(
                "speed must be between 0.5 and 2.0",
            ));
        }

        let chunk_duration_ms = payload
            .chunk_duration_ms
            .unwrap_or(DEFAULT_CHUNK_DURATION_MS)
            .clamp(MIN_CHUNK_DURATION_MS, MAX_CHUNK_DURATION_MS);

        // Reject if another stream is already active.
        if self.tts.active_stream_id.lock().unwrap().is_some() {
            return Err(crate::Error::not_ready(
                "A streaming synthesis is already active; cancel it before starting a new one",
            ));
        }

        // Spawn or reuse the TTS worker.
        {
            let mut worker_guard = self.tts.worker.lock().expect("tts worker mutex poisoned");
            let needs_spawn = worker_guard
                .as_ref()
                .map(|w| w.model_id != model_id)
                .unwrap_or(true);

            if needs_spawn {
                *worker_guard = None;
                *self.tts.lifecycle_state.lock().unwrap() = LifecycleState::Initializing;
                match self.spawn_tts_worker(&model_id) {
                    Ok(worker) => {
                        *worker_guard = Some(worker);
                        *self.tts.active_model_id.lock().unwrap() = Some(model_id.clone());
                        *self.tts.startup_error.lock().unwrap() = None;
                        *self.tts.startup_preload_status.lock().unwrap() =
                            StartupPreloadStatus::Ready;
                        *self.tts.lifecycle_state.lock().unwrap() = LifecycleState::Ready;
                    }
                    Err(err) => {
                        let msg = err.to_string();
                        *self.tts.startup_error.lock().unwrap() = Some(msg);
                        *self.tts.startup_preload_status.lock().unwrap() =
                            StartupPreloadStatus::Failed;
                        *self.tts.lifecycle_state.lock().unwrap() = LifecycleState::Failed;
                        return Err(err);
                    }
                }
            }
        }

        let synthesis_id = format!("syn-{}", now_ms());

        // Install routing channel before writing to stdin.
        let (line_tx, line_rx) = mpsc::channel::<String>();
        *self.tts.stream_line_tx.lock().unwrap() = Some(line_tx);
        *self.tts.active_stream_id.lock().unwrap() = Some(synthesis_id.clone());
        *self.tts.stream_cancelled.lock().unwrap() = false;

        // Write stream request to worker stdin.
        {
            let mut worker_guard = self.tts.worker.lock().expect("tts worker mutex poisoned");
            let worker = worker_guard.as_mut().ok_or_else(|| {
                crate::Error::not_ready("TTS worker not initialized")
            })?;
            let req_line = serde_json::json!({
                "id": synthesis_id,
                "mode": "stream",
                "text": text,
                "model": model_id,
                "voice": voice_id,
                "language": language_code,
                "speed": speed,
                "chunk_duration_ms": chunk_duration_ms,
            })
            .to_string();
            if let Err(e) = writeln!(worker.stdin, "{}", req_line) {
                *worker_guard = None;
                *self.tts.lifecycle_state.lock().unwrap() = LifecycleState::Failed;
                *self.tts.active_stream_id.lock().unwrap() = None;
                *self.tts.stream_line_tx.lock().unwrap() = None;
                return Err(crate::Error::generation_failed(format!(
                    "Failed to write stream request to TTS worker stdin: {e}"
                )));
            }
            if let Err(e) = worker.stdin.flush() {
                *worker_guard = None;
                *self.tts.lifecycle_state.lock().unwrap() = LifecycleState::Failed;
                *self.tts.active_stream_id.lock().unwrap() = None;
                *self.tts.stream_line_tx.lock().unwrap() = None;
                return Err(crate::Error::generation_failed(format!(
                    "Failed to flush TTS worker stdin: {e}"
                )));
            }
        }

        // Spawn background thread that reads stream lines and emits Tauri events.
        let app = self.app.clone();
        let active_stream_id = Arc::clone(&self.tts.active_stream_id);
        let stream_cancelled = Arc::clone(&self.tts.stream_cancelled);
        let stream_line_tx = Arc::clone(&self.tts.stream_line_tx);
        let synthesis_id_bg = synthesis_id.clone();

        std::thread::spawn(move || {
            let mut sequence: u64 = 0;

            loop {
                // 120 s total stream timeout.
                let result = line_rx.recv_timeout(Duration::from_secs(120));

                match result {
                    Ok(line) => {
                        // Drop lines for a cancelled synthesis.
                        if *stream_cancelled.lock().unwrap() {
                            break;
                        }

                        let val: serde_json::Value = match serde_json::from_str(&line) {
                            Ok(v) => v,
                            Err(_) => continue,
                        };

                        let status =
                            val.get("status").and_then(|s| s.as_str()).unwrap_or("");

                        let event_type = match status {
                            "stream_start" => TtsStreamEventType::Start,
                            "stream_chunk" => TtsStreamEventType::Chunk,
                            "stream_complete" => TtsStreamEventType::Complete,
                            "stream_error" => TtsStreamEventType::Error,
                            "stream_cancelled" => TtsStreamEventType::Cancelled,
                            _ => continue,
                        };

                        let is_terminal = matches!(
                            event_type,
                            TtsStreamEventType::Complete
                                | TtsStreamEventType::Error
                                | TtsStreamEventType::Cancelled
                        );

                        let now = now_ms();
                        let event = TtsStreamEvent {
                            contract_version: CONTRACT_VERSION.to_string(),
                            event_id: format!("tts-stream-{now}"),
                            emitted_at_ms: now,
                            synthesis_id: synthesis_id_bg.clone(),
                            sequence,
                            event_type,
                            model_id: val
                                .get("model")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            voice_id: val
                                .get("voice")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            language_code: val
                                .get("language")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            sample_rate_hz: val
                                .get("sample_rate_hz")
                                .and_then(|v| v.as_u64())
                                .map(|v| v as u32),
                            channels: val
                                .get("channels")
                                .and_then(|v| v.as_u64())
                                .map(|v| v as u16),
                            format: val
                                .get("format")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            audio_base64: val
                                .get("audio_base64")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            duration_ms: val
                                .get("duration_ms")
                                .and_then(|v| v.as_u64())
                                .map(|v| v as u32),
                            final_chunk: val
                                .get("final")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false),
                            error: val
                                .get("error")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                        };

                        sequence += 1;
                        let _ = app.emit(TTS_STREAM_CHANNEL, event);

                        if is_terminal {
                            break;
                        }
                    }
                    Err(_) => {
                        // Timeout or sender dropped - emit error event.
                        if !*stream_cancelled.lock().unwrap() {
                            let now = now_ms();
                            let event = TtsStreamEvent {
                                contract_version: CONTRACT_VERSION.to_string(),
                                event_id: format!("tts-stream-{now}"),
                                emitted_at_ms: now,
                                synthesis_id: synthesis_id_bg.clone(),
                                sequence,
                                event_type: TtsStreamEventType::Error,
                                model_id: None,
                                voice_id: None,
                                language_code: None,
                                sample_rate_hz: None,
                                channels: None,
                                format: None,
                                audio_base64: None,
                                duration_ms: None,
                                final_chunk: false,
                                error: Some("Stream timeout or worker disconnect".to_string()),
                            };
                            let _ = app.emit(TTS_STREAM_CHANNEL, event);
                        }
                        break;
                    }
                }
            }

            // Clear streaming state.
            *active_stream_id.lock().unwrap() = None;
            *stream_cancelled.lock().unwrap() = false;
            *stream_line_tx.lock().unwrap() = None;
        });

        Ok(StreamSpeechResponse {
            synthesis_id,
            model_id,
            voice_id,
            language_code: Some(language_code),
            format: DEFAULT_STREAM_FORMAT.to_string(),
            sample_rate_hz: DEFAULT_STREAM_SAMPLE_RATE_HZ,
            channels: DEFAULT_STREAM_CHANNELS,
        })
    }

    pub fn cancel_speech(
        &self,
        payload: CancelSpeechRequest,
    ) -> crate::Result<CancelSpeechResponse> {
        let active_id = self.tts.active_stream_id.lock().unwrap().clone();

        // Return false if this synthesis ID is not the active one.
        if active_id.as_deref() != Some(payload.synthesis_id.as_str()) {
            return Ok(CancelSpeechResponse {
                synthesis_id: payload.synthesis_id,
                cancelled: false,
            });
        }

        // Mark cancelled before writing to worker to suppress incoming chunks.
        *self.tts.stream_cancelled.lock().unwrap() = true;

        // Send cancel request to the worker.
        let cancel_sent = {
            let mut worker_guard = self.tts.worker.lock().expect("tts worker mutex poisoned");
            if let Some(ref mut worker) = *worker_guard {
                let cancel_line = serde_json::json!({
                    "command": "cancel",
                    "id": payload.synthesis_id,
                })
                .to_string();
                writeln!(worker.stdin, "{}", cancel_line).is_ok()
                    && worker.stdin.flush().is_ok()
            } else {
                false
            }
        };

        // If worker is unavailable, emit cancelled event and clean up ourselves.
        if !cancel_sent {
            let now = now_ms();
            let event = TtsStreamEvent {
                contract_version: CONTRACT_VERSION.to_string(),
                event_id: format!("tts-stream-{now}"),
                emitted_at_ms: now,
                synthesis_id: payload.synthesis_id.clone(),
                sequence: 0,
                event_type: TtsStreamEventType::Cancelled,
                model_id: None,
                voice_id: None,
                language_code: None,
                sample_rate_hz: None,
                channels: None,
                format: None,
                audio_base64: None,
                duration_ms: None,
                final_chunk: false,
                error: None,
            };
            let _ = self.app.emit(TTS_STREAM_CHANNEL, event);
            *self.tts.active_stream_id.lock().unwrap() = None;
            *self.tts.stream_cancelled.lock().unwrap() = false;
            *self.tts.stream_line_tx.lock().unwrap() = None;
        }

        Ok(CancelSpeechResponse {
            synthesis_id: payload.synthesis_id,
            cancelled: true,
        })
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

            let health = self.voice_health(HealthRequest {})?;
            if let HealthResponse::NotReady { reason, .. } = health {
                return Err(crate::Error::not_ready(format!(
                    "STT is not ready: {reason}"
                )));
            }

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
                hide_overlay_on_recording_stop: Some(false),
            };
            self.app
                .recorder()
                .set_hotkey_bindings(rec_bindings, rec_config)
                .map_err(|e| crate::Error::invalid_input(e.to_string()))?;

            let app = self.app.clone();
            let pipeline_state = self.pipeline_state.clone();
            let stt = self.stt.clone();
            let tts = self.tts.clone();
            let listener_id = self.app.listen_any(COMPLETE_CHANNEL, move |event| {
                let payload = event.payload().to_string();
                let app_for_task = app.clone();
                let state_for_task = pipeline_state.clone();
                let stt_for_task = stt.clone();
                let tts_for_task = tts.clone();

                tauri::async_runtime::spawn(async move {
                    process_complete_event(
                        app_for_task,
                        state_for_task,
                        stt_for_task,
                        tts_for_task,
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
            Err(crate::Error::feature_disabled(
                "Recorder bridge is disabled",
            ))
        }
    }

    async fn transcribe_file_internal(
        &self,
        payload: TranscribeRequest,
        emit_live: bool,
    ) -> crate::Result<TranscribeResponse> {
        // 1. Enforce readiness gate (includes worker check).
        let health = self.voice_health(HealthRequest {})?;
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
            let mut worker_guard = self.stt.worker.lock().expect("worker mutex poisoned");
            let worker = worker_guard.as_mut().ok_or_else(|| {
                crate::Error::not_ready("STT worker not initialized; call bootstrapVoice first")
            })?;

            // Write JSON request line to worker stdin.
            let req_line = serde_json::json!({"id": req_id, "audio": payload.path}).to_string();

            // Clear slot on I/O failure so voice_health reflects degraded state.
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

            let resp: serde_json::Value = serde_json::from_str(&resp_line).map_err(|e| {
                crate::Error::generation_failed(format!("Worker response parse failed: {e}"))
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

    pub fn voice_health(&self, _payload: HealthRequest) -> crate::Result<HealthResponse> {
        let mut diagnostics = Vec::new();
        let stt_health = self.stt.health();
        let tts_health = self.tts.health();

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
                    lifecycle_state: LifecycleState::Failed,
                    reason,
                    engines: vec![stt_health, tts_health],
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
        diagnostics.extend(stt_health.diagnostics.clone());
        diagnostics.extend(tts_health.diagnostics.clone());

        let engines = vec![stt_health, tts_health];
        let required_engines_ready = engines
            .iter()
            .filter(|engine| engine.required)
            .all(|engine| engine.ready);
        let shared_ready = diagnostics
            .iter()
            .filter(|diagnostic| {
                !diagnostic.name.starts_with("stt_") && !diagnostic.name.starts_with("tts_")
            })
            .all(|diagnostic| diagnostic.ready);

        let lifecycle_state = if engines
            .iter()
            .filter(|engine| engine.required)
            .any(|engine| engine.lifecycle_state == LifecycleState::Failed)
        {
            LifecycleState::Failed
        } else if engines
            .iter()
            .filter(|engine| engine.required)
            .any(|engine| engine.lifecycle_state == LifecycleState::Initializing)
        {
            LifecycleState::Initializing
        } else if shared_ready && required_engines_ready {
            LifecycleState::Ready
        } else {
            LifecycleState::Uninitialized
        };

        // Aggregate results.
        if shared_ready && required_engines_ready {
            Ok(HealthResponse::Ready {
                lifecycle_state,
                engines,
                diagnostics,
            })
        } else {
            let reason = diagnostics
                .iter()
                .find(|d| !d.ready)
                .and_then(|d| d.reason.clone())
                .unwrap_or_else(|| "Unknown readiness failure".into());

            Ok(HealthResponse::NotReady {
                lifecycle_state,
                reason,
                engines,
                diagnostics,
            })
        }
    }
}

#[cfg(feature = "recorder-bridge")]
async fn process_complete_event<R: Runtime>(
    app: AppHandle<R>,
    pipeline_state: Arc<Mutex<PipelineRuntimeState>>,
    stt: SttEngineState,
    tts: TtsEngineState,
    payload: String,
) {
    use tauri_plugin_recorder::RecorderExt;

    let complete_event: CompleteEvent = match serde_json::from_str(&payload) {
        Ok(event) => event,
        Err(_) => return,
    };

    let config = {
        let state = pipeline_state
            .lock()
            .expect("pipeline state mutex poisoned");
        state.config.clone()
    };
    let Some(config) = config else {
        return;
    };

    emit_state_transition(&app, Phase::Transcribing);

    let service = TauriPluginVoice {
        app: app.clone(),
        pipeline_state,
        stt,
        tts,
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
                    let _ = app_handle.recorder().hide_overlay();
                });
            } else {
                emit_state_transition(&app, Phase::Idle);
                let _ = app.recorder().hide_overlay();
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
            let _ = app.recorder().hide_overlay();
        }
    }
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
    let value = serde_json::to_value(res).map_err(|e| {
        crate::Error::generation_failed(format!("Bridge serialization failed: {e}"))
    })?;
    serde_json::from_value(value).map_err(|e| {
        crate::Error::generation_failed(format!("Bridge schema conversion failed: {e}"))
    })
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn short_request_id() -> String {
    format!("{:x}", now_ms() % 0x100000)
}

fn sanitize_file_prefix(prefix: &str) -> String {
    let sanitized: String = prefix
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(32)
        .collect();
    if sanitized.is_empty() {
        "tts".to_string()
    } else {
        sanitized
    }
}
