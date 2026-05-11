# High-Level Design: TTS Example Application

## Objective
Update the existing example application (`tauri-plugin-voice/examples/tauri-app`) to demonstrate and debug the new Text-to-Speech (TTS) capabilities of `tauri-plugin-voice`, alongside the existing STT features.

## Proposed Changes

### 1. Rename and Reframe UI
- Rename the main header from "STT Plugin Debugger" to "Voice Plugin Debugger".

### 2. Update Model Management (Section 5)
- Enhance the model management section to support downloading both STT and TTS models.
- Add a dropdown to select the engine (`stt` or `tts`) for the `download_model` command.
- When `tts` is selected, the model ID dropdown should include the default TTS model: `mlx-community/Kokoro-82M-bf16`.
- The model progress display will now show the engine type, as progress events will contain `engine = stt | tts`.

### 3. Add Section 6 — Text-to-Speech (TTS)
Introduce a new section to interact with the TTS API:
- **Text Input**: A `<textarea>` for the user to enter the text they want to synthesize.
- **Model & Voice Selection**: 
  - Input field or dropdown for `modelId` (defaulting to the downloaded TTS model).
  - Input field for `voiceId` (optional, for models that support multiple voices).
- **Synthesize Button**: A "Synthesize Speech" button that calls the new `synthesize_speech` command via `tauri-plugin-voice-api`.
- **Result Display**:
  - Show the returned `outputAudioPath`, `modelId`, `voiceId`, and `duration` in the result log.
  - Provide an HTML5 `<audio>` element to play the generated audio file directly in the app. Use Tauri's `@tauri-apps/api/core` `convertFileSrc` to load the local file path into the `<audio>` element securely.

### 4. API Integration
The app will use the updated/new API methods from `tauri-plugin-voice-api`:
- `downloadModel({ engine: 'tts', modelId: '...' })`
- `synthesizeSpeech({ text: '...', modelId: '...', voiceId: '...', outputDir: '...' })`

## Validation
- The example app successfully downloads a TTS model.
- The user can type text, synthesize it, and receive an audio file path.
- The user can play the generated audio file within the example app.
- Existing STT functionality remains unaffected.