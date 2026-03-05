# Low Level Design: Minimal Bootstrap Implementation (CODIN-235)

## 1. Purpose

Define minimal implementation steps for `bootstrap_stt` with Python `3.14` and lock-file install only.

## 2. Files to Touch

Primary files:
- `tauri-plugin-stt/src/commands.rs`
- `tauri-plugin-stt/src/desktop.rs`
- `tauri-plugin-stt/src/lib.rs`
- `tauri-plugin-stt/src/error.rs`
- `tauri-plugin-stt/src/models.rs`

New file:
- `tauri-plugin-stt/src/bootstrap_manager.rs`
- `tauri-plugin-stt/src/sidecar_uv.rs`

Dependency lock file:
- `tauri-plugin-stt/requirements/requirements-stt.lock.txt`

## 3. Command Contract

Request:
- no input payload

Response:
- `status: "ok" | "failed"`
- `details: string`

## 4. Runtime Steps

1. Resolve `APP_DATA_DIR/python`.
2. Ensure directory exists.
3. Execute:
   - `uv venv APP_DATA_DIR/python/.venv --python 3.14`
4. Execute:
   - `uv pip install --python APP_DATA_DIR/python/.venv/bin/python -r tauri-plugin-stt/requirements/requirements-stt.lock.txt`
5. Map result to minimal response.

## 5. Internal Responsibilities

`bootstrap_manager.rs`
- Runs ordered bootstrap steps.
- Converts execution outcomes into response or error.

`sidecar_uv.rs`
- Executes only allowed `uv` subcommands:
  - `venv`
  - `pip install`

No fingerprint state, marker state, or extra lock file handling in this story.

## 6. Error Mapping

Map to stable codes:
- `bootstrap_failed`
- `dependency_install_failed`
- `not_ready` (sidecar/lock-file missing)

Each error includes user-safe message.

## 7. Tests (Minimal)

Unit:
- Command argument generation for both `uv` calls.
- Response mapping for success/failure.

Integration:
- Fresh environment bootstrap succeeds.
- Repeated bootstrap succeeds.
- Missing lock file returns clear error.

## 8. Non-Goals

- No fingerprint hashing.
- No marker persistence.
- No reinstall policy flags.
- No health command expansion.
- No transcription path changes.
