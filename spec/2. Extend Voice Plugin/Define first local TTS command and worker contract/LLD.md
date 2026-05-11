# Low Level Design: First Local TTS Command and Worker Contract

## 1. Objective

Define the implementation-ready contract for the first local TTS command in `tauri-plugin-voice`.

This LLD is limited to the command and worker contract. It does not implement the Rust command, Python worker, dependency lock file, generated permissions, TypeScript binding, playback, streaming, or consumer integration.

## 2. Scope

### In Scope

- Public command name and payload shape.
- Rust model additions needed for the command contract.
- Python worker protocol.
- Output path policy.
- Error mapping.
- TTS progress event behavior.
- Health and readiness expectations.
- Focused validation plan.

### Out of Scope

- Direct playback.
- Streaming synthesis.
- Voice cloning.
- Conversation orchestration.
- Recorder bridge changes.
- STT behavior changes.
- Broad engine registry abstractions.
- Consumer app migration.
- Automatic cleanup of generated audio files.

## 3. Files To Change During Implementation

### `tauri-plugin-voice/src/models.rs`

Add the TTS request and response models near the existing voice command models.

Models to add:

- `SynthesizeSpeechRequest`
- `SynthesizeSpeechResponse`
- `AudioFormat`, if a string literal is not preferred.

Request fields:

- `text: String`
- `model_id: Option<String>`
- `voice_id: Option<String>`
- `language_code: Option<String>`
- `speed: Option<f32>`
- `output_path: Option<String>`
- `output_dir: Option<String>`
- `file_name_prefix: Option<String>`

Response fields:

- `audio_path: String`
- `model_id: String`
- `voice_id: String`
- `language_code: Option<String>`
- `duration_ms: Option<u64>`
- `format: String`
- `sample_rate_hz: Option<u32>`
- `created_at_ms: u64`

Use `camelCase` serialization to match existing public models.

### `tauri-plugin-voice/src/commands.rs`

Register the `synthesize_speech` command only when implementation starts.

The command should delegate directly to `app.tauri_plugin_voice().synthesize_speech(payload).await`.

Do not add compatibility aliases.

### `tauri-plugin-voice/src/desktop.rs`

Add TTS command handling inside the voice plugin state.

Responsibilities:

- Validate request fields.
- Resolve effective model, voice, language, speed, and output path.
- Ensure TTS readiness or start the TTS worker according to the selected startup policy.
- Send one JSON request line to the TTS worker.
- Read one JSON response line.
- Map worker errors to existing plugin errors.
- Return `SynthesizeSpeechResponse`.
- Clear the TTS worker slot on I/O failure, timeout, or disconnect.

Keep this logic direct. Do not add a generic engine dispatcher.

### `tauri-plugin-voice/scripts/tts_worker.py`

Add a separate persistent TTS worker.

Responsibilities:

- Load the Kokoro model once at startup.
- Emit progress lines during download and preload where supported.
- Emit one ready line after preload.
- Read newline-delimited JSON requests from stdin.
- Generate one WAV file per request.
- Write a compact JSON success or error line to stdout.

Keep the protocol parallel to the existing STT `worker.py`.

### `tauri-plugin-voice/guest-js/index.ts`

Add TypeScript types and `synthesizeSpeech`.

The function should invoke `plugin:voice|synthesize_speech`.

### Permissions and Generated Schemas

When the command is implemented, add `allow-synthesize-speech` and include it in the voice default permission set if the command is intended to be part of the default public API.

Regenerate schemas through the existing Tauri plugin workflow rather than hand-editing generated schema files where possible.

### Documentation and Tests

Update README or usage docs only for the new TTS command.

Add focused smoke tests for serialization and error-shape expectations.

## 4. Constants and Defaults

Use explicit local constants near the TTS implementation:

- Default model ID: `mlx-community/Kokoro-82M-bf16`
- Default voice ID: `af_heart`
- Default speed: `1.0`
- Default format: `wav`
- Default output subdirectory: `tts/output`
- Worker timeout: a conservative synthesis timeout, separate from STT transcription timeout.

If language code is exposed in the first implementation, default it to the Kokoro American English path used by `mlx-audio`.

## 5. Request Validation

Validation order:

1. Trim and check `text`.
2. Enforce maximum text length.
3. Resolve and validate `modelId`.
4. Resolve and validate `voiceId`.
5. Resolve and validate `languageCode`.
6. Resolve and validate `speed`.
7. Reject simultaneous `outputPath` and `outputDir`.
8. Resolve the final WAV output path.
9. Ensure the parent output directory exists or can be created.

Recommended initial limits:

- Maximum text length: 5000 Unicode scalar values.
- Speed range: `0.5` through `2.0`.
- Extension: `.wav` when `outputPath` is provided.

