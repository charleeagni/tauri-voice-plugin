# High Level Design: Typed Error Taxonomy and Readiness Diagnostics (CODIN-238)

## 1. Objective

Stabilize the plugin error contract and health/readiness reporting so callers can:
- Reliably branch on typed error codes.
- Distinguish bootstrap/dependency/model/generation failures.
- Understand non-ready states before calling transcription.

This story hardens contract semantics, not feature scope.

## 2. Parent Alignment

Parent: `CODIN-233`

Locked parent constraints applied:
- STT-only scope.
- Local-only runtime.
- Request/response protocol only.
- Sidecar-controlled execution path.
- Python runtime lock: `3.14`.

## 3. Requirements Assessment

Explicit requirement from `CODIN-238`:
- Implement stable error codes:
  - `bootstrap_failed`
  - `dependency_install_failed`
  - `model_download_failed`
  - `generation_failed`
  - `invalid_input`
  - `not_ready`
- Implement readiness diagnostics.

Inherited context from `CODIN-233` and sibling stories:
- `CODIN-236` already defines command contract shape.
- `CODIN-237` enforces model policy with typed failure behavior.
- This ticket should consolidate and make mapping deterministic across command paths.

## 4. Scope

In scope:
- Define and enforce one-to-one mapping from internal failures to stable error codes.
- Ensure command handlers emit user-safe messages plus optional machine-friendly details.
- Expand health/readiness reporting with explicit diagnostic checks and reasons.
- Keep taxonomy stable across `bootstrap_stt`, `transcribe_file`, and `stt_health`.

Out of scope:
- New command interfaces.
- Streaming status/events.
- Policy redesign outside existing model allowlist behavior.
- Dependency/runtime architecture changes.
- Consumer/UI-specific error presentation.

## 5. Architectural Intent

Error semantics remain centralized in the plugin error layer:
- `error.rs` defines code-stable taxonomy and conversion rules.
- `desktop.rs` maps runtime failures into taxonomy without leaking raw internals.
- `models.rs` carries readiness diagnostic payloads for request/response APIs.
- `commands.rs` remains a transport boundary with no custom mapping logic.

This prevents drift where each command invents its own error semantics.

## 6. Error Taxonomy Boundaries

`invalid_input`
- Invalid/missing path.
- Unsupported request values.
- Malformed user-supplied input.

`not_ready`
- Sidecar/runtime prerequisites missing before execution.
- Venv/python executable unavailable.
- Bootstrap not completed.

`bootstrap_failed`
- Bootstrap orchestration flow failed before dependency install phase classification applies.

`dependency_install_failed`
- Package install/resolution step failed during bootstrap.

`model_download_failed`
- Model acquisition/setup fails when runtime attempts to resolve model assets.

`generation_failed`
- Transcription runtime execution fails after input validation and readiness checks pass.

## 7. Readiness Diagnostics Intent

`stt_health` should return structured diagnostics describing readiness gates, including:
- Sidecar availability check result.
- Venv existence check result.
- Python executable check result.
- Dependency/runtime probe result (non-installing).
- Overall readiness status derived from gate aggregation.

Diagnostics are observational only and must not mutate runtime state.

## 8. Contract Stability Rules

- Error `code` strings are stable and treated as API contract.
- User-facing `message` stays safe and concise.
- Optional `details` may include controlled debug context without exposing secrets.
- Unknown internal failures must still map to one taxonomy code deterministically.

## 9. Validation Expectations

- Every command failure path maps to exactly one stable code.
- Health reports deterministic diagnostics for the same runtime state.
- `not_ready` and readiness diagnostic reasons are consistent with each other.
- Taxonomy behavior remains request/response-only and non-streaming.

## 10. Success Criteria

- Error handling is code-stable and predictable across command handlers.
- Readiness diagnostics are actionable and side-effect free.
- Scope remains constrained to taxonomy and readiness reporting for `CODIN-238`.
