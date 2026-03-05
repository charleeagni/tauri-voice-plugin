# Low Level Design: STT Command Contract Implementation (CODIN-236)

## 1. Purpose

Define minimal implementation steps to convert placeholder desktop command handlers into concrete command execution paths for:
- `bootstrap_stt`
- `transcribe_file`
- `stt_health`

## 2. Locked Constraints

- Parent alignment: `CODIN-233`.
- Runtime lock: Python `3.14`.
- Scope lock: STT-only, request/response only.
- Error lock: use existing typed error variants; no taxonomy expansion.
- Policy split: leave model allowlist/fixed-output enforcement to `CODIN-237`.

## 3. Files to Touch

Primary files:
- `tauri-plugin-stt/src/desktop.rs`
- `tauri-plugin-stt/src/models.rs`
- `tauri-plugin-stt/src/error.rs`
- `tauri-plugin-stt/src/commands.rs` (only if command signatures require alignment)

Optional coordination points:
- `tauri-plugin-stt/src/lib.rs` (only if new internal module exports are required)

No changes to unrelated modules or consumer integrations.

## 4. Model Contract Updates

Refine request/response structs in `models.rs` to support command contract needs while staying minimal:
- Keep `BootstrapRequest` as an explicit typed empty payload (no fields), not an `any` payload.
- Use `BootstrapResponse` as an enum for successful command outcomes:
  - `Ready { details }`
  - `AlreadyReady { details }`
- Expand `TranscribeRequest` only for required contract inputs.
- Expand `TranscribeResponse` only for required request/response payload.
- Use `HealthResponse` as an enum for readiness state:
  - `Ready`
  - `NotReady { reason }`

All fields use serde camelCase naming, consistent with existing style.

## 5. Desktop Command Flow

`desktop.rs` implementation pattern for each public method:
1. Validate request payload.
2. Resolve runtime preconditions for command.
3. Execute command-specific runtime path.
4. Map execution result into response struct.
5. Convert failures into existing typed `Error` variants.

Method-specific behavior:
- `bootstrap_stt`: invoke bootstrap path and return enum success state.
- `transcribe_file`: reject invalid paths early, enforce readiness gate, execute transcription path, return transcript payload.
- `stt_health`: perform lightweight readiness checks, return enum readiness state without mutation.

Lightweight readiness check definition:
1. Resolve `APP_DATA_DIR` and expected runtime paths.
2. Verify `uv` sidecar path is resolvable and executable.
3. Verify `APP_DATA_DIR/python/.venv` exists.
4. Verify `APP_DATA_DIR/python/.venv/bin/python` exists and is executable.
5. Return `HealthResponse::Ready` or `HealthResponse::NotReady { reason }`.

Checks explicitly excluded from lightweight readiness:
- No dependency install.
- No model download.
- No transcription run.
- No network calls.
- No filesystem mutation.

## 6. Error Mapping Rules

Use existing variants consistently:
- `InvalidInput` for malformed/empty/non-local paths.
- `NotReady` when runtime dependencies are unavailable.
- `BootstrapFailed` when bootstrap orchestration fails.
- `DependencyInstallFailed` only when dependency installation stage fails.
- `GenerationFailed` for transcription runtime failures.
- `ModelDownloadFailed` only when model fetch/setup path explicitly fails.

Do not add, rename, or remove error variants in this story.

## 7. Boundary Rules

- Do not implement model allowlist policy here.
- Do not redesign response enums beyond the locked variants in this LLD.
- Do not introduce progress streaming/event channels.
- Do not modify mobile plugin path.
- Do not alter packaging, sidecar declaration, or dependency lock ownership.

## 8. Implementation Sequence

1. Finalize command request/response structs in `models.rs`.
2. Implement `desktop.rs` command methods with validation and typed error mapping.
3. Align `commands.rs` signatures if model contract changed.
4. Verify plugin compiles and commands register unchanged in `lib.rs`.
5. Run targeted checks for command success/failure paths.

## 9. Validation Plan

Unit-level checks:
- Input validation behavior for `transcribe_file`.
- Error mapping behavior for each command failure class.
- Enum variant mapping for successful command execution.
- Enum readiness mapping for non-mutating health checks.

Integration-level checks:
- `bootstrap_stt` returns deterministic enum success states.
- `stt_health` reports readiness safely across repeated calls using enum states.
- `transcribe_file` returns transcript payload on valid input and typed errors on invalid/not-ready states.

## 10. Exit Criteria

- Placeholder desktop command responses are replaced by real execution flow.
- Contract is deterministic and request/response-only.
- Error handling uses existing typed variants without taxonomy expansion.
- Changes are limited to the command contract scope for `CODIN-236`.