Return invalid-input errors for validation failures.

## 6. Output Path Resolution

If `outputPath` is provided:

- Expand it as a normal filesystem path.
- Require `.wav`.
- Require the parent directory to exist or be creatable.
- Do not overwrite an existing file unless a later requirement explicitly allows it.

If `outputDir` is provided:

- Use that directory.
- Generate a unique WAV filename.

If neither output field is provided:

- Use `APP_DATA_DIR/tts/output`.
- Generate a unique WAV filename.

Generated filename format:

- Prefix from `fileNamePrefix`, or `tts`.
- Timestamp in milliseconds.
- Short request ID.
- `.wav`.

Rust should pass the final absolute path to Python. Python should not choose public output locations.

## 7. Worker Startup Protocol

Spawn the TTS worker with:

- Shared Python binary from `APP_DATA_DIR/python/.venv/bin/python`.
- Script path `scripts/tts_worker.py`.
- `--model` set to the effective TTS model ID.

Expected ready line:

- `status`: `ready`
- `model`: effective model ID
- `voice`: default or loaded voice ID when known

Startup timeout should return not-ready. The TTS worker slot should remain empty after startup failure.

## 8. Worker Request Protocol

Each request line should include:

- `id`: request ID generated by Rust.
- `text`: validated input text.
- `output`: final absolute WAV path.
- `model`: effective model ID.
- `voice`: effective voice ID.
- `language`: effective language code, when used.
- `speed`: effective speed.

The worker should process requests serially.

## 9. Worker Response Protocol

Success response fields:

- `id`
- `audio`
- `model`
- `voice`
- `language`
- `duration_ms`
- `sample_rate_hz`
- `format`
- `error`

The success response must set `error` to null.

Error response fields:

- `id`
- `audio`
- `error`
- `error_code`

Rust should require `audio` to match the requested output path and verify the file exists after success.

## 10. Progress Events

The worker may emit progress lines with:

- `status`
- `phase`
- `state`
- `model`
- `percent`
- `filename`
- `error`

Rust should convert progress lines into `ModelProgressEvent` with:

- `engine = Engine::Tts`
- existing `contractVersion`
- generated `eventId`
- current `emittedAtMs`

Use the existing public channel:

- `tauri-voice-plugin://model-progress`

Progress events should not be forwarded through the worker response channel.

## 11. Error Mapping

Map errors as follows:

- Empty text: invalid input.
- Text over maximum length: invalid input.
- Unsupported model: invalid input.
- Unsupported voice: invalid input.
- Unsupported language code: invalid input.
- Speed outside range: invalid input.
- Invalid or unsafe output path: invalid input.
- Missing Python binary: not ready.
- Missing TTS worker script: generation failed.
- Worker startup timeout: not ready.
- Worker request timeout: generation failed.
- Worker stdin or stdout failure: generation failed.
- Worker-reported synthesis error: generation failed.
- Missing output file after success: generation failed.

On worker I/O failure, timeout, or disconnect, clear the TTS worker slot before returning.

## 12. Health and Readiness

Extend `voice_health` to include TTS diagnostics when implementation starts.

Diagnostics should distinguish:

- Shared Python environment readiness.
- TTS dependency readiness.
- TTS worker readiness.
- TTS active model.
- TTS startup preload state.
- Last TTS startup error.

TTS should not block aggregate voice readiness unless TTS is configured as required.

`synthesize_speech` should gate on TTS readiness. It should not require STT readiness.

## 13. Tests

Add focused tests for:

- `SynthesizeSpeechRequest` camelCase serialization.
- `SynthesizeSpeechResponse` camelCase serialization.
- Default output path policy.
- Rejection of empty text.
- Rejection of unsupported model.
- Rejection of unsupported voice.
- Rejection of invalid speed.
- Rejection of conflicting output fields.
- Worker success response parsing.
- Worker error response mapping.
- TTS progress event serialization with `engine = "tts"`.
- `voice_health` shape with TTS diagnostics.

Keep STT smoke tests unchanged except where shared health shape is intentionally extended.

## 14. Implementation Order

1. Add models and serialization tests.
2. Add output path resolution and validation.
3. Add TTS worker process ownership.
4. Add `tts_worker.py` with ready, progress, request, and response protocol.
5. Add `synthesize_speech` command.
6. Add TypeScript binding.
7. Add permission metadata.
8. Add README or usage documentation.
9. Run Rust tests and any available plugin contract tests.

Do not move on to playback or streaming until this file-output contract is implemented and validated.

## 15. Rollback Plan

If implementation fails, remove only the new TTS command registration, TTS models, TTS worker file, TypeScript binding, permission entry, tests, and docs added for this ticket.

Do not revert the existing voice rename, STT worker, recorder bridge, or shared bootstrap changes.
