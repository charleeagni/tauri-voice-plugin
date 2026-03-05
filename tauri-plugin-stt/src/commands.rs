use tauri::{AppHandle, command, Runtime};

use crate::models::*;
use crate::Result;
use crate::TauriPluginSttExt;

#[command]
pub(crate) async fn bootstrap_stt<R: Runtime>(
    app: AppHandle<R>,
    payload: BootstrapRequest,
) -> Result<BootstrapResponse> {
    app.tauri_plugin_stt().bootstrap_stt(payload).await
}

#[command]
pub(crate) async fn transcribe_file<R: Runtime>(
    app: AppHandle<R>,
    payload: TranscribeRequest,
) -> Result<TranscribeResponse> {
    app.tauri_plugin_stt().transcribe_file(payload).await
}

#[command]
pub(crate) async fn stt_health<R: Runtime>(
    app: AppHandle<R>,
    payload: HealthRequest,
) -> Result<HealthResponse> {
    app.tauri_plugin_stt().stt_health(payload)
}
