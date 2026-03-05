# High Level Design: Idempotent Bootstrap Manager (CODIN-235)

## 1. Objective

Implement an idempotent bootstrap flow for the STT plugin that:
- Creates `APP_DATA_DIR/python/.venv` using Python `3.14`.
- Installs pinned STT dependencies from lock/constraints input.
- Returns deterministic request/response results without transcription execution.

This design assumes `CODIN-234` scaffold is fully implemented.

## 2. Parent and Constraint Alignment

Parent: `CODIN-233`

Inherited constraints:
- STT-only scope.
- Sidecar-only execution model (`uv`).
- Local-only runtime.
- Request/response UX (no progress stream).
- macOS Apple Silicon target.

Resolved version policy for this story:
- Python runtime is locked to `3.14`.

## 3. Scope

In scope:
- Bootstrap command path for venv creation and dependency installation.
- Idempotency controls for repeated bootstrap requests.
- Safe filesystem coordination for concurrent invocations.
- Bootstrap result contract that reports whether work was performed or skipped.

Out of scope:
- Transcription execution (`transcribe_file`).
- Full health endpoint completion (`stt_health`).
- Model allowlist enforcement (handled later).
- Full typed error taxonomy finalization (handled later).

## 4. Requirements Interpreted for CODIN-235

- Bootstrap must create and manage venv at `APP_DATA_DIR/python/.venv`.
- Dependency install must use pinned lock/constraints source only.
- Re-running bootstrap must not cause incorrect side effects.
- Concurrency must not corrupt venv state.
- Any failure must return stable and actionable error information.

## 5. High-Level Architecture

Primary components:
- Bootstrap command handler.
- Bootstrap manager orchestration layer.
- Sidecar runner for `uv` command execution.
- Filesystem/state layer for lock files and install marker.

Control boundaries:
- Frontend or caller can only invoke bootstrap command.
- Bootstrap manager controls command allowlist and arguments.
- Filesystem writes remain under `APP_DATA_DIR/python`.

Diagram assets:
- `spec/Tauri Recorder/CODIN-235/diagrams/bootstrap_component_overview.png`
- `spec/Tauri Recorder/CODIN-235/diagrams/bootstrap_sequence_idempotent.png`
- `spec/Tauri Recorder/CODIN-235/diagrams/bootstrap_class_design.png`

## 6. Idempotency Strategy

Idempotency is enforced through three mechanisms:

1. Single-flight lock:
- Acquire process lock at `APP_DATA_DIR/python/.bootstrap.lock`.
- Prevent simultaneous bootstrap mutation.

2. State fingerprint:
- Compute dependency fingerprint from lock/constraints source content.
- Track installed state in marker file under venv directory.

3. Conditional execution:
- If venv exists with expected Python `3.14` and fingerprint matches, skip install.
- Otherwise execute only required missing steps and update marker atomically.

## 7. Bootstrap Flow (Conceptual)

1. Resolve app data paths and verify sidecar availability.
2. Acquire bootstrap lock.
3. Ensure `APP_DATA_DIR/python` exists.
4. Ensure `APP_DATA_DIR/python/.venv` exists with Python `3.14`.
5. Compare installed marker fingerprint to current lock fingerprint.
6. Run dependency install only when mismatch or missing marker.
7. Persist marker with Python version and fingerprint.
8. Release lock and return bootstrap status.

Returned statuses:
- `already_ready`: no work needed.
- `venv_created`: venv was created but dependencies already valid.
- `dependencies_installed`: dependency installation performed.
- `repaired`: existing state fixed and made consistent.

## 8. Error Boundaries

Error categories in this story:
- Bootstrap precondition failure (missing sidecar, invalid paths).
- Venv creation failure.
- Dependency installation failure.
- State marker read/write failure.
- Lock acquisition timeout/failure.

Each error response must include:
- Stable machine-readable code.
- User-safe message.
- Optional diagnostic details for logs.

## 9. Security and Safety

- No shell passthrough from user input.
- Only controlled sidecar commands are allowed.
- Paths are normalized and constrained to app-owned directories.
- Bootstrap does not execute arbitrary package sources outside configured lock input.

## 10. Validation Expectations

Minimum validation for this story:
- First run on clean app-data creates venv and installs dependencies.
- Second run without changes returns idempotent skip status.
- Lock/constraints change triggers reinstall path.
- Concurrent bootstrap calls do not race or corrupt state.
- Partial failure leaves retry-safe state.

## 11. Exit Criteria

- Bootstrap manager design is clear, bounded, and idempotent.
- Python `3.14` lock is explicit throughout.
- Lock-based and fingerprint-based controls are documented.
- Implementation can proceed without changing architecture assumptions from `CODIN-234`.
