# Low Level Design: Recorder Bridge in Voice Plugin (CODIN-252)

## 1. Purpose

Define file-level implementation plan to expose recorder capabilities through `tauri-plugin-stt` while keeping recorder contract names and channels intact.

## 2. Constraints

- Keep pass-through behavior; do not reimplement recorder core.
- Keep recorder command names unchanged.
- Keep recorder event channel names unchanged.
- Make recorder integration optional at compile-time.
- Keep default recorder destination behavior unless explicitly overridden.
- Overlay shows final transcript text for 2 seconds after transcription completion.

## 3. Files to Update

- `tauri-plugin-stt/Cargo.toml`
- `tauri-plugin-stt/src/lib.rs`
- `tauri-plugin-stt/src/commands.rs`
- `tauri-plugin-stt/src/models.rs`
- `tauri-plugin-stt/src/error.rs`
- `tauri-plugin-stt/src/desktop.rs`
- `tauri-plugin-stt/build.rs`
- `tauri-plugin-stt/guest-js/index.ts`
- `tauri-plugin-stt/README.md`
- `tauri-plugin-stt/permissions/default.toml`
- `tauri-plugin-stt/tests/smoke_contract.rs`

## 4. Dependency and Feature Wiring

### 4.1 Cargo feature and optional dependency

Add optional dependency:
- `tauri-plugin-recorder = { path = "../../tauri_recorder_plugin/tauri-plugin-recorder", optional = true }`

Add feature flags:
- `default = ["recorder-bridge"]`
- `recorder-bridge = ["dep:tauri-plugin-recorder"]`

Rationale:
- Keeps recorder bridge enabled by default for app simplicity.
- Allows opt-out builds.

### 4.2 Setup-time plugin registration

In `src/lib.rs` setup closure:
- If `recorder-bridge` enabled, call `app.handle().plugin(tauri_plugin_recorder::init())`.
- Ensure this runs before any recorder pass-through command execution.

When bridge disabled:
- No recorder plugin registration occurs.

## 5. Command Exposure Plan

Recorder command set to expose through voice plugin invoke handler:
- `initialize_recorder_runtime`
- `start_recording`
- `stop_recording`
- `set_hotkey_bindings`
- `capture_hotkey`
- `get_runtime_state`
- `set_output_destination`
- `get_output_destination`
- `set_overlay_mode`
- `get_overlay_mode`

Implementation behavior:
- Bridge-enabled build: forward each command to recorder runtime path.
- Bridge-disabled build: return typed error code `feature_disabled`.

## 6. Model and Error Additions

### 6.1 Recorder model imports

Reuse recorder payload contracts where practical to avoid schema drift.
If direct reuse is not possible, copy equivalent structs with identical field names and serde casing.

### 6.2 Error contract additions

Add machine-parseable code:
- `feature_disabled`

Usage:
- Returned by recorder-facing commands when bridge feature is disabled.

No renaming of existing STT error codes.

## 7. Guest JS API Plan

`guest-js/index.ts` expands to include recorder methods under same names used by recorder API.

Expected behavior:
- Consumers import from one package (`tauri-plugin-stt`).
- Recorder method names remain unchanged.
- Existing recorder event channels are re-exported unchanged.

Include helper for overlay text display state:
- A lightweight utility to represent final transcript text visibility window (2 seconds).

## 8. Overlay and Transcript Display Behavior

### 8.1 Overlay defaults

- Default mode remains enabled unless explicitly set to disabled.
- Existing recorder overlay mode commands remain the control surface.

### 8.2 Final transcript display

After successful transcription completion:
- Publish overlay text payload using existing overlay pathway.
- Keep text visible for 2 seconds.
- Auto-clear text after timer expiry.

Timing behavior requirements:
- Deterministic duration target: 2000 ms.
- New transcript arrival resets timer and replaces prior text.

## 9. Permissions and Build Artifacts

- Extend `build.rs` command list with recorder command names.
- Regenerate permission metadata to include recorder pass-through commands.
- Update `permissions/default.toml` accordingly.

## 10. Testing Plan

### 10.1 Contract tests

Add/extend smoke tests for:
- Recorder command request/response serialization shapes.
- `feature_disabled` error serialization when bridge is disabled.

### 10.2 Runtime behavior tests

Validate:
- Recorder bridge registration succeeds when feature enabled.
- Recorder pass-through commands execute through voice plugin.
- Output destination default and override behavior.
- Overlay mode default and disable behavior.
- Final transcript overlay text persists for 2 seconds and clears.

## 11. Rollout Notes

- Preserve backward compatibility for existing STT commands.
- Recorder bridge remains additive.
- Any future recorder contract changes should be mirrored via shared-model strategy first.

## 12. Exit Criteria

- Single plugin consumption path is functional for target apps.
- Recorder hotkey + record + transcribe path works through voice plugin surface.
- Overlay default/disable and final text 2-second display behavior are verified.
