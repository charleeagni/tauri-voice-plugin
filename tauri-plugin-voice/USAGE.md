# Tauri Voice Plugin Usage Guide

This plugin (`tauri-plugin-voice`) provides standalone Speech-to-Text (STT), first local file-output Text-to-Speech (TTS), and a bridge for the `tauri-plugin-recorder`. By using the voice plugin, target applications can natively access transcription, synthesis, and recording/hotkey features without configuring the recorder plugin separately.

## Integration Steps

To consume the plugin in a target Tauri app, you need to add both the Rust backend and the JavaScript/TypeScript frontend configuration.

### 1. Add the Rust Dependency
In your target Tauri app's `src-tauri/Cargo.toml`, add the `tauri-plugin-voice` dependency. The recorder bridge is enabled by default.

```toml
[dependencies]
tauri-plugin-voice = { path = "path/to/voice_plugin/tauri-plugin-voice" }

# To opt-out of the recorder bridge and use only voice commands:
# tauri-plugin-voice = { path = "...", default-features = false }
```

### 2. Register the Plugin
In your `src-tauri/src/lib.rs` (or `main.rs`), initialize the plugin. This automatically registers the recorder plugin via the bridge feature internally.

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Register the voice plugin natively
        .plugin(tauri_plugin_voice::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 3. Tauri Permissions
Update your target app capabilities file (e.g. `src-tauri/capabilities/default.json`) to grant access to the `voice` plugin's commands. This allows access to both STT functions and pass-through recorder endpoints.

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "voice:default"
  ]
}
```

### 4. Install the Guest-JS Package
In your frontend directory, you must add the plugin's frontend API:

```bash
npm install "file:path/to/voice_plugin/tauri-plugin-voice"
# Or using yarn/pnpm/bun
```
*(Ensure the plugin has first been built so the `dist-js` folder exists)*

---

## Lifecycle Contract

The STT engine operates under a strict preload paradigm. Lazy initialization is not permitted prior to transcription calls. TTS starts lazily on the first `synthesizeSpeech()` call and returns a generated WAV file path. The lifecycle relies on an explicit `LifecycleState`:

- `uninitialized`: Prior to bootstrapping or after a fatal background tear-down.
- `initializing`: Process environment setup, model network fetch, or model memory preload is occurring.
- `ready`: Worker process is alive and the target model is loaded.
- `failed`: An unrecoverable error occurred during startup preload.

### Enforcement
- The application should explicitly query `voiceHealth()` to check the `lifecycleState` or listen for `PROGRESS` events to determine readiness.
- The `transcribeFile()` operation strictly targets the preloaded STT worker; it will fail immediately with a `notReady` error if the `lifecycleState` is not `ready`.
- The `synthesizeSpeech()` operation targets only TTS readiness and does not require STT or recorder readiness.

## Code Examples (Frontend)

The frontend API exports all the commands and events from both the STT and Recorder domains in a unified location.

### Basic Workflow Example
A complete flow to activate hotkeys, listen for the recording artifact, and transcribe it.

```typescript
import { 
  bootstrapVoice, 
  setHotkeyBindings, 
  startRecording,
  stopRecording,
  transcribeFile,
  subscribeOverlayState,
  createTranscriptDisplayState,
  CHANNELS
} from 'tauri-plugin-voice-api';
import { listen } from '@tauri-apps/api/event';

async function initializeAppVoice() {
  // 1. Initialize STT 
  await bootstrapVoice();

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

### Local TTS Example
Generate a WAV file from text with the default Kokoro model and voice.

```typescript
import { bootstrapVoice, synthesizeSpeech } from 'tauri-plugin-voice-api';

async function createSpeechFile() {
  await bootstrapVoice();

  const result = await synthesizeSpeech({
    text: "Hello from local Kokoro TTS.",
    voiceId: "af_heart",
    languageCode: "a",
    speed: 1.0,
    fileNamePrefix: "reply"
  });

  console.log("Generated WAV:", result.audioPath);
}
```

TTS v1 writes WAV files only. It does not play audio directly, stream chunks, clone voices, or manage conversation turns.

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
