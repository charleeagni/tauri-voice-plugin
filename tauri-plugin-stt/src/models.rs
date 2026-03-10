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

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartRecordingRequest {
    pub output_dir: Option<String>,
    pub file_name_prefix: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartRecordingResponse {
    pub session_id: String,
    pub status: String,
    pub started_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StopRecordingRequest {
    pub session_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StopRecordingResponse {
    pub status: String,
    pub path: String,
    pub duration_ms: u64,
}
