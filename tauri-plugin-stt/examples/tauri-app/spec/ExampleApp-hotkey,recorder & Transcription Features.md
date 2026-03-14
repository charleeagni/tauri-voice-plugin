# Example App: Hotkey, Recorder & Transcription Features

Add three features to the example Svelte app ([examples/tauri-app/src/App.svelte](file:///Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/examples/tauri-app/src/App.svelte)). All necessary backend commands already exist in the plugin's JS API ([guest-js/index.ts](file:///Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/guest-js/index.ts)) — this is a **frontend-only change**.

## Proposed Changes

### Example App UI

#### [MODIFY] [App.svelte](file:///Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/examples/tauri-app/src/App.svelte)

Split the UI into three labeled sections. All calls go through the already-exported functions from `tauri-plugin-tauri-plugin-stt-api`.

**Section 1 — Hotkey Registration**
- A "Capture Hotkey" button that calls [captureHotkey(20000)](file:///Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/guest-js/index.ts#268-279).
- Displays the returned `shortcut` string (e.g. `"Alt+H"`) below the button as the registered hotkey.
- When the hotkey is subsequently pressed, the recorder plugin fires a `tauri-audio-plugin://state` event; the app subscribes via `listen(CHANNELS.STATE, ...)` on mount and shows the received phase change in the log so the user can see the hotkey was read.

**Section 2 — Recorder Button**
- A single toggle button labelled **"Start Recording"** / **"Stop Recording"**.
- On first press: calls [initializeRecorderRuntime()](file:///Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/guest-js/index.ts#213-221) (once, guarded by a flag), then [startRecording()](file:///Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/guest-js/index.ts#222-238).
- On second press: calls [stopRecording(sessionId)](file:///Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/guest-js/index.ts#239-250) and extracts `result.recordingWavPath` from the response, displaying it as a clickable path in the UI.

**Section 3 — Transcription**
- A "Transcribe Last Recording" button (enabled only when a `lastRecordingPath` is known).
- Calls [transcribeFile({ path: lastRecordingPath })](file:///Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/guest-js/index.ts#185-196).
- Renders the returned [text](file:///Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/guest-js/index.ts#388-391) in a `<textarea>` (read-only) below the button.

The existing debug log console (`<pre>`) remains at the bottom for all API responses.

## Verification Plan

### Manual Verification

1. Build and run the example app:
   ```
   cd examples/tauri-app && npx tauri dev
   ```
2. **Hotkey**: Click "Capture Hotkey", press a key combination (e.g. `Alt+H`), verify the shortcut string appears in the UI.
3. **Recorder**: Click "Start Recording", wait a few seconds, click "Stop Recording", verify the WAV file path is shown.
4. **Transcription**: After stopping, click "Transcribe Last Recording", verify text appears in the textarea.
