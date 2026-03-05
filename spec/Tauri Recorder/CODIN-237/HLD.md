# High Level Design: Model Allowlist and Fixed-Output Policy (CODIN-237)

## 1. Objective

Add policy enforcement to STT transcription requests so that:
- Only allowlisted model IDs are accepted.
- Output behavior remains fixed for the POC contract.
- Invalid model requests fail with typed errors.

This story is policy enforcement only, not runtime redesign.

## 2. Parent Alignment

Parent: `CODIN-233`

Locked parent constraints applied:
- STT-only scope.
- Local-only runtime.
- Request/response protocol only.
- Sidecar-controlled execution path.
- Python runtime lock: `3.14`.

## 3. Requirements Assessment

Explicit requirement from `CODIN-237`:
- Restrict model IDs via allowlist.
- Enforce fixed-output defaults for POC.
- Reject invalid model requests with typed errors.

Inherited context from `CODIN-233` and sibling stories:
- Typed error surface already exists (`invalid_input`, `not_ready`, `generation_failed`, etc.).
- Command contract work remains request/response and non-streaming.
- No expansion into smoke tests/docs (`CODIN-239`) or error taxonomy redesign (`CODIN-238`).

## 4. Scope

In scope:
- Add model allowlist policy at transcription boundary.
- Resolve requested model ID against a fixed allowlist.
- Apply fixed output policy defaults for transcription response behavior.
- Return typed errors for policy violations.

Out of scope:
- Introducing dynamic model discovery or remote catalog fetch.
- Adding new command interfaces.
- Expanding error taxonomy beyond existing typed variants.
- Changing bootstrap/runtime installation flow.
- Consumer integration or UI changes.

## 5. Architectural Intent

Policy checks are enforced in the desktop transcription execution path:
- Request model selection is validated before runtime execution.
- Policy remains local to plugin runtime, not delegated to callers.
- Output format remains deterministic and bounded to the existing response contract.

This keeps policy centralized and prevents caller-side bypass.

## 6. Policy Boundaries

Model allowlist policy:
- Allowlist equals the full MLX Whisper-supported model set for the pinned dependency version.
- A finite set of allowed model IDs is defined in plugin runtime and kept in sync with lock updates.
- If `model_id` is absent, fixed default model `tiny` is used.
- If `model_id` is present but not allowlisted, request fails fast with typed error.

Fixed-output policy:
- Response shape remains stable for POC and does not vary by request.
- Disallowed output customization inputs are ignored or rejected at boundary.
- No streaming or alternative output modes are introduced.

## 7. Error Contract Usage

Policy violations map to existing typed errors:
- `invalid_input` for unsupported or malformed model requests.
- `not_ready` for runtime unavailability (unchanged behavior).
- `generation_failed` for runtime execution failures after policy passes.

No new error codes are added in this story.

## 8. Validation Expectations

- Allowlisted model requests pass policy checks.
- Missing model request resolves to fixed default `tiny` model.
- Non-allowlisted model requests fail deterministically with typed error.
- Output contract remains unchanged across valid model choices.
- Request/response behavior remains deterministic and side-effect bounded.

## 9. Success Criteria

- Model selection is no longer unconstrained.
- Fixed-output policy is consistently enforced.
- Invalid model requests are rejected with typed, stable errors.
- Scope remains tightly limited to `CODIN-237` policy enforcement.
