# tauri-plugin-stt to tauri-plugin-voice Migration Guide

This document explains what target apps need to change when moving from the old STT-focused package name to the voice plugin, and how to use the first local TTS command.

## What Changed

The public surface moved from `tauri-plugin-stt` to `tauri-plugin-voice`.

The rename affects:

- Rust dependency names
- Rust crate imports
- Plugin initialization
- Tauri command namespace
- Tauri permission namespace
- TypeScript package name
- TypeScript command wrappers
- Capability files and generated schemas

There are no legacy compatibility aliases. Consumers should update to the new names directly.

## Required Consumer Changes

### Rust Dependency

Update your `src-tauri/Cargo.toml` dependency from the old crate name to the new one.

```toml
[dependencies]
tauri-plugin-voice = { path = "path/to/voice_plugin/tauri-plugin-voice" }
```

If your app depended on the old crate name, replace it everywhere with `tauri-plugin-voice`.

### Rust Imports and Initialization

Update crate imports and plugin initialization to the new voice crate.

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_voice::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

If you imported `tauri_plugin_stt`, replace it with `tauri_plugin_voice`.

### Tauri Permissions

Update capability files from `stt:*` permissions to `voice:*`.

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

If you referenced individual permissions, rename them as well:

- `stt:allow-bootstrap-stt` -> `voice:allow-bootstrap-voice`
- `stt:allow-stt-health` -> `voice:allow-voice-health`
- `stt:allow-download-model` -> `voice:allow-download-model`
- `stt:allow-transcribe-file` -> `voice:allow-transcribe-file`

### Frontend Package

Update the frontend dependency to the voice package.

```bash
npm install "file:path/to/voice_plugin/tauri-plugin-voice"
```

Then update imports from the old package name to the new voice package name.

### Raw Invoke Namespace

If your app calls Tauri commands directly, update the namespace from:

```text
plugin:stt|...
```

to:

```text
plugin:voice|...
```

## Command Name Mapping

The core command names changed as follows:

- `bootstrap_stt` -> `bootstrap_voice`
- `stt_health` -> `voice_health`
- `download_model` stays `download_model`
- `transcribe_file` stays `transcribe_file`
- `setup_record_transcribe_pipeline` stays `setup_record_transcribe_pipeline`

Recorder bridge commands keep their names, but they now live under the voice plugin namespace.

## TTS Usage

The first local TTS command is `synthesize_speech`.

It generates a local WAV file and returns the output path. It does not play audio, stream audio, clone voices, or manage conversation turns.

### TypeScript Example

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

### TTS Request Fields

The first TTS contract accepts:

- `text`: required synthesis input
- `modelId`: optional model ID
- `voiceId`: optional Kokoro voice preset
- `languageCode`: optional language code
- `speed`: optional speech speed
- `outputPath`: optional exact WAV file path
- `outputDir`: optional output directory
- `fileNamePrefix`: optional generated file prefix

### TTS Defaults

When callers omit fields, the current defaults are:

- Model: `mlx-community/Kokoro-82M-bf16`
- Voice: `af_heart`
- Language: `a`
- Speed: `1.0`
- Output format: `wav`
- Output directory: `APP_DATA_DIR/tts/output`

### TTS Constraints

- Text must be non-empty.
- Output files must use a `.wav` extension.
- `outputPath` and `outputDir` are mutually exclusive.
- The plugin returns a generated local file path instead of playback.

## Migration Checklist

1. Rename the Rust dependency to `tauri-plugin-voice`.
2. Replace `tauri_plugin_stt` imports with `tauri_plugin_voice`.
3. Update plugin initialization to `tauri_plugin_voice::init()`.
4. Change capability permissions from `stt:*` to `voice:*`.
5. Update raw command invocations from `plugin:stt|...` to `plugin:voice|...`.
6. Update frontend imports to the voice package.
7. Switch bootstrap and health calls to `bootstrapVoice` and `voiceHealth`.
8. Use `synthesizeSpeech` for local TTS output.

