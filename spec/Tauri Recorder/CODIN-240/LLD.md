# Low Level Design: Plugin-Driven Recording Integration for Hello World Tauri (CODIN-240)

## 1. Objective
Enable recording of new audio directly from the Hello World Tauri app by using `voice_plugin` command APIs (not browser `MediaRecorder`), then allow the same app flow to transcribe the newly recorded file.

Locked decisions from product input:
- Platform scope: macOS only (Apple Silicon first).
- Recording source: new audio recorded in-app through `voice_plugin`.
- Dependency mode: local path dependency (already in use by host app).

## 2. Scope
In scope:
- Add recording command APIs to `tauri-plugin-stt`.
- Implement desktop recorder lifecycle (start/stop) in plugin runtime.
- Wire Hello World Record buttons to plugin commands.
- Use recorded output file for transcription flow.
- Update permissions/capabilities for new command invocations.

Out of scope:
- Linux/Windows/mobile recording support.
- Multi-session or parallel recording.
- UI redesign beyond wiring and status text.

## 3. Repository and Files
Plugin repo:
- `/Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/src/models.rs`
- `/Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/src/error.rs`
- `/Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/src/commands.rs`
- `/Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/src/lib.rs`
- `/Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/src/desktop.rs`
- `/Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/README.md`
- New file: `/Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/src/recorder_manager.rs`

Host app repo:
- `/Users/karthik/merge_conflicts/scratch/hello-world-tauri/src/main.js`
- `/Users/karthik/merge_conflicts/scratch/hello-world-tauri/src-tauri/capabilities/default.json`
- `/Users/karthik/merge_conflicts/scratch/hello-world-tauri/src-tauri/src/lib.rs` (only if helper command wiring is adjusted)

## 4. Command Contract Additions

### 4.1 start_recording
Command name:
- `plugin:stt|start_recording`

Request payload:
- `outputDir: Option<String>` (absolute path, optional)
- `fileNamePrefix: Option<String>` (optional)

Response payload:
- `sessionId: String`
- `status: "recording"`
- `startedAt: String` (ISO-8601)

Behavior:
- Reject if a session is already active.
- Allocate output target path and persist active session state.

### 4.2 stop_recording
Command name:
- `plugin:stt|stop_recording`

Request payload:
- `sessionId: String`

Response payload:
- `status: "stopped"`
- `path: String` (absolute recorded audio path)
- `durationMs: u64`

Behavior:
- Reject if no recording is active.
- Validate session ID matches active session.
- Stop recorder process and verify output file exists.

## 5. Data Model Changes
In `models.rs`, add:
- `StartRecordingRequest`
- `StartRecordingResponse`
- `StopRecordingRequest`
- `StopRecordingResponse`

Serialization style:
- Keep `#[serde(rename_all = "camelCase")]` and tagged status conventions consistent with existing plugin types.

## 6. Error Model Changes
In `error.rs`, add typed variants and wire serializer codes:
- `RecordingInProgress` -> `recording_in_progress`
- `RecordingNotActive` -> `recording_not_active`
- `RecordingStartFailed` -> `recording_start_failed`
- `RecordingStopFailed` -> `recording_stop_failed`

Retain current common codes for validation/precondition:
- `invalid_input`
- `not_ready`

## 7. Recorder Runtime Design (Desktop)

### 7.1 State Ownership
Create `RecorderManager` (new module) managed in plugin state:
- `active_session: Option<ActiveRecordingSession>`
- Session stores:
  - `session_id`
  - `started_at`
  - `output_path`
  - backend process/handle metadata

Concurrency policy:
- Global single active recording per app instance.
- Guard state with synchronization primitive (`Mutex`/`RwLock`) to prevent race conditions from repeated UI clicks.

### 7.2 Output Path Policy
Default output dir:
- `APP_DATA_DIR/recordings`

Default file naming:
- `recording-<UTC timestamp>.m4a`

Path override rules:
- If `outputDir` is provided, it must be absolute and writable; else return `invalid_input`.

### 7.3 Command Flow
`start_recording` flow:
1. Lock manager state.
2. Fail with `recording_in_progress` if active session exists.
3. Resolve output path and ensure directory exists.
4. Start recorder backend for microphone capture.
5. Save active session and return `StartRecordingResponse`.

`stop_recording` flow:
1. Lock manager state.
2. Fail with `recording_not_active` if no session.
3. Validate session ID.
4. Stop backend gracefully and await output finalization.
5. Validate file is present and non-empty.
6. Compute duration and return `StopRecordingResponse`.
7. Clear active session.

## 8. Host App Integration (Hello World)
In `src/main.js`:
- Remove browser `MediaRecorder` dependency for start/stop actions.
- `Start Recording` button invokes `plugin:stt|start_recording`.
- `Stop Recording` button invokes `plugin:stt|stop_recording`.
- Save returned `path` as `lastRecordedPath`.
- Update status text with live session states and errors.

Transcription behavior:
- `Run Transcription` uses `lastRecordedPath` as primary input.
- If no recording has been captured in current session, return actionable error message instructing user to record first.

## 9. Capability and Permission Updates
Host capability file (`default.json`) must include command permissions for:
- `stt:allow-start-recording`
- `stt:allow-stop-recording`

If permission manifests are generated in plugin:
- Add matching permission descriptors for new commands and update reference docs.

## 10. Testing Plan
Plugin tests:
- Unit/contract tests for payload serialization of new request/response structs.
- Error serialization tests for new recorder error codes.
- State transition tests:
  - start -> stop happy path
  - stop without start
  - double-start rejection

Host manual E2E:
1. Launch app.
2. Bootstrap plugin (`Bootstrap`).
3. Click `Start Recording`, speak, click `Stop Recording`.
4. Click `Run Transcription`.
5. Verify transcribed text corresponds to newly recorded audio.

Negative checks:
- Mic permission denied.
- Invalid session ID on stop.
- No recorded file path when transcribe is clicked.

## 11. Acceptance Criteria
- Record buttons in Hello World call plugin recording commands instead of browser recorder APIs.
- New audio is captured and persisted to a plugin-managed file path.
- Transcription runs on the newly recorded file from current app usage.
- macOS-only behavior is documented; non-macOS behavior is clearly out of scope.
- Typed recorder errors are returned with stable machine-readable codes.
