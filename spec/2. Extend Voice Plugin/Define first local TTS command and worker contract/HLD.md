# High Level Design: First Local TTS Command and Worker Contract

## 1. Objective

Define the first local text-to-speech contract for `tauri-plugin-voice` after the approved voice plugin HLD.

This design selects a minimal file-output TTS API, defines the request and response shape, describes worker responsibilities, and records the health, progress, model, and error behavior needed before implementation.

## 2. Approved Context

The parent voice plugin planning has already approved these constraints:

- The plugin public surface is `tauri-plugin-voice`.
- Commands use the `plugin:voice|...` namespace.
- Permissions use the `voice:*` namespace.
- STT and TTS are separate engines under shared voice infrastructure.
- STT and TTS use separate persistent workers.
- The shared Python environment remains `APP_DATA_DIR/python/.venv` unless dependency conflicts are proven.
- TTS v1 uses a file-output contract.
- TTS v1 does not stream audio, play audio directly, clone voices, or orchestrate conversations.
- The first local TTS target is `mlx-audio` with `mlx-community/Kokoro-82M-bf16`.
- The default Kokoro voice is `af_heart`.
- The first output format is WAV.

## 3. Contract Decision

The first TTS command is `synthesize_speech`.

The command returns a generated audio file path. It does not play audio, stream chunks, return raw bytes, or manage conversation state.

This keeps the first API stable and easy to validate:

- Callers get a durable local artifact they can inspect, play, attach, or delete.
- The plugin avoids owning playback lifecycle and output device behavior.
- The worker protocol stays newline-delimited JSON like the STT worker.
- Later playback or streaming work can build on the same synthesis result without changing the minimal contract.

## 4. Public Command Contract

### 4.1 Command Name

Register `synthesize_speech` under `plugin:voice|synthesize_speech`.

Expose the TypeScript function as `synthesizeSpeech`.

### 4.2 Request Model

The request should include:

- `text`: Required synthesis input.
- `modelId`: Optional TTS model ID.
- `voiceId`: Optional Kokoro voice preset.
- `languageCode`: Optional Kokoro language code.
- `speed`: Optional speech speed multiplier.
- `outputPath`: Optional exact WAV output file path.
- `outputDir`: Optional output directory when `outputPath` is omitted.
- `fileNamePrefix`: Optional generated file prefix when `outputPath` is omitted.

Validation rules:

- `text` must be non-empty after trimming.
- `text` should have a documented maximum length to avoid unbounded worker requests.
- `modelId` defaults to `mlx-community/Kokoro-82M-bf16`.
- `voiceId` defaults to `af_heart`.
- `languageCode` defaults to the Kokoro American English path used by the selected runtime.
- `speed` defaults to `1.0`.
- `speed` must stay inside a conservative supported range.
- `outputPath` and `outputDir` are mutually exclusive.
- `outputPath` must resolve to a writable WAV file path.
- `outputDir` must resolve to a writable directory.
- When neither output field is provided, write under the app data TTS output directory.

### 4.3 Response Model

The response should include:

- `audioPath`: Absolute path to the generated WAV file.
- `modelId`: Effective model ID.
- `voiceId`: Effective voice ID.
- `languageCode`: Effective language code, when used.
- `durationMs`: Audio duration when the worker can determine it.
- `format`: `wav`.
- `sampleRateHz`: Sample rate when available.
- `createdAtMs`: Unix timestamp in milliseconds.

`audioPath`, `modelId`, `voiceId`, `format`, and `createdAtMs` are required. Runtime-derived metadata can be optional.

## 5. Output Policy

The default output directory is under app data, using a TTS-specific subdirectory:

- `APP_DATA_DIR/tts/output`

Generated filenames should be deterministic enough to debug and unique enough to avoid collisions:

- Prefix from `fileNamePrefix`, or `tts`.
- Current timestamp in milliseconds.
- A short request identifier.
- `.wav` extension.

The plugin should create the output directory if it does not exist.

The plugin should not delete generated files automatically in v1. Cleanup policy should be a separate task.

The worker should write only to the path selected and validated by Rust. Rust owns path policy; Python owns synthesis.

## 6. Model and Voice Selection

The first allowlisted TTS model is:

- `mlx-community/Kokoro-82M-bf16`

The default voice is:

- `af_heart`

The first implementation should reject unsupported models and voices with typed invalid-input errors.

Do not add a broad model registry. A small explicit allowlist is easier to read and matches the current STT model policy style.

## 7. Worker Protocol

The TTS worker should be a persistent Python process.

Startup arguments:

- `--model <model_id>`

Ready line:

- status `ready`
- model ID
- default voice ID when known

Request line fields:

- `id`
- `text`
- `output`
- `model`
- `voice`
- `language`
- `speed`

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

Error response fields:

- `id`
- `audio`
- `error`
- `error_code`

The worker should process one synthesis request at a time in v1. Concurrent synthesis can be handled later if needed.

## 8. Progress Events

TTS should reuse the existing public model progress event shape with `engine = "tts"`.

The public channel remains:

- `tauri-voice-plugin://model-progress`

The event includes:

- `contractVersion`
- `eventId`
- `emittedAtMs`
- `engine`
- `model`
- `phase`
- `state`
- `percent`
- `filename`
- `error`

Allowed TTS phases for v1:

- `download`
- `preload`

Allowed states:

- `start`
- `in_progress`
- `complete`
- `failed`

The worker may emit partial progress lines. Rust should publish the final public event shape so STT and TTS stay consistent.

## 9. Error Behavior

Use existing plugin error categories where possible:

- Invalid input for empty text, unsupported model, unsupported voice, invalid speed, or invalid output paths.
- Not ready when TTS dependencies, Python, sidecar, or worker readiness are missing.
- Generation failed when synthesis fails after a valid request reaches the worker.
- Feature disabled if TTS is compiled or configured off.

Worker I/O failure should clear the TTS worker slot so health reports degraded readiness.

Worker timeout should return generation failed and clear the worker slot.

Empty or missing output should return generation failed.

## 10. Health and Readiness

`voice_health` remains the canonical health command.

TTS readiness should report:

- TTS lifecycle state.
- TTS worker readiness.
- TTS model ID when loaded.
- Shared Python environment readiness.
- Dependency readiness when implemented.
- Startup preload status when TTS preload is enabled.

TTS should not block aggregate voice readiness unless TTS is configured as required or a TTS command is being executed.

`synthesize_speech` must check TTS readiness, not STT readiness and not recorder readiness.

## 11. Bootstrap Behavior

`bootstrap_voice` may prepare shared Python infrastructure. It should initialize TTS only when TTS is enabled by configuration or when a later implementation task explicitly wires TTS preload.

The first TTS command may lazily start the TTS worker if the implementation plan chooses lazy startup. If lazy startup is selected, the command must surface model download and preload progress through the same progress channel before returning.

## 12. Boundaries

This contract does not add:

- Direct audio playback.
- Streaming audio chunks.
- Raw audio bytes in command responses.
- Voice cloning.
- Conversational turn orchestration.
- Recorder bridge changes.
- STT behavior changes.
- External consumer migrations.

## 13. Validation Expectations

Implementation should validate:

- Request and response serialization shapes.
- Invalid text, model, voice, speed, and output path errors.
- Default app-data output path generation.
- Worker ready, success, error, timeout, and disconnect behavior.
- TTS progress events include `engine = "tts"`.
- `voice_health` reports TTS readiness without breaking STT readiness.
- Existing STT smoke contracts remain unchanged.
