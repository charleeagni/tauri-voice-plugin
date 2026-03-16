# Tauri Voice Plugin (STT) Usage Guide

This plugin (`tauri-plugin-stt`) provides both standalone Speech-to-Text (STT) functionality and acts as a bridge for the `tauri-plugin-recorder`. By using the STT plugin, target applications can natively access both transcription models and recording/hotkey features without having to install or configure the recorder plugin separately.

## Integration Steps

To consume the plugin in a target Tauri app, you need to add both the Rust backend and the JavaScript/TypeScript frontend configuration.

### 1. Add the Rust Dependency
In your target Tauri app's `src-tauri/Cargo.toml`, add the `tauri-plugin-stt` dependency. The recorder bridge is enabled by default.

```toml
[dependencies]
tauri-plugin-stt = { path = "path/to/voice_plugin/tauri-plugin-stt" }

# To opt-out of the recorder bridge and use only STT:
# tauri-plugin-stt = { path = "...", default-features = false }
```

### 2. Register the Plugin
In your `src-tauri/src/lib.rs` (or `main.rs`), initialize the plugin. This automatically registers the recorder plugin via the bridge feature internally.

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Register the STT plugin natively
        .plugin(tauri_plugin_stt::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 3. Tauri Permissions
Update your target app capabilities file (e.g. `src-tauri/capabilities/default.json`) to grant access to the `stt` plugin's commands. This allows access to both the STT functions and the pass-through recorder endpoints.

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "stt:default"
  ]
}
```

### 4. Install the Guest-JS Package
In your frontend directory, you must add the plugin's frontend API:

```bash
npm install "file:path/to/voice_plugin/tauri-plugin-stt"
# Or using yarn/pnpm/bun
```
*(Ensure the plugin has first been built so the `dist-js` folder exists)*

---

## Lifecycle Contract

The STT engine operates under a strict preload paradigm. Lazy initialization is not permitted prior to transcription calls. The lifecycle relies on an explicit `LifecycleState`:

- `uninitialized`: Prior to bootstrapping or after a fatal background tear-down.
- `initializing`: Process environment setup, model network fetch, or model memory preload is occurring.
- `ready`: Worker process is alive and the target model is loaded.
- `failed`: An unrecoverable error occurred during startup preload.

### Enforcement
- The application should explicitly query `sttHealth()` to check the `lifecycleState` or listen for `PROGRESS` events to determine readiness.
- The `transcribeFile()` operation strictly targets the preloaded STT worker; it will fail immediately with a `notReady` error if the `lifecycleState` is not `ready`.

## Code Examples (Frontend)

The frontend API exports all the commands and events from both the STT and Recorder domains in a unified location.

### Basic Workflow Example
A complete flow to activate hotkeys, listen for the recording artifact, and transcribe it.

```typescript
import { 
  bootstrapStt, 
  setHotkeyBindings, 
  startRecording,
  stopRecording,
  transcribeFile,
  subscribeOverlayState,
  createTranscriptDisplayState,
  CHANNELS
} from 'tauri-plugin-stt-api';
import { listen } from '@tauri-apps/api/event';

async function initializeAppVoice() {
  // 1. Initialize STT 
  await bootstrapStt();

  // 2. Set global hotkey to toggle recording (e.g., CommandOrControl+Shift+R)
  await setHotkeyBindings(
    { toggle: "CommandOrControl+Shift+R" }, 
    { outputDir: null, fileNamePrefix: "voice_note" }
  );

  // 3. Keep UI sync'd with background state phases (Idle, Recording, Transcribing, Error)
  await subscribeOverlayState((state) => {
    console.log("Current recorder phase is:", state.phase);
  });

  // 4. Automatically trigger transcribe when a hotkey recording completes
  listen(CHANNELS.COMPLETE, async (event: any) => {
    const wavPath = event.payload.result.recordingWavPath;
    
    // Pass the audio file up to STT model
    const transcriptRes = await transcribeFile({ path: wavPath });
    console.log("Final Transcribed Text:", transcriptRes.text);
  });
}
```

### Transcript Overlay Helper
The guest-js provides an un-opinionated state helper for the 2-second transcript overlay requirement. Use it in your UI rendering components:

```typescript
// Create the determinisitic 2000ms timer
const transcriptDisplay = createTranscriptDisplayState((text) => {
  const element = document.getElementById("overlayText");
  if (text) {
    element.innerText = text;
    element.style.display = "block";
  } else {
    element.innerText = "";
    element.style.display = "none";
  }
});

// Assuming you are listening to tauri-audio-plugin://live from transcript responses
listen(CHANNELS.LIVE, (event: any) => {
    transcriptDisplay.showTranscript(event.payload.text);
});
```

## Error Handling
When the `recorder-bridge` backend feature flag is disabled, any invocation of recorder commands via JS will safely return an error with `{ code: "feature_disabled", message: "Recorder bridge is disabled" }`.
