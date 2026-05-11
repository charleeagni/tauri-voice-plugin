## Record→Transcribe Single-Setup Pipeline (Rust-Owned)

### Summary
Introduce a new STT plugin setup API that target apps call once to enable a full hotkey-driven timeline: record start, record stop, transcribe, final transcript display, and overlay lifecycle.  
This is additive only: existing commands remain for partial workflows, and the example app stays unchanged.

### Public API Changes
- Add new command and guest-js binding:
  - `setup_record_transcribe_pipeline(payload)`
- `payload`:
  - `toggleShortcut: string` (required)
  - `recorderConfig?: { outputDir?: string | null; fileNamePrefix?: string | null }`
  - `modelId?: string`
  - `showFinalTranscript?: boolean` (default `true`)
- Response:
  - `SetupRecordTranscribePipelineResponse { contractVersion, active: true }`
- Behavior contract:
  - First call wins for app lifetime.
  - Subsequent calls return stable error code/message (already-configured pipeline).
- Permissions/build:
  - Register command in plugin invoke handler/build command list and permissions manifests.

### Implementation Changes
- Add a small pipeline runtime state inside STT plugin desktop layer:
  - `configured_once` guard
  - retained setup config (`modelId`, `showFinalTranscript`)
  - listener handle for recorder `complete` event
- On setup:
  - Validate recorder bridge and runtime readiness.
  - Call recorder bridge to initialize runtime and bind toggle hotkey (alternate path from plain recorder-only binding).
  - Attach one `AppHandle.listen_any("tauri-audio-plugin://complete", ...)` listener.
- On each recorder `complete` event:
  - Move overlay/state to transcribing via existing recorder pathway (no new overlay channel).
  - Call STT transcription from Rust using recorded `recordingWavPath`.
  - If `showFinalTranscript=true`, emit transcript via existing `tauri-audio-plugin://live` payload and keep visible for 3 seconds.
  - Return overlay/state to idle/hidden after display window.
  - On failures, emit existing typed error event and restore idle state.
- Keep all existing APIs untouched (`start_recording`, `stop_recording`, `transcribe_file`, etc.).

### Test Plan
- Contract tests:
  - New setup payload/response serialization and casing.
  - Repeat setup call returns deterministic error contract.
- Runtime behavior tests (unit/integration):
  - Setup registers listener exactly once.
  - Synthetic `complete` event triggers transcription path.
  - `showFinalTranscript=false` suppresses transcript display emission.
  - 3-second display path ends in idle overlay state.
  - Error during transcription emits typed error and recovers state.
- Regression:
  - Existing smoke contract tests remain green.

### Assumptions
- Example app is intentionally unchanged.
- Hotkey capture UI remains separate; this API consumes a known shortcut string.
- Existing recorder overlay mechanism is the only overlay transport used.
- `showFinalTranscript` default is `true`, with fixed 3-second visibility.
- This v1 supports only one configured pipeline instance per app session.
