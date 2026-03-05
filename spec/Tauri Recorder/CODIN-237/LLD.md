# Low Level Design: Model Allowlist and Fixed-Output Enforcement (CODIN-237)

## 1. Purpose

Define minimal implementation steps to enforce model allowlist policy and fixed-output defaults in `transcribe_file`.

## 2. Locked Constraints

- Parent alignment: `CODIN-233`.
- Runtime lock: Python `3.14`.
- Scope lock: STT-only, request/response-only.
- Policy lock: allowlisted models only, fixed output behavior.
- Error lock: reuse existing typed error variants.

## 3. Files to Touch

Primary files:
- `tauri-plugin-stt/src/models.rs`
- `tauri-plugin-stt/src/desktop.rs`

Potential coordination file:
- `tauri-plugin-stt/src/commands.rs` only if request struct changes require signature alignment.

No changes to bootstrap manager, sidecar execution, or unrelated modules.

## 4. Data Contract Adjustments

`TranscribeRequest`:
- Add optional `model_id` field.
- Keep existing `path` field unchanged.

`TranscribeResponse`:
- Keep existing response structure unchanged for fixed-output policy.

Rationale:
- Policy must validate caller intent while preserving current response contract.

## 5. Policy Source of Truth

In `desktop.rs`, define:
- One default model ID constant set to `tiny`.
- One static allowlist collection containing all MLX Whisper-supported model IDs for the pinned dependency version.

Rules:
- If `model_id` is `None`, use default.
- If provided, value must exist in allowlist.
- Comparison is exact-match and case-sensitive.
- Allowlist updates are coupled with lock-file version updates.

## 6. Transcription Policy Flow

Inside `transcribe_file` desktop handler:
1. Validate `path` is non-empty and usable.
2. Resolve effective model ID (`requested` or default).
3. Validate effective model ID against allowlist.
4. If invalid, return `Error::InvalidInput` with clear message.
5. Continue with existing runtime readiness/transcription path.
6. Return fixed output response shape.

Policy validation runs before runtime execution.

## 7. Fixed-Output Enforcement

For this story, fixed-output policy means:
- No caller-controlled output format knobs are introduced.
- Response remains deterministic and identical in schema for all valid models.
- Any future output customization requests are out of scope for this ticket.

## 8. Error Mapping Rules

- Invalid/unsupported model ID: `Error::InvalidInput`.
- Missing runtime prerequisites: `Error::NotReady`.
- Runtime transcription failure after valid policy checks: `Error::GenerationFailed`.

No new error variants are introduced.

## 9. Boundary Rules

- Do not add model catalog fetch or network model validation.
- Do not alter bootstrap and dependency install logic.
- Do not add new public commands or consumer APIs.
- Do not change mobile path behavior.
- Do not modify output payload schema.

## 10. Implementation Sequence

1. Update `TranscribeRequest` with optional `model_id`.
2. Add default model and allowlist constants in `desktop.rs`.
3. Add pre-execution policy validation logic in `transcribe_file`.
4. Ensure invalid model requests return typed errors.
5. Verify no command signature drift in `commands.rs`.
6. Compile and run targeted checks for policy pass/fail paths.

## 11. Validation Plan

Unit checks:
- Missing `model_id` resolves to default `tiny`.
- Allowlisted model ID passes validation.
- Non-allowlisted model ID fails with `InvalidInput`.
- Response schema remains unchanged.

Integration checks:
- `transcribe_file` rejects invalid model before runtime execution.
- Valid model path preserves existing request/response behavior.
- Error mapping remains stable and typed.

## 12. Exit Criteria

- Allowlist enforcement is active for `transcribe_file`.
- Fixed-output behavior is preserved for POC.
- Invalid model requests fail deterministically with typed errors.
- Changes remain scoped to `CODIN-237` policy concerns only.
