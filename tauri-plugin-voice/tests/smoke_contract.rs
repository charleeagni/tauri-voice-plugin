use serde_json::Value;
use tauri_plugin_voice::{
    BootstrapResponse, CancelSpeechRequest, CancelSpeechResponse, DiagnosticEntry,
    DownloadModelRequest, DownloadModelResponse, Engine, EngineHealth, Error, HealthResponse,
    ModelProgressEvent, ModelProgressPhase, ModelProgressState,
    SetupRecordTranscribePipelineRequest, SetupRecordTranscribePipelineResponse,
    StreamSpeechRequest, StreamSpeechResponse, SynthesizeSpeechRequest, SynthesizeSpeechResponse,
    TranscribeRequest, TtsStreamEvent, TtsStreamEventType,
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
        lifecycle_state: tauri_plugin_voice::LifecycleState::Uninitialized,
        reason: "Virtual environment not found".to_string(),
        engines: vec![EngineHealth {
            engine: Engine::Stt,
            lifecycle_state: tauri_plugin_voice::LifecycleState::Uninitialized,
            ready: false,
            required: true,
            diagnostics: vec![],
        }],
        diagnostics: vec![DiagnosticEntry {
            name: "python_venv".to_string(),
            ready: false,
            reason: Some("Virtual environment not found".to_string()),
        }],
    };
    let value = serde_json::to_value(payload).expect("health response should serialize");

    assert_eq!(value["status"], "notReady");
    assert_eq!(value["lifecycleState"], "uninitialized");
    assert_eq!(value["reason"], "Virtual environment not found");
    assert_eq!(value["engines"][0]["engine"], "stt");
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
fn download_model_contract_serializes_engine() {
    // Verify engine-aware model request shape.

    let request = DownloadModelRequest {
        engine: Engine::Stt,
        model_id: "tiny.en".to_string(),
    };
    let request_value = serde_json::to_value(request).expect("download request should serialize");

    assert_eq!(request_value["engine"], "stt");
    assert_eq!(request_value["modelId"], "tiny.en");

    let response = DownloadModelResponse {
        engine: Engine::Stt,
        model_id: "tiny.en".to_string(),
        already_active: false,
    };
    let response_value =
        serde_json::to_value(response).expect("download response should serialize");

    assert_eq!(response_value["engine"], "stt");
    assert_eq!(response_value["modelId"], "tiny.en");
    assert_eq!(response_value["alreadyActive"], false);
}

#[test]
fn model_progress_contract_serializes_engine() {
    // Verify progress events include engine identity.

    let payload = ModelProgressEvent {
        contract_version: "0.1.0".to_string(),
        event_id: "progress-1".to_string(),
        emitted_at_ms: 1,
        engine: Engine::Stt,
        model: "mlx-community/whisper-tiny.en-mlx".to_string(),
        phase: ModelProgressPhase::Preload,
        state: ModelProgressState::Complete,
        percent: None,
        filename: None,
        error: None,
    };
    let value = serde_json::to_value(payload).expect("progress event should serialize");

    assert_eq!(value["engine"], "stt");
    assert_eq!(value["phase"], "preload");
    assert_eq!(value["state"], "complete");
}

#[test]
fn tts_progress_contract_serializes_engine() {
    // Verify TTS progress events use engine identity.

    let payload = ModelProgressEvent {
        contract_version: "0.1.0".to_string(),
        event_id: "progress-tts-1".to_string(),
        emitted_at_ms: 1,
        engine: Engine::Tts,
        model: "mlx-community/Kokoro-82M-bf16".to_string(),
        phase: ModelProgressPhase::Preload,
        state: ModelProgressState::Complete,
        percent: Some(1.0),
        filename: None,
        error: None,
    };
    let value = serde_json::to_value(payload).expect("progress event should serialize");

    assert_eq!(value["engine"], "tts");
    assert_eq!(value["phase"], "preload");
    assert_eq!(value["state"], "complete");
}

