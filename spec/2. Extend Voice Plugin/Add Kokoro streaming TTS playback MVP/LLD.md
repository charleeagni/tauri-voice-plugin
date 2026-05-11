# Low Level Design: Kokoro Streaming TTS Playback MVP

## 1. Objective

Define the implementation-ready plan for explicit Kokoro streaming playback in `tauri-plugin-voice`.

This LLD is limited to streaming TTS command, worker protocol, Tauri events, guest JS playback, cancellation, docs, and focused tests. It does not implement direct native playback, Python speaker playback, STT changes, recorder changes, or streaming-plus-file persistence.

## 2. Scope

### In Scope

- Streaming request and response models.
- TTS stream event model.
- `stream_speech` command.
- `cancel_speech` command.
- TTS worker stream mode.
- Rust event forwarding.
- Guest JS Web Audio playback helper.
- File-output fallback documentation.
- Focused serialization and cancellation tests.

### Out of Scope

- Direct Rust audio output.
- Python speaker playback.
- Concurrent streaming synthesis.
- Streaming and file output in one request.
- Conversational turn orchestration.
- STT behavior changes.
- Recorder behavior changes.
- Generic engine registries or broad event abstractions.

## 3. Files To Change During Implementation

### `tauri-plugin-voice/src/models.rs`

Add streaming request, response, event, and cancellation models near the TTS file-output models.

Models to add:

- `StreamSpeechRequest`
- `StreamSpeechResponse`
- `CancelSpeechRequest`
- `CancelSpeechResponse`
- `TtsStreamEvent`
- `TtsStreamEventType`

Request fields:

- `text: String`
- `model_id: Option<String>`
- `voice_id: Option<String>`
- `language_code: Option<String>`
- `speed: Option<f32>`
- `chunk_duration_ms: Option<u32>`

Response fields:

- `synthesis_id: String`
- `model_id: String`
- `voice_id: String`
- `language_code: Option<String>`
- `format: String`
- `sample_rate_hz: u32`
- `channels: u16`

Cancel request fields:

- `synthesis_id: String`

Cancel response fields:

- `synthesis_id: String`
- `cancelled: bool`

Event fields:

- `contract_version: String`
- `event_id: String`
- `emitted_at_ms: u64`
- `synthesis_id: String`
- `sequence: u64`
- `event_type: TtsStreamEventType`
- `model_id: Option<String>`
- `voice_id: Option<String>`
- `language_code: Option<String>`
- `sample_rate_hz: Option<u32>`
- `channels: Option<u16>`
- `format: Option<String>`
- `audio_base64: Option<String>`
- `duration_ms: Option<u32>`
- `final_chunk: bool`
- `error: Option<String>`

Use `camelCase` serialization. Rename `event_type` to `type` and `final_chunk` to `final`.

### `tauri-plugin-voice/src/commands.rs`

Register two commands:

- `stream_speech`
- `cancel_speech`

Each command should delegate directly to the plugin state:

- `app.tauri_plugin_voice().stream_speech(payload).await`
- `app.tauri_plugin_voice().cancel_speech(payload)`

Do not add compatibility aliases.

### `tauri-plugin-voice/src/desktop.rs`

Add direct TTS streaming state beside the existing TTS worker state.

State to add:

- active synthesis ID
- cancellation flag for the active synthesis
- last emitted sequence

Responsibilities:

- Validate streaming fields using the same model, voice, language, speed, and text rules as file output.
- Resolve `chunkDurationMs` with a default of 200 ms.
- Reject a second active stream with a not-ready error.
- Spawn or reuse the TTS worker for the selected Kokoro model.
- Send a `mode = stream` worker request.
- Forward worker stream lines to `tauri-voice-plugin://tts-stream`.
- Drop chunks for cancelled or stale synthesis IDs.
- Emit `error` on worker failures.
- Clear active stream state on complete, error, or cancel.

Keep this implementation local to TTS handling. Do not introduce a generic streaming dispatcher.

### `tauri-plugin-voice/scripts/tts_worker.py`

Extend the existing worker protocol.

Supported request modes:

- file output when `mode` is absent or `mode = file`
- stream output when `mode = stream`

Streaming request fields:

- `id`
- `mode`
- `text`
- `model`
- `voice`
- `language`
- `speed`
- `chunk_duration_ms`

Streaming output statuses:

- `stream_start`
- `stream_chunk`
- `stream_complete`
- `stream_error`
- `stream_cancelled`

Chunk output fields:

- `id`
- `sequence`
- `sample_rate_hz`
- `channels`
- `format`
- `audio_base64`
- `duration_ms`
- `final`

Cancellation request fields:

- `command`
- `id`

Implementation notes:

- Generate audio through the existing Kokoro model path.
- Convert generated float audio to PCM16 little-endian bytes.
- Split PCM into chunk-sized frame ranges.
- Emit ordered chunks with monotonically increasing sequence numbers.
- Check cancellation between generated result segments and before each chunk emit.
- If generation cannot be interrupted mid-call, suppress later chunks once cancellation is observed.

