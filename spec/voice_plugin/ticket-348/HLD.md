# HLD: Control Recorder Overlay Hide Behavior for STT

## 1. Overview
The goal is to finalize the recorder/STT overlay handoff. Currently, stopping the recording unconditionally hides the overlay. We need to allow the STT plugin or the consuming app to keep the overlay visible during the STT transcribing phase (preventing UI flicker) by configuring the recorder, while still providing an explicit API to hide the overlay later.

## 2. Configuration Changes
Add a configuration field to the recorder plugin:
- `hide_overlay_on_recording_stop: bool` (default: `true`)
When `true`, stopping the recording behaves as it currently does (hiding the overlay).
When `false`, stopping the recording completes the capture but leaves the native overlay visible.

## 3. API Additions
Expose a new explicit command in the recorder:
- **Rust Command:** `hide_overlay`
- **JS Binding:** Export a corresponding `hideOverlay()` function if JS exposure is required.
This allows the app or STT bridge to manually dismiss the overlay once the STT transcribing phase (or final transcript display delay) is complete.

## 4. STT Integration Flow
1. App configures the STT/recorder bridge with `hide_overlay_on_recording_stop = false`.
2. Recorder completes the WAV capture and stops without hiding the overlay.
3. The STT bridge transitions state to `Transcribing`, seamlessly updating the visible overlay.
4. After transcription finishes (and any read-delay), the STT bridge or App calls the new `hideOverlay()` API.

## 5. Affected Components
- **Recorder Models:** Add `hide_overlay_on_recording_stop` to the configuration/state structs.
- **Recorder Desktop/Runtime:** Respect the new flag during the stop transition. Implement explicit hide logic.
- **Recorder Commands:** Expose the `hide_overlay` command.
- **Recorder JS bindings & Permissions:** Wire up `hideOverlay()` and autogenerate `.toml` permission entries.
- **STT/App Integration:** Invoke the new hide API at the end of the transcription pipeline in the STT bridge/app layer.