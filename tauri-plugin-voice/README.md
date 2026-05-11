# tauri-plugin-voice

Local voice plugin for Tauri using a bundled `uv` sidecar.

## Scope
- Voice plugin surface with STT and first local TTS command.
- Bootstrap and readiness checks through plugin commands.
- Request/response workflow only.

## Platform Constraints
- Primary target: `aarch64-apple-darwin` (macOS Apple Silicon).
- Runtime expects `uv` sidecar packaging in host Tauri app.
- Python runtime is pinned to `3.12`.

## Canonical Runtime Paths
- `APP_DATA_DIR/python/.venv`
- `APP_DATA_DIR/python/.venv/bin/python`
- `APP_DATA_DIR/python/cache`
- `APP_DATA_DIR/python/tools`
- `APP_DATA_DIR/logs`

## Pinned Dependencies
- Python: `3.12`
- Python packages: `tauri-plugin-voice/requirements/requirements-stt.lock.txt`
- TTS packages: `tauri-plugin-voice/requirements/requirements-tts.lock.txt`
- Rust crate toolchain: from `Cargo.toml` and `Cargo.lock`

## Plugin Commands

### `bootstrap_voice(payload)`
Creates the app-local virtual environment and installs pinned dependencies.

Request:
- `{}` (empty object)

Response:
- `{"status":"ready","details":"Bootstrap complete"}`
- `{"status":"alreadyReady","details":"..."}`

Error codes:
- `bootstrap_failed`
- `dependency_install_failed`
- `not_ready`

### `voice_health(payload)`
Reports STT readiness through the voice lifecycle command.

Request:
- `{}`

Response:
- `{"status":"ready","diagnostics":[...]}`
- `{"status":"notReady","reason":"...","diagnostics":[...]}`

Error codes:
- `not_ready` (for command-level precondition failures)

### `setup_record_transcribe_pipeline(payload)`
Registers a single hotkey-driven record-to-transcribe pipeline coordinated in Rust.

Request:
- `{"toggleShortcut":"Command+Shift+R","recorderConfig":{"outputDir":null,"fileNamePrefix":null},"modelId":"tiny","showFinalTranscript":true}`

Response:
- `{"contractVersion":"0.1.0","active":true}`

Behavior:
- First call wins for process lifetime.
- Repeated setup calls return `invalid_input`.

### `transcribe_file(payload)`
Validates readiness, validates file path, validates model allowlist, then runs transcription.

Request:
- `{"path":"<absolute_audio_path>","modelId":"tiny"}` (`modelId` optional)

Response:
- `{"text":"..."}`

Current behavior:
- The command path is wired and validated.
- Transcription executes by spawning the Python transcriber script.
- Model resolution and load happen during transcription execution.

Error codes:
- `not_ready`
- `invalid_input`
- `generation_failed`

### `synthesize_speech(payload)`
Synthesizes text to a local WAV file through the TTS engine.

Request:
- `{"text":"Hello","modelId":"mlx-community/Kokoro-82M-bf16","voiceId":"af_heart","languageCode":"a","speed":1.0,"outputDir":null,"outputPath":null,"fileNamePrefix":"tts"}`

Response:
- `{"audioPath":"<absolute_wav_path>","modelId":"mlx-community/Kokoro-82M-bf16","voiceId":"af_heart","languageCode":"a","durationMs":1234,"format":"wav","sampleRateHz":24000,"createdAtMs":123456789}`

Current behavior:
- File-output only.
- Default model is `mlx-community/Kokoro-82M-bf16`.
- Default voice is `af_heart`.
- Default language code is `a`.
- Default output directory is `APP_DATA_DIR/tts/output`.
- No direct playback, streaming, voice cloning, or conversation orchestration.

Error codes:
- `not_ready`
- `invalid_input`
- `generation_failed`

## Recorder Ownership
- Recording, keybinding, and overlay are out of scope for this plugin.
- Use `/Users/karthik/merge_conflicts/tauri_recorder_plugin` for recorder features.

## Clean-Machine Bootstrap Flow
1. Build plugin crate:
   - `cargo build`
2. Ensure host app packages the `uv` sidecar as `uv`.
3. Start host app with this plugin installed.
4. Invoke `bootstrap_voice({})`.
5. Invoke `voice_health({})` and confirm `status = ready`.
6. Invoke `transcribe_file({ path, modelId })` with a valid audio file path.
7. Invoke `synthesize_speech({ text })` to generate a local WAV file.

## Smoke Tests
Contract-level smoke tests are included for stable payload shapes:
- bootstrap response tagging
- health response diagnostics shape
- transcribe request serialization
- synthesize speech request/response serialization
- TTS progress event serialization
- typed error serialization codes

Run:
- `cargo test --test smoke_contract`

## Troubleshooting

### `not_ready` with missing venv or python
- Run `bootstrap_voice({})` first.
- Check `APP_DATA_DIR/python/.venv/bin/python` exists.

### `bootstrap_failed`
- Verify `uv` sidecar is bundled and named `uv`.
- Verify app can write to `APP_DATA_DIR`.

### `dependency_install_failed`
- Verify lock file exists:
  - `tauri-plugin-voice/requirements/requirements-stt.lock.txt`
- Verify network access for first-time dependency install.

### `invalid_input` from `transcribe_file`
- Ensure `path` exists and points to a file.
- Ensure `modelId` is in the allowlist:
  - `tiny`, `tiny.en`, `base`, `base.en`, `small`, `small.en`, `medium`, `medium.en`, `large`, `large-v1`, `large-v2`, `large-v3`, `large-v3-turbo`, `turbo`

### `invalid_input` from `synthesize_speech`
- Ensure `text` is non-empty.
- Ensure `modelId` is `mlx-community/Kokoro-82M-bf16`.
- Ensure `voiceId` is a supported Kokoro preset such as `af_heart`.
- Ensure `outputPath` ends with `.wav` and does not already exist.
