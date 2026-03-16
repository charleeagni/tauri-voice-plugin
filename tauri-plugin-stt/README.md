# tauri-plugin-stt

Local STT plugin for Tauri using a bundled `uv` sidecar.

## Scope
- STT-only plugin surface.
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
- Python packages: `tauri-plugin-stt/requirements/requirements-stt.lock.txt`
- Rust crate toolchain: from `Cargo.toml` and `Cargo.lock`

## Plugin Commands

### `bootstrap_stt(payload)`
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

### `stt_health(payload)`
Reports readiness of app data path, sidecar availability, venv, and python binary.

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

## Recorder Ownership
- Recording, keybinding, and overlay are out of scope for this plugin.
- Use `/Users/karthik/merge_conflicts/tauri_recorder_plugin` for recorder features.

## Clean-Machine Bootstrap Flow
1. Build plugin crate:
   - `cargo build`
2. Ensure host app packages the `uv` sidecar as `uv`.
3. Start host app with this plugin installed.
4. Invoke `bootstrap_stt({})`.
5. Invoke `stt_health({})` and confirm `status = ready`.
6. Invoke `transcribe_file({ path, modelId })` with a valid audio file path.

## Smoke Tests
Contract-level smoke tests are included for stable payload shapes:
- bootstrap response tagging
- health response diagnostics shape
- transcribe request serialization
- typed error serialization codes

Run:
- `cargo test --test smoke_contract`

## Troubleshooting

### `not_ready` with missing venv or python
- Run `bootstrap_stt({})` first.
- Check `APP_DATA_DIR/python/.venv/bin/python` exists.

### `bootstrap_failed`
- Verify `uv` sidecar is bundled and named `uv`.
- Verify app can write to `APP_DATA_DIR`.

### `dependency_install_failed`
- Verify lock file exists:
  - `tauri-plugin-stt/requirements/requirements-stt.lock.txt`
- Verify network access for first-time dependency install.

### `invalid_input` from `transcribe_file`
- Ensure `path` exists and points to a file.
- Ensure `modelId` is in the allowlist:
  - `tiny`, `tiny.en`, `base`, `base.en`, `small`, `small.en`, `medium`, `medium.en`, `large`, `large-v1`, `large-v2`, `large-v3`, `large-v3-turbo`, `turbo`
