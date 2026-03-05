# Low Level Design: Error Taxonomy Hardening and Health Diagnostics (CODIN-238)

## 1. Purpose

Define minimal implementation steps to enforce stable typed error mapping and actionable readiness diagnostics for STT command handlers.

## 2. Locked Constraints

- Parent alignment: `CODIN-233`.
- Runtime lock: Python `3.14`.
- Scope lock: STT-only, request/response-only.
- Error lock: keep exact stable code set from ticket.
- Integration lock: no consumer or interface expansion.

## 3. Files to Touch

Primary files:
- `tauri-plugin-stt/src/error.rs`
- `tauri-plugin-stt/src/desktop.rs`
- `tauri-plugin-stt/src/models.rs`

Coordination file:
- `tauri-plugin-stt/src/commands.rs` only if response model signatures change.

No changes to unrelated modules or app integrations.

## 4. Data Contract Adjustments

`models.rs` updates:
- Add explicit health diagnostic structures:
  - readiness summary status.
  - per-check diagnostic entries (`name`, `ready`, `reason`).
- Keep existing command response envelope style.
- Ensure serde camelCase naming consistency.

`error.rs` updates:
- Keep one canonical enum to represent taxonomy variants.
- Guarantee serialized `code` values match ticket strings exactly.
- Keep user-safe message generation centralized.
- Add optional details field handling with controlled content.

## 5. Error Mapping Matrix

Define deterministic mapping points in `desktop.rs`:
- Request validation failures -> `InvalidInput`.
- Missing sidecar/venv/python prerequisites -> `NotReady`.
- Bootstrap orchestration failures before pip stage -> `BootstrapFailed`.
- Dependency installation stage failures -> `DependencyInstallFailed`.
- Model asset resolution/download failures -> `ModelDownloadFailed`.
- Runtime transcription failures after readiness pass -> `GenerationFailed`.

If an internal error can match multiple categories, precedence order is:
1. `InvalidInput`
2. `NotReady`
3. `DependencyInstallFailed`
4. `ModelDownloadFailed`
5. `BootstrapFailed`
6. `GenerationFailed`

This order prevents ambiguous code emission.

## 6. Health Diagnostic Flow

In `desktop.rs`, implement `stt_health` as a read-only gate pipeline:
1. Resolve app data paths.
2. Check sidecar path exists and executable.
3. Check `APP_DATA_DIR/python/.venv` exists.
4. Check `APP_DATA_DIR/python/.venv/bin/python` exists and executable.
5. Run lightweight dependency probe (version/import check only).
6. Build diagnostics list with pass/fail reasons.
7. Derive overall status:
   - ready when all gates pass.
   - not_ready when any gate fails.

No install/download/generation side effects in health.

## 7. Command Path Integration

`bootstrap_stt`:
- Wrap internal failures with taxonomy mapping.
- Distinguish dependency install failures from generic bootstrap failures.

`transcribe_file`:
- Preserve early input validation.
- Enforce readiness gate before runtime generation.
- Map model acquisition errors to `ModelDownloadFailed`.
- Map runtime execution errors to `GenerationFailed`.

`stt_health`:
- Return structured diagnostics regardless of ready/not-ready outcome.
- Avoid throwing unless a non-recoverable internal invariant is violated.

## 8. Boundary Rules

- Do not add or rename error codes.
- Do not add new command endpoints.
- Do not introduce progress streaming.
- Do not alter mobile behavior.
- Do not change model allowlist policy semantics from `CODIN-237`.

## 9. Implementation Sequence

1. Normalize taxonomy serialization and message/details helpers in `error.rs`.
2. Add health diagnostic model structs in `models.rs`.
3. Implement read-only health gate checks and diagnostic assembly in `desktop.rs`.
4. Apply deterministic error mapping in bootstrap/transcription flows.
5. Align command signatures if model changes require it.
6. Compile and run targeted tests for mapping and diagnostics.

## 10. Validation Plan

Unit checks:
- Each failure category serializes to expected stable `code`.
- Health diagnostics include deterministic check names and reasons.
- Precedence rules select a single expected code for ambiguous failures.

Integration checks:
- `bootstrap_stt` emits `dependency_install_failed` for pip-stage failures.
- `transcribe_file` emits `not_ready` for missing runtime prerequisites.
- `transcribe_file` emits `model_download_failed` on model acquisition failure.
- `transcribe_file` emits `generation_failed` for runtime generation failure.
- `stt_health` returns structured diagnostics without side effects.

## 11. Exit Criteria

- All command failures map to stable taxonomy codes.
- Health endpoint reports actionable readiness diagnostics.
- Changes remain tightly scoped to `CODIN-238` contract hardening.
