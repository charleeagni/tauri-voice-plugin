# High Level Design: STT Command Execution Contract (CODIN-236)

## 1. Objective

Implement the command execution contract for three STT plugin commands:
- `bootstrap_stt`
- `transcribe_file`
- `stt_health`

This story focuses on desktop command orchestration and response shaping.

## 2. Parent Alignment

Parent: `CODIN-233`

Locked constraints used here:
- STT-only scope.
- Request/response protocol only.
- Local-only runtime.
- Sidecar-controlled execution model.
- Python runtime lock: `3.14`.

## 3. Scope

In scope:
- Wire desktop command handlers to concrete command execution paths.
- Standardize request/response behavior for the three command APIs.
- Validate command inputs at plugin boundary.
- Return stable, user-safe errors using existing typed error variants.

Out of scope:
- Model allowlist and fixed-output policy (`CODIN-237`).
- Expanding or redesigning error taxonomy (`CODIN-238`).
- Smoke tests and documentation expansion (`CODIN-239`).
- Streaming progress/events.
- TTS or cloud fallback.

## 4. Architectural Intent

The plugin keeps a thin command API layer and pushes behavior into desktop runtime methods:
- Command layer (`commands.rs`) remains transport-oriented.
- Desktop runtime (`desktop.rs`) owns execution flow selection.
- Models (`models.rs`) define stable payload contracts.
- Error mapping (`error.rs`) stays code-stable and message-safe.

The flow is intentionally simple: validate input, call underlying runtime operation, map to response, return typed failure when needed.

## 5. Command Behavior Boundaries

`bootstrap_stt`
- Accepts no functional input.
- Triggers bootstrap execution path.
- Returns success/failure status and details.
- Uses existing bootstrap-related error variants only.

`transcribe_file`
- Accepts local audio file path and optional command parameters.
- Validates path and request shape before execution.
- Returns transcript payload and minimal metadata defined by model contract.
- Fails with `invalid_input`, `not_ready`, or `generation_failed` as appropriate.

`stt_health`
- Returns readiness snapshot for command eligibility.
- Must be safe to call repeatedly.
- Reports `not_ready` state without side effects.

## 6. Error Contract Usage

This story uses existing typed error variants already defined in plugin scope:
- `bootstrap_failed`
- `dependency_install_failed`
- `model_download_failed`
- `generation_failed`
- `invalid_input`
- `not_ready`

No new error codes are introduced in this story.

## 7. Dependency Expectations

`CODIN-236` depends on scaffold and bootstrap foundations:
- Scaffold command surfaces from `CODIN-234`.
- Bootstrap execution capability from `CODIN-235`.

If bootstrap internals are incomplete, `bootstrap_stt` and readiness/transcription paths should fail gracefully with existing typed errors.

## 8. Validation Expectations

- Each command returns deterministic request/response payloads.
- Invalid transcribe inputs fail fast with `invalid_input`.
- Not-ready runtime states return `not_ready` without panics.
- Repeated health calls are side-effect free.
- Command behavior remains request/response only.

## 9. Success Criteria

- Desktop plugin commands are no longer placeholder responses.
- API contract is consistent with parent STT POC constraints.
- Scope remains tightly bounded to command execution contract wiring.
