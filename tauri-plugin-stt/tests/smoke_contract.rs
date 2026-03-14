use serde_json::Value;
use tauri_plugin_stt::{
    BootstrapResponse, DiagnosticEntry, Error, HealthResponse, TranscribeRequest,
};

#[test]
fn bootstrap_ready_serializes_with_tagged_status() {
    // Verify bootstrap response contract.

    let payload = BootstrapResponse::Ready {
        details: "Bootstrap complete".to_string(),
    };
    let value = serde_json::to_value(payload).expect("bootstrap response should serialize");

    assert_eq!(value["status"], "ready");
    assert_eq!(value["details"], "Bootstrap complete");
}

#[test]
fn health_not_ready_serializes_reason_and_diagnostics() {
    // Verify readiness diagnostics shape.

    let payload = HealthResponse::NotReady {
        reason: "Virtual environment not found".to_string(),
        diagnostics: vec![DiagnosticEntry {
            name: "python_venv".to_string(),
            ready: false,
            reason: Some("Virtual environment not found".to_string()),
        }],
    };
    let value = serde_json::to_value(payload).expect("health response should serialize");

    assert_eq!(value["status"], "notReady");
    assert_eq!(value["reason"], "Virtual environment not found");
    assert_eq!(value["diagnostics"][0]["name"], "python_venv");
    assert_eq!(value["diagnostics"][0]["ready"], false);
}

#[test]
fn transcribe_request_serializes_camel_case_fields() {
    // Verify transcribe request payload shape.

    let payload = TranscribeRequest {
        path: "/tmp/sample.wav".to_string(),
        model_id: Some("tiny".to_string()),
    };
    let value = serde_json::to_value(payload).expect("transcribe request should serialize");

    assert_eq!(value["path"], "/tmp/sample.wav");
    assert_eq!(value["modelId"], "tiny");
}

#[test]
fn error_serialization_uses_stable_not_ready_code() {
    // Verify not-ready typed error contract.

    let value = serialize_error(Error::not_ready("Environment not bootstrapped"));

    assert_eq!(value["code"], "not_ready");
    assert_eq!(value["message"], "Environment not bootstrapped");
    assert!(value["details"].is_null());
}

#[test]
fn error_serialization_uses_stable_generation_failed_code() {
    // Verify generation-failed typed error contract.

    let value = serialize_error(Error::generation_failed("Transcription failed"));

    assert_eq!(value["code"], "generation_failed");
    assert_eq!(value["message"], "Transcription failed");
    assert!(value["details"].is_null());
}

fn serialize_error(err: Error) -> Value {
    serde_json::to_value(err).expect("error should serialize")
}

#[test]
fn error_serialization_uses_feature_disabled_code() {
    let value = serialize_error(Error::feature_disabled("Bridge is disabled"));

    assert_eq!(value["code"], "feature_disabled");
    assert_eq!(value["message"], "Bridge is disabled");
    assert!(value["details"].is_null());
}

#[test]
fn recorder_bridge_shapes_serialize() {
    use tauri_plugin_stt::{RuntimeState, Phase, PermissionState, GetRuntimeStateResponse, Readiness, AggregateStatus};

    let state = RuntimeState {
        phase: Phase::Recording,
        mic_permission: PermissionState::Granted,
    };

    let readiness = Readiness {
        aggregate_status: AggregateStatus::Ready,
        recording_ready: true,
        shortcut_ready: true,
        checks: vec![],
        issues: vec![],
    };

    let response = GetRuntimeStateResponse {
        contract_version: "0.1.0".to_string(),
        state,
        readiness,
    };

    let value = serde_json::to_value(response).expect("GetRuntimeStateResponse should serialize");
    
    // Testing casing translations
    assert_eq!(value["contractVersion"], "0.1.0");
    assert_eq!(value["state"]["phase"], "recording");
    assert_eq!(value["state"]["micPermission"], "granted");
    assert_eq!(value["readiness"]["aggregateStatus"], "ready");
    assert_eq!(value["readiness"]["recordingReady"], true);
    assert_eq!(value["readiness"]["shortcutReady"], true);
}
