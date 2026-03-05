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
#[serde(tag = "status", rename_all = "camelCase")]
pub enum HealthResponse {
  Ready,
  NotReady { reason: String },
}
