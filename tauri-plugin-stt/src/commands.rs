use tauri::{command, AppHandle, Runtime};

use crate::models::*;
use crate::Result;
use crate::TauriPluginSttExt;

fn convert_bridge_response<T, U>(res: T) -> Result<U>
where
    T: serde::Serialize,
    U: serde::de::DeserializeOwned,
{
    let value = serde_json::to_value(res)
        .map_err(|e| crate::Error::generation_failed(format!("Bridge serialization failed: {e}")))?;
    serde_json::from_value(value)
        .map_err(|e| crate::Error::generation_failed(format!("Bridge schema conversion failed: {e}")))
}

#[command]
pub(crate) async fn bootstrap_stt<R: Runtime>(
    app: AppHandle<R>,
    payload: BootstrapRequest,
) -> Result<BootstrapResponse> {
    app.tauri_plugin_stt().bootstrap_stt(payload).await
}

#[command]
pub(crate) async fn download_model<R: Runtime>(
    app: AppHandle<R>,
    payload: DownloadModelRequest,
) -> Result<DownloadModelResponse> {
    app.tauri_plugin_stt().download_model(payload)
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

#[command]
pub(crate) async fn setup_record_transcribe_pipeline<R: Runtime>(
    app: AppHandle<R>,
    payload: SetupRecordTranscribePipelineRequest,
) -> Result<SetupRecordTranscribePipelineResponse> {
    app.tauri_plugin_stt()
        .setup_record_transcribe_pipeline(payload)
}

// -----------------------------------------------------------------------------
// Recorder Bridge Commands
// -----------------------------------------------------------------------------

#[command]
pub(crate) async fn initialize_recorder_runtime<R: Runtime>(
    #[allow(unused_variables)] app: AppHandle<R>,
) -> Result<InitializeRuntimeResponse> {
    #[cfg(feature = "recorder-bridge")]
    {
        use tauri_plugin_recorder::RecorderExt;
        let res = app.recorder().initialize_recorder_runtime().map_err(|e| crate::Error::not_ready(e.to_string()))?;
        convert_bridge_response(res)
    }
    #[cfg(not(feature = "recorder-bridge"))]
    {
        Err(crate::Error::feature_disabled("Recorder bridge is disabled"))
    }
}

#[command]
pub(crate) async fn start_recording<R: Runtime>(
    #[allow(unused_variables)] app: AppHandle<R>,
    #[allow(unused_variables)] output_dir: Option<String>,
    #[allow(unused_variables)] file_name_prefix: Option<String>,
) -> Result<StartRecordingResponse> {
    #[cfg(feature = "recorder-bridge")]
    {
        use tauri_plugin_recorder::RecorderExt;
        let res = app.recorder().start_recording(output_dir, file_name_prefix, tauri_plugin_recorder::TriggerSource::Manual).map_err(|e| crate::Error::invalid_input(e.to_string()))?;
        convert_bridge_response(res)
    }
    #[cfg(not(feature = "recorder-bridge"))]
    {
        Err(crate::Error::feature_disabled("Recorder bridge is disabled"))
    }
}

#[command]
pub(crate) async fn stop_recording<R: Runtime>(
    #[allow(unused_variables)] app: AppHandle<R>,
    #[allow(unused_variables)] session_id: Option<String>,
) -> Result<StopRecordingResponse> {
    #[cfg(feature = "recorder-bridge")]
    {
        use tauri_plugin_recorder::RecorderExt;
        let res = app.recorder().stop_recording(session_id, tauri_plugin_recorder::TriggerSource::Manual).map_err(|e| crate::Error::invalid_input(e.to_string()))?;
        convert_bridge_response(res)
    }
    #[cfg(not(feature = "recorder-bridge"))]
    {
        Err(crate::Error::feature_disabled("Recorder bridge is disabled"))
    }
}

#[command]
pub(crate) async fn set_hotkey_bindings<R: Runtime>(
    #[allow(unused_variables)] app: AppHandle<R>,
    #[allow(unused_variables)] bindings: HotkeyBindings,
    #[allow(unused_variables)] recorder_config: RecorderConfig,
) -> Result<SetHotkeyBindingsResponse> {
    #[cfg(feature = "recorder-bridge")]
    {
        use tauri_plugin_recorder::RecorderExt;
        // The stt HotkeyBindings type needs to be mapped to the recorder HotkeyBindings type to compile.
        let rec_bindings = tauri_plugin_recorder::HotkeyBindings {
            push_to_talk: bindings.push_to_talk,
            toggle: bindings.toggle,
        };
        let rec_config = tauri_plugin_recorder::RecorderConfig {
            output_dir: recorder_config.output_dir,
            file_name_prefix: recorder_config.file_name_prefix,
        };
        let res = app.recorder().set_hotkey_bindings(rec_bindings, rec_config).map_err(|e| crate::Error::invalid_input(e.to_string()))?;
        convert_bridge_response(res)
    }
    #[cfg(not(feature = "recorder-bridge"))]
    {
        Err(crate::Error::feature_disabled("Recorder bridge is disabled"))
    }
}

#[command]
pub(crate) async fn capture_hotkey<R: Runtime>(
    #[allow(unused_variables)] app: AppHandle<R>,
    #[allow(unused_variables)] timeout_ms: Option<u64>,
) -> Result<CaptureHotkeyResponse> {
    #[cfg(feature = "recorder-bridge")]
    {
        use tauri_plugin_recorder::RecorderExt;
        let res = app.recorder().capture_hotkey(timeout_ms.unwrap_or(20000)).map_err(|e| crate::Error::invalid_input(e.to_string()))?;
        convert_bridge_response(res)
    }
    #[cfg(not(feature = "recorder-bridge"))]
    {
        Err(crate::Error::feature_disabled("Recorder bridge is disabled"))
    }
}

#[command]
pub(crate) async fn get_runtime_state<R: Runtime>(
    #[allow(unused_variables)] app: AppHandle<R>,
) -> Result<GetRuntimeStateResponse> {
    #[cfg(feature = "recorder-bridge")]
    {
        use tauri_plugin_recorder::RecorderExt;
        let res = app.recorder().get_runtime_state().map_err(|e| crate::Error::not_ready(e.to_string()))?;
        convert_bridge_response(res)
    }
    #[cfg(not(feature = "recorder-bridge"))]
    {
        Err(crate::Error::feature_disabled("Recorder bridge is disabled"))
    }
}

#[command]
pub(crate) async fn set_output_destination<R: Runtime>(
    #[allow(unused_variables)] app: AppHandle<R>,
    #[allow(unused_variables)] policy: DestinationPolicy,
    #[allow(unused_variables)] base_dir: Option<String>,
    #[allow(unused_variables)] subdir: Option<String>,
    #[allow(unused_variables)] file_pattern: Option<String>,
) -> Result<ConfigDestinationResponse> {
    #[cfg(feature = "recorder-bridge")]
    {
        use tauri_plugin_recorder::RecorderExt;
        let rec_policy = match policy {
            DestinationPolicy::AppData => tauri_plugin_recorder::DestinationPolicy::AppData,
            DestinationPolicy::CustomPath => tauri_plugin_recorder::DestinationPolicy::CustomPath,
            DestinationPolicy::Temp => tauri_plugin_recorder::DestinationPolicy::Temp,
        };
        let res = app.recorder().set_output_destination(rec_policy, base_dir, subdir, file_pattern).map_err(|e| crate::Error::invalid_input(e.to_string()))?;
        convert_bridge_response(res)
    }
    #[cfg(not(feature = "recorder-bridge"))]
    {
        Err(crate::Error::feature_disabled("Recorder bridge is disabled"))
    }
}

#[command]
pub(crate) async fn get_output_destination<R: Runtime>(
    #[allow(unused_variables)] app: AppHandle<R>,
) -> Result<ConfigDestinationResponse> {
    #[cfg(feature = "recorder-bridge")]
    {
        use tauri_plugin_recorder::RecorderExt;
        let res = app.recorder().get_output_destination().map_err(|e| crate::Error::not_ready(e.to_string()))?;
        convert_bridge_response(res)
    }
    #[cfg(not(feature = "recorder-bridge"))]
    {
        Err(crate::Error::feature_disabled("Recorder bridge is disabled"))
    }
}

#[command]
pub(crate) async fn set_overlay_mode<R: Runtime>(
    #[allow(unused_variables)] app: AppHandle<R>,
    #[allow(unused_variables)] overlay_mode: OverlayMode,
) -> Result<OverlayModeResponse> {
    #[cfg(feature = "recorder-bridge")]
    {
        use tauri_plugin_recorder::RecorderExt;
        let rec_mode = match overlay_mode {
            OverlayMode::Default => tauri_plugin_recorder::OverlayMode::Default,
            OverlayMode::Consumer => tauri_plugin_recorder::OverlayMode::Consumer,
            OverlayMode::Disabled => tauri_plugin_recorder::OverlayMode::Disabled,
        };
        let res = app.recorder().set_overlay_mode(rec_mode).map_err(|e| crate::Error::invalid_input(e.to_string()))?;
        convert_bridge_response(res)
    }
    #[cfg(not(feature = "recorder-bridge"))]
    {
        Err(crate::Error::feature_disabled("Recorder bridge is disabled"))
    }
}

#[command]
pub(crate) async fn get_overlay_mode<R: Runtime>(
    #[allow(unused_variables)] app: AppHandle<R>,
) -> Result<OverlayModeResponse> {
    #[cfg(feature = "recorder-bridge")]
    {
        use tauri_plugin_recorder::RecorderExt;
        let res = app.recorder().get_overlay_mode().map_err(|e| crate::Error::not_ready(e.to_string()))?;
        convert_bridge_response(res)
    }
    #[cfg(not(feature = "recorder-bridge"))]
    {
        Err(crate::Error::feature_disabled("Recorder bridge is disabled"))
    }
}