#[test]
fn synthesize_speech_request_serializes_camel_case_fields() {
    // Verify TTS request payload shape.

    let payload = SynthesizeSpeechRequest {
        text: "Hello".to_string(),
        model_id: Some("mlx-community/Kokoro-82M-bf16".to_string()),
        voice_id: Some("af_heart".to_string()),
        language_code: Some("a".to_string()),
        speed: Some(1.0),
        output_path: None,
        output_dir: Some("/tmp".to_string()),
        file_name_prefix: Some("speech".to_string()),
    };
    let value = serde_json::to_value(payload).expect("tts request should serialize");

    assert_eq!(value["text"], "Hello");
    assert_eq!(value["modelId"], "mlx-community/Kokoro-82M-bf16");
    assert_eq!(value["voiceId"], "af_heart");
    assert_eq!(value["languageCode"], "a");
    assert_eq!(value["outputDir"], "/tmp");
    assert_eq!(value["fileNamePrefix"], "speech");
}

#[test]
fn synthesize_speech_response_serializes_camel_case_fields() {
    // Verify TTS response payload shape.

    let payload = SynthesizeSpeechResponse {
        audio_path: "/tmp/speech.wav".to_string(),
        model_id: "mlx-community/Kokoro-82M-bf16".to_string(),
        voice_id: "af_heart".to_string(),
        language_code: Some("a".to_string()),
        duration_ms: Some(1000),
        format: "wav".to_string(),
        sample_rate_hz: Some(24000),
        created_at_ms: 1,
    };
    let value = serde_json::to_value(payload).expect("tts response should serialize");

    assert_eq!(value["audioPath"], "/tmp/speech.wav");
    assert_eq!(value["modelId"], "mlx-community/Kokoro-82M-bf16");
    assert_eq!(value["voiceId"], "af_heart");
    assert_eq!(value["languageCode"], "a");
    assert_eq!(value["durationMs"], 1000);
    assert_eq!(value["sampleRateHz"], 24000);
    assert_eq!(value["createdAtMs"], 1);
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
fn stream_speech_request_serializes_camel_case_fields() {
    // Verify streaming request payload shape.

    let payload = StreamSpeechRequest {
        text: "Hello streaming".to_string(),
        model_id: Some("mlx-community/Kokoro-82M-bf16".to_string()),
        voice_id: Some("af_heart".to_string()),
        language_code: Some("a".to_string()),
        speed: Some(1.0),
        chunk_duration_ms: Some(200),
    };
    let value = serde_json::to_value(payload).expect("stream request should serialize");

    assert_eq!(value["text"], "Hello streaming");
    assert_eq!(value["modelId"], "mlx-community/Kokoro-82M-bf16");
    assert_eq!(value["voiceId"], "af_heart");
    assert_eq!(value["languageCode"], "a");
    assert_eq!(value["chunkDurationMs"], 200);
}

#[test]
fn stream_speech_response_serializes_camel_case_fields() {
    // Verify streaming response payload shape.

    let payload = StreamSpeechResponse {
        synthesis_id: "syn-12345".to_string(),
        model_id: "mlx-community/Kokoro-82M-bf16".to_string(),
        voice_id: "af_heart".to_string(),
        language_code: Some("a".to_string()),
        format: "pcm_s16le".to_string(),
        sample_rate_hz: 24000,
        channels: 1,
    };
    let value = serde_json::to_value(payload).expect("stream response should serialize");

    assert_eq!(value["synthesisId"], "syn-12345");
    assert_eq!(value["modelId"], "mlx-community/Kokoro-82M-bf16");
    assert_eq!(value["format"], "pcm_s16le");
    assert_eq!(value["sampleRateHz"], 24000);
    assert_eq!(value["channels"], 1);
}

#[test]
fn tts_stream_event_chunk_serializes_contract_fields() {
    // Verify stream event payload includes type, synthesisId, sequence, and final.

    let event = TtsStreamEvent {
        contract_version: "0.1.0".to_string(),
        event_id: "evt-1".to_string(),
        emitted_at_ms: 1000,
        synthesis_id: "syn-1".to_string(),
        sequence: 3,
        event_type: TtsStreamEventType::Chunk,
        model_id: Some("mlx-community/Kokoro-82M-bf16".to_string()),
        voice_id: Some("af_heart".to_string()),
        language_code: Some("a".to_string()),
        sample_rate_hz: Some(24000),
        channels: Some(1),
        format: Some("pcm_s16le".to_string()),
        audio_base64: Some("AAAA".to_string()),
        duration_ms: Some(200),
        final_chunk: false,
        error: None,
    };
    let value = serde_json::to_value(event).expect("stream event should serialize");

    assert_eq!(value["type"], "chunk");
    assert_eq!(value["synthesisId"], "syn-1");
    assert_eq!(value["sequence"], 3);
    assert_eq!(value["final"], false);
    assert_eq!(value["sampleRateHz"], 24000);
    assert_eq!(value["audioBase64"], "AAAA");
}

#[test]
fn tts_stream_event_complete_serializes_type() {
    // Verify complete event uses "complete" type string.

    let event = TtsStreamEvent {
        contract_version: "0.1.0".to_string(),
        event_id: "evt-2".to_string(),
        emitted_at_ms: 2000,
        synthesis_id: "syn-1".to_string(),
        sequence: 10,
        event_type: TtsStreamEventType::Complete,
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
    let value = serde_json::to_value(event).expect("complete event should serialize");

    assert_eq!(value["type"], "complete");
}

#[test]
fn cancel_speech_request_and_response_serialize_camel_case() {
    // Verify cancel request and response payload shapes.

    let req = CancelSpeechRequest {
        synthesis_id: "syn-99".to_string(),
    };
    let req_value = serde_json::to_value(req).expect("cancel request should serialize");
    assert_eq!(req_value["synthesisId"], "syn-99");

    let res = CancelSpeechResponse {
        synthesis_id: "syn-99".to_string(),
        cancelled: true,
    };
    let res_value = serde_json::to_value(res).expect("cancel response should serialize");
    assert_eq!(res_value["synthesisId"], "syn-99");
    assert_eq!(res_value["cancelled"], true);
}

#[test]
fn file_output_synthesize_speech_unchanged_after_streaming_addition() {
    // Confirm file-output contract remains unchanged.

    let payload = SynthesizeSpeechRequest {
        text: "still works".to_string(),
        model_id: None,
        voice_id: None,
        language_code: None,
        speed: None,
        output_path: None,
        output_dir: Some("/tmp".to_string()),
        file_name_prefix: None,
    };
    let value = serde_json::to_value(payload).expect("synthesize request should serialize");

    assert_eq!(value["text"], "still works");
    assert_eq!(value["outputDir"], "/tmp");
    assert!(value.get("synthesisId").is_none());
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
    use tauri_plugin_voice::{
        AggregateStatus, GetRuntimeStateResponse, PermissionState, Phase, Readiness, RuntimeState,
    };

    let state = RuntimeState {
        phase: Phase::Capturing,
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
    assert_eq!(value["state"]["phase"], "capturing");
    assert_eq!(value["state"]["micPermission"], "granted");
    assert_eq!(value["readiness"]["aggregateStatus"], "ready");
    assert_eq!(value["readiness"]["recordingReady"], true);
    assert_eq!(value["readiness"]["shortcutReady"], true);
}

#[test]
fn setup_pipeline_request_serializes_camel_case_fields() {
    let payload = SetupRecordTranscribePipelineRequest {
        toggle_shortcut: "Command+Shift+R".to_string(),
        recorder_config: None,
        model_id: Some("tiny".to_string()),
        show_final_transcript: Some(true),
    };
    let value = serde_json::to_value(payload).expect("setup request should serialize");

    assert_eq!(value["toggleShortcut"], "Command+Shift+R");
    assert_eq!(value["modelId"], "tiny");
    assert_eq!(value["showFinalTranscript"], true);
}

#[test]
fn setup_pipeline_response_serializes_active_contract() {
    let payload = SetupRecordTranscribePipelineResponse {
        contract_version: "0.1.0".to_string(),
        active: true,
    };
    let value = serde_json::to_value(payload).expect("setup response should serialize");

    assert_eq!(value["contractVersion"], "0.1.0");
    assert_eq!(value["active"], true);
}
