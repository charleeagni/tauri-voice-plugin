# HLD: CODIN-251 Implement macOS overlay runtime via tauri-nspanel

## Scope
Port the existing macOS overlay runtime from Steno into `tauri-plugin-recorder` and make it reusable for plugin consumers.

Source of truth for porting:
- `/Users/karthik/merge_conflicts/steno/src-tauri/src/overlay_runtime.rs`
- `/Users/karthik/merge_conflicts/steno/src-tauri/src/runtime.rs` (overlay sync hooks)

This is a feature port, not downstream validation.

## Requirements (Confirmed)
- macOS-only overlay integration via `tauri-nspanel`.
- Overlay visibility must be tied to recorder phase transitions.
- Required recorder phases:
  - `recording`
  - `transcribing`
  - `error`
  - `idle`
- Overlay failures must degrade gracefully and never break recording lifecycle.
- Plugin must ship a default overlay indicator UI.
- Consumer must be able to replace default UI with their own JS/React UI.
- Consumer UI must have access to recorder state updates.

## Core Decisions
- Recorder state remains the single source of truth.
- Overlay runtime is a thin observer of recorder state.
- Overlay shell behavior remains in Rust; content rendering is configurable.
- Plugin default behavior is preserved unless consumer explicitly overrides overlay mode.

## Phase-to-Overlay Mapping
- `recording` -> overlay visible with recording state.
- `transcribing` -> overlay visible with transcribing state.
- `idle` -> overlay hidden.
- `error` -> overlay hidden (with error still emitted through existing error channel).

## Consumer Override Contract
- Default mode: plugin-managed overlay indicator UI.
- Consumer mode: plugin disables default indicator rendering and exposes phase/state through plugin state events.
- Plugin guest layer provides a React helper that accepts phase components from the consumer.
- Consumer app passes custom components for each phase instead of wiring event listeners manually.

Contract guarantee:
- Consumer receives the same runtime phase transitions as plugin default UI.
- Switching to consumer mode does not alter recording/transcription logic.

## Failure Handling
- Any overlay operation failure (`show`, `hide`, `position`, render slot) is logged.
- Runtime continues and recorder state transitions remain authoritative.
- No overlay failure may block start/stop recording or completion events.

## Compatibility
- macOS: full overlay runtime.
- non-macOS: explicit no-op behavior for overlay runtime paths.

## Acceptance Criteria
- Overlay deterministically follows recorder state transitions.
- Recorder state includes `transcribing` during post-stop processing window.
- Overlay failures do not regress recording lifecycle behavior.
- Consumer override mode fully replaces default UI while keeping state access.
- Existing state/error/complete channels remain compatible for consumers.