### `tauri-plugin-voice/guest-js/index.ts`

Add TypeScript models and helpers.

Types to add:

- `StreamSpeechRequest`
- `StreamSpeechResponse`
- `CancelSpeechRequest`
- `CancelSpeechResponse`
- `TtsStreamEvent`
- `TtsStreamEventType`
- `StreamPlaybackHandle`

Functions to add:

- `streamSpeech`
- `cancelSpeech`
- `listenToTtsStream`
- `playStreamedSpeech`

`streamSpeech` invokes `plugin:voice|stream_speech`.

`cancelSpeech` invokes `plugin:voice|cancel_speech`.

`listenToTtsStream` subscribes to `tauri-voice-plugin://tts-stream`.

`playStreamedSpeech` should:

- call `streamSpeech`
- filter events by `synthesisId`
- decode PCM16 base64 chunks
- schedule chunks through Web Audio
- start after the buffering threshold
- expose `cancel`
- clean up listeners on complete, error, or cancellation

### Permissions and Generated Schemas

Add generated permissions for:

- `allow-stream-speech`
- `allow-cancel-speech`

Include both in the default voice permission set if streaming is part of the default public API.

Regenerate schemas through the existing Tauri plugin workflow when available.

### Documentation

Update `README.md` and `USAGE.md` only for TTS streaming.

Document:

- file output remains the fallback
- streaming is explicit
- stream events and metadata
- Web Audio helper behavior
- cancellation behavior
- no direct native playback in this MVP

### Tests

Add focused smoke tests in `tauri-plugin-voice/tests/smoke_contract.rs` for:

- stream request camelCase shape
- stream response camelCase shape
- stream event `type`, `synthesisId`, `sequence`, audio metadata, and `final`
- cancel request and response shape
- existing file-output request and response shape still serialize unchanged

Add worker-level tests only if the existing test harness can run Python without model download. Otherwise keep model-free Rust and TypeScript contract tests in this slice.

## 4. Constants and Defaults

Add local constants near existing TTS constants:

- Stream event channel: `tauri-voice-plugin://tts-stream`
- Default stream format: `pcm_s16le`
- Default stream sample rate: `24000`
- Default stream channels: `1`
- Default chunk duration: `200`
- Minimum chunk duration: `50`
- Maximum chunk duration: `1000`
- Initial guest buffer duration: `400`

Use the existing Kokoro defaults:

- Model: `mlx-community/Kokoro-82M-bf16`
- Voice: `af_heart`
- Language: `a`
- Speed: `1.0`

## 5. Validation Rules

Validation order:

1. Trim and validate `text`.
2. Enforce the existing TTS text length limit.
3. Resolve and validate `modelId`.
4. Resolve and validate `voiceId`.
5. Resolve and validate `languageCode`.
6. Resolve and validate `speed`.
7. Resolve `chunkDurationMs`.
8. Reject if another stream is active.
9. Spawn or reuse the matching TTS worker.

Chunk duration must be between 50 ms and 1000 ms.

Return invalid-input errors for field validation. Return not-ready when a second stream is requested while one is active.

## 6. Event Mapping

Map worker statuses to public event types:

- `stream_start` maps to `start`
- `stream_chunk` maps to `chunk`
- `stream_complete` maps to `complete`
- `stream_error` maps to `error`
- `stream_cancelled` maps to `cancelled`

Rust should generate public `eventId` and `emittedAtMs` values. The worker should provide synthesis ID, sequence, metadata, payload, final flag, and error text.

Rust should not trust worker synthesis IDs blindly. It should forward only the active synthesis ID.

## 7. Cancellation Behavior

`cancel_speech` should be idempotent for the active synthesis.

If the synthesis ID matches the active stream:

- mark cancellation requested
- send the worker cancel line
- emit `cancelled` if the worker is unavailable
- clear active stream state

If the synthesis ID does not match an active stream:

- return `cancelled = false`

Rust must suppress any later worker chunks for cancelled synthesis IDs.

Guest JS must stop all future buffer scheduling and disconnect scheduled sources where possible.

## 8. File-Output Fallback

Do not change `synthesize_speech`, `SynthesizeSpeechRequest`, or `SynthesizeSpeechResponse` except where shared validation reuse is strictly local and readable.

The file-output path remains the fallback for callers that need durable audio, deterministic completion, or compatibility with environments where Web Audio playback is unavailable.

## 9. Validation Plan

Run these checks after implementation:

- `cargo test`
- `npm run build` or the existing guest-js build command
- targeted Python worker protocol tests if a model-free harness exists

Manual validation:

- stream a short Kokoro sentence and hear playback
- cancel during a longer sentence and confirm playback stops
- call `synthesizeSpeech` after streaming and verify WAV output still works
- call `voiceHealth` and verify streaming is not a separate readiness requirement

## 10. Rollback

Streaming can be rolled back by removing:

- `stream_speech`
- `cancel_speech`
- stream models
- stream event forwarding
- guest JS playback helper
- stream docs and permissions

The existing `synthesize_speech` file-output path should remain untouched by rollback.
