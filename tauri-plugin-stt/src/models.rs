use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapRequest {}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum BootstrapResponse {
    Ready { details: String },
    AlreadyReady { details: String },
}

/// Partial progress payload emitted by worker.py; metadata is added by Rust.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerProgressLine {
    pub model: String,
    pub phase: ModelProgressPhase,
    pub state: ModelProgressState,
    pub percent: Option<f32>,
    pub filename: Option<String>,
    pub error: Option<String>,
}

// On-demand model download request/response.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadModelRequest {
    /// Allowlisted Whisper model ID to download and load.
    pub model_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadModelResponse {
    /// Effective model now loaded in the worker.
    pub model_id: String,

    /// True if the worker was already loaded with this model.
    pub already_active: bool,
}

// Progress phase: download (network fetch) or preload (memory load).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelProgressPhase {
    Download,
    Preload,
}

// Progress lifecycle state for a phase.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelProgressState {
    Start,
    InProgress,
    Complete,
    Failed,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelProgressEvent {
    pub contract_version: String,
    pub event_id: String,
    pub emitted_at_ms: u64,

    /// Model repo identifier being loaded.
    pub model: String,

    pub phase: ModelProgressPhase,
    pub state: ModelProgressState,

    /// Download fraction 0.0–1.0; None when unavailable.
    pub percent: Option<f32>,

    /// Active filename during download; None during preload.
    pub filename: Option<String>,

    /// Error message; present only on Failed state.
    pub error: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscribeRequest {
    pub path: String,
    pub model_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscribeResponse {
    pub text: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupRecordTranscribePipelineRequest {
    pub toggle_shortcut: String,
    pub recorder_config: Option<RecorderConfig>,
    pub model_id: Option<String>,
    pub show_final_transcript: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupRecordTranscribePipelineResponse {
    pub contract_version: String,
    pub active: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthRequest {}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticEntry {
    pub name: String,
    pub ready: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum HealthResponse {
    Ready {
        diagnostics: Vec<DiagnosticEntry>,
    },
    NotReady {
        reason: String,
        diagnostics: Vec<DiagnosticEntry>,
    },
}

// -----------------------------------------------------------------------------
// Recorder Bridge Models
// -----------------------------------------------------------------------------

pub const CONTRACT_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryMode {
    Chunks,
    File,
    Both,
}

impl Default for DeliveryMode {
    fn default() -> Self {
        Self::File
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TriggerSource {
    Manual,
    ShortcutPushToTalk,
    ShortcutToggle,
}

impl Default for TriggerSource {
    fn default() -> Self {
        Self::Manual
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DestinationPolicy {
    AppData,
    CustomPath,
    Temp,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecorderConfig {
    pub output_dir: Option<String>,
    pub file_name_prefix: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyBindings {
    pub push_to_talk: Option<String>,
    pub toggle: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputDestination {
    pub policy: DestinationPolicy,
    pub base_dir: Option<String>,
    pub subdir: Option<String>,
    pub file_pattern: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveOutputDestination {
    pub policy: DestinationPolicy,
    pub base_dir: Option<String>,
    pub subdir: Option<String>,
    pub file_pattern: Option<String>,
    pub resolved_base_dir: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Idle,
    Recording,
    Transcribing,
    Error,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum OverlayMode {
    Default,
    #[serde(rename_all = "camelCase")]
    Consumer { consumer_url: String },
    Disabled,
}

impl Default for OverlayMode {
    fn default() -> Self {
        Self::Default
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    PushToTalk,
    Toggle,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionState {
    Unknown,
    Granted,
    Denied,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeState {
    pub phase: Phase,
    pub mic_permission: PermissionState,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AggregateStatus {
    Ready,
    Degraded,
    Blocked,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub check_id: String,
    pub scope: String,
    pub status: String,
    pub code: String,
    pub guidance: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub code: String,
    pub message: String,
    pub guidance: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Readiness {
    pub aggregate_status: AggregateStatus,
    pub recording_ready: bool,
    pub shortcut_ready: bool,
    pub checks: Vec<Check>,
    pub issues: Vec<Issue>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingResult {
    pub recording_wav_path: String,
    pub duration_ms: u64,
    pub sample_rate_hz: u32,
    pub channels: u32,
    pub bit_depth: u32,
    pub delivery_mode: DeliveryMode,
    pub trigger_source: TriggerSource,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    /// Whisper model ID to load at startup. Defaults to "tiny.en".
    pub model_id: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model_id: Some("tiny.en".to_string()),
        }
    }
}

// Commands schema
// Using snake_case default by serde config (needs #[serde(rename_all = "snake_case")] if struct but fields are already snake_case)

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeRuntimeResponse {
    pub contract_version: String,
    pub state: RuntimeState,
    pub readiness: Readiness,
    pub shortcut_error: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartRecordingResponse {
    pub contract_version: String,
    pub session_id: String,
    pub trigger_source: TriggerSource,
    pub state: RuntimeState,
    pub effective_output_destination: EffectiveOutputDestination,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StopRecordingResponse {
    pub contract_version: String,
    pub result: RecordingResult,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRuntimeStateResponse {
    pub contract_version: String,
    pub state: RuntimeState,
    pub readiness: Readiness,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetHotkeyBindingsResponse {
    pub contract_version: String,
    pub state: RuntimeState,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CaptureHotkeyStatus {
    Captured,
    Cancelled,
    Timeout,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureHotkeyResponse {
    pub contract_version: String,
    pub status: CaptureHotkeyStatus,
    pub shortcut: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigDestinationResponse {
    pub contract_version: String,
    pub output_destination: OutputDestination,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayModeResponse {
    pub contract_version: String,
    pub overlay_mode: OverlayMode,
}

// Events schema
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateEvent {
    pub contract_version: String,
    pub event_id: String,
    pub emitted_at_ms: u64,
    pub state: RuntimeState,
    pub readiness: Readiness,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveEvent {
    pub contract_version: String,
    pub event_id: String,
    pub session_id: Option<String>,
    pub seq: u64,
    pub emitted_at_ms: u64,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorEvent {
    pub contract_version: String,
    pub event_id: String,
    pub emitted_at_ms: u64,
    pub error: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteEvent {
    pub contract_version: String,
    pub event_id: String,
    pub session_id: String,
    pub trigger_source: TriggerSource,
    pub emitted_at_ms: u64,
    pub result: RecordingResult,
}
