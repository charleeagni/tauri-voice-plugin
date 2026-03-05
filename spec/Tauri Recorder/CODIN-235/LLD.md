# Low Level Design: Idempotent Bootstrap Manager (CODIN-235)

## 1. Purpose

Define the implementation harness for bootstrap idempotency on top of the `CODIN-234` scaffold, with Python `3.14` fixed as runtime target.

This document defines structure and flow only. It does not include transcription logic.

## 2. Targeted File-Level Changes

Primary Rust files:
- `tauri-plugin-stt/src/commands.rs`
- `tauri-plugin-stt/src/models.rs`
- `tauri-plugin-stt/src/desktop.rs`
- `tauri-plugin-stt/src/error.rs`
- `tauri-plugin-stt/src/lib.rs`

New Rust files to introduce:
- `tauri-plugin-stt/src/bootstrap_manager.rs`
- `tauri-plugin-stt/src/bootstrap_state.rs`
- `tauri-plugin-stt/src/sidecar_uv.rs`

Dependency/metadata files:
- `tauri-plugin-stt/requirements/requirements-stt.lock.txt`

## 3. Data Contracts

Bootstrap request model:
- Optional `force_reinstall` boolean (default false).

Bootstrap response model:
- `status`: `already_ready | venv_created | dependencies_installed | repaired`
- `python_version`: expected `3.14`
- `venv_path`: resolved absolute path.
- `lock_fingerprint`: hash for installed dependency lock.
- `details`: optional human-readable summary.

Marker state model stored in venv:
- `python_version`
- `lock_fingerprint`
- `installed_at`
- `bootstrap_version`

## 4. Internal Module Responsibilities

`bootstrap_manager.rs`
- Orchestrates the full bootstrap flow.
- Applies idempotency decisions.
- Coordinates lock acquisition and release.

`bootstrap_state.rs`
- Reads/writes marker state.
- Computes lock fingerprint.
- Performs atomic marker update.

`sidecar_uv.rs`
- Provides constrained wrappers:
  - create venv with Python `3.14`
  - install pinned dependencies from lock input
- Hides subprocess argument construction from higher layers.

## 5. Command Wiring

`commands.rs`:
- Replace or extend placeholder command surface with `bootstrap_stt`.
- Route to plugin runtime object in desktop layer.

`lib.rs`:
- Register `bootstrap_stt` in invoke handler list.
- Export new modules required for bootstrap.

`desktop.rs`:
- Expose `bootstrap_stt` method that calls bootstrap manager.

## 6. Detailed Execution Flow

Step 1:
- Resolve `APP_DATA_DIR/python` and sidecar binary path.

Step 2:
- Acquire exclusive lock file `APP_DATA_DIR/python/.bootstrap.lock`.

Step 3:
- Validate lock file source exists and is readable.

Step 4:
- Ensure venv at `APP_DATA_DIR/python/.venv` with Python `3.14`.

Step 5:
- Compute dependency lock fingerprint.
- Read marker state if present.

Step 6:
- Decide action:
  - if `force_reinstall` true, install dependencies.
  - else if marker missing or fingerprint mismatch, install dependencies.
  - else skip install.

Step 7:
- On successful install or repair, update marker atomically.

Step 8:
- Return normalized bootstrap response and release lock.

## 7. Error Mapping Strategy

Expected mapping targets:
- Venv creation failures map to bootstrap failure code.
- Dependency command failures map to dependency install failure code.
- Missing sidecar or invalid runtime prerequisites map to not-ready style code.
- State file read/write failures map to bootstrap failure with diagnostics.

Error output must preserve:
- Stable code.
- User-safe message.
- Optional debug detail payload.

## 8. Idempotency and Concurrency Rules

- Bootstrap command is safe to call repeatedly.
- Only one mutating bootstrap operation can run at a time.
- Interrupted runs must be retry-safe.
- Marker update must occur only after successful dependency stage.

## 9. Test Plan

Unit tests:
- Fingerprint generation stability.
- Marker read/write and corruption handling.
- Action decision matrix from marker and lock changes.

Integration tests:
- Clean bootstrap run creates expected venv path.
- Immediate re-run returns `already_ready`.
- Changed lock file triggers install path.
- Concurrent calls serialize through lock behavior.
- Simulated install failure does not write success marker.

## 10. Non-Goals for This LLD

- No model download policy.
- No transcription command implementation.
- No final health report completeness.
- No cross-app integration.

## 11. Acceptance Gate for Implementation Start

- File ownership and boundaries are unambiguous.
- Request/response contract is explicit for bootstrap.
- Idempotency and locking behavior are implementation-ready.
- Python `3.14` lock is reflected in all bootstrap decisions.
