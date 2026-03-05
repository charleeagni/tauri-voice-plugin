# High Level Design: Minimal Bootstrap Manager (CODIN-235)

## 1. Objective

Implement the smallest useful bootstrap flow for STT setup:
- Create `APP_DATA_DIR/python/.venv` with Python `3.14`.
- Install pinned dependencies from lock file.
- Return a simple request/response result.

This story does not add transcription behavior.

## 2. Parent Alignment

Parent: `CODIN-233`

Locked constraints used here:
- STT only.
- Sidecar-only execution (`uv`).
- Local-only runtime.
- User-invoked bootstrap command.
- Python version fixed to `3.14`.

## 3. Dependency Lock Location

Lock file location:
- `tauri-plugin-stt/requirements/requirements-stt.lock.txt`

This is the single dependency source for bootstrap install.

## 4. Scope

In scope:
- `bootstrap_stt` command implementation.
- Venv creation under app data.
- Dependency install from lock file.
- Minimal success/failure response.

Out of scope:
- Fingerprint hashing.
- Marker files.
- Runtime dependency drift detection.
- Transcription execution.
- Health/reporting expansion.

## 5. Architecture (Minimal)

Components:
- `bootstrap_stt` command handler.
- `BootstrapManager` orchestration.
- `UvSidecarRunner` for controlled `uv` commands.

Flow:
1. Resolve app data path.
2. Ensure `APP_DATA_DIR/python` exists.
3. Run `uv venv APP_DATA_DIR/python/.venv --python 3.14`.
4. Run `uv pip install --python APP_DATA_DIR/python/.venv/bin/python -r tauri-plugin-stt/requirements/requirements-stt.lock.txt`.
5. Return command result.

Idempotency approach:
- Rely on `uv venv` and `uv pip install` being safe on repeat calls.
- No extra runtime state tracking in this phase.

## 6. Response Contract

Minimal bootstrap response fields:
- `status`: `ok | failed`
- `details`: short message

No python path/version/fingerprint fields in public response.

## 7. Error Boundary

Error classes:
- Sidecar missing/unavailable.
- Venv creation failure.
- Dependency install failure.
- Lock file not found/unreadable.

Errors return stable code + user-safe message.

## 8. Validation Expectations

- First call creates venv and installs dependencies.
- Repeated call succeeds without requiring extra state files.
- Missing lock file returns clear failure.
- Command remains user-invoked, not startup-triggered.
