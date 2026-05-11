# High Level Design: Voice Plugin Rename and STT/TTS Architecture (CODIN-340)

## 1. Objective

Rename the current STT-focused plugin into `tauri-plugin-voice` and define the high-level architecture for one local voice plugin that owns separate STT and TTS engines through shared runtime infrastructure.

The design must make the public surface readable, avoid long-lived compatibility aliases, keep STT behavior intact during migration, and prepare TTS without implementing a TTS model in this phase.

## 2. Current State

The current plugin is implemented under `tauri-plugin-stt` and exposes STT-first naming across the Rust crate, Tauri namespace, TypeScript package, generated permissions, and consumer configuration.

Key current surfaces:

- Rust package and crate: `tauri-plugin-stt`, `tauri_plugin_stt`.
- Rust extension: `TauriPluginSttExt` and `tauri_plugin_stt()`.
- Tauri namespace: `plugin:stt|...`.
- Permission namespace: `stt:*`.
- TypeScript package: `tauri-plugin-tauri-plugin-stt-api`.
- STT commands: `bootstrap_stt`, `download_model`, `transcribe_file`, `stt_health`, and `setup_record_transcribe_pipeline`.
- Recorder bridge commands exposed through the same plugin namespace.
- Model progress event payloads with no explicit engine discriminator.

The public rename impact audit confirms this is a breaking public surface change unless explicit compatibility aliases are added.

## 3. Decision Summary

- Rename the primary public surface to `tauri-plugin-voice`.
- Use `plugin:voice|...` as the only Tauri command namespace.
- Use `voice:*` as the only permission namespace.
- Do not keep legacy `plugin:stt|...` commands as temporary aliases.
- Do not keep legacy Rust or TypeScript compatibility aliases.
- Keep STT domain language where it describes STT behavior.
- Add TTS as a separate engine, not as an extension of STT state.
- Share Python bootstrap and app data conventions by default.
- Add an explicit engine discriminator to model progress and model management contracts.
- Keep recorder bridge behavior pass-through and scoped to record/transcribe flows.

## 4. Compatibility Strategy

This rename should be a clean breaking change.

Temporary aliases are not recommended because they would require maintaining two command namespaces, two permission namespaces, duplicate generated schemas, duplicate TypeScript wrappers, and parallel documentation. That would reduce readability and create unclear ownership for future STT and TTS commands.

Migration should instead be explicit:

- Consumers update Rust dependency names and crate imports to `tauri-plugin-voice` and `tauri_plugin_voice`.
- Consumers update plugin initialization to the new crate.
- Consumers update capabilities from `stt:*` to `voice:*`.
- Frontend callers update raw invokes from `plugin:stt|...` to `plugin:voice|...`.
- Frontend callers update TypeScript imports to the renamed package.
- Steno is migrated as a normal consumer after the rename LLD is approved.
- Historical specs can remain unchanged unless they are current migration guidance.

The breaking change should be documented in migration notes, but no runtime compatibility layer should be added.

## 5. Target Public Surface

### 5.1 Rust Surface

The Rust package becomes `tauri-plugin-voice`, and the crate becomes `tauri_plugin_voice`.

Primary Rust symbols should use voice naming:

- `TauriPluginVoice`
- `TauriPluginVoiceExt`
- `tauri_plugin_voice()`
- `init()`

STT and TTS engine internals may use `Stt` and `Tts` names when they describe engine-specific behavior.

### 5.2 Tauri Command Surface

All commands should live under `plugin:voice|...`.

Voice-level commands:

- `bootstrap_voice`
- `voice_health`

STT commands:

- `transcribe_file`
- `download_model` with `engine = stt`

TTS commands, once the TTS contract is approved:

- `synthesize_speech`
- `download_model` with `engine = tts`

Recorder bridge commands keep their recorder-oriented names, but move to the voice namespace:

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
- `setup_record_transcribe_pipeline`

### 5.3 TypeScript Surface

The TypeScript package should be renamed to a voice package name and expose voice-level entry points.

Recommended exported functions:

- `bootstrapVoice`
- `voiceHealth`
- `downloadModel`
- `transcribeFile`
- `synthesizeSpeech`, after TTS contract approval
- Existing recorder bridge function names, with updated voice namespace invokes

STT-specific compatibility exports such as `bootstrapStt` and `sttHealth` should not be retained after the clean rename. If a later migration note needs examples, it should show old-to-new replacements instead of exporting aliases.

## 6. Engine Ownership

The voice plugin should have shared infrastructure and separate engine ownership.

Shared infrastructure:

- App-local Python directory under app data.
- Python environment creation through the existing `uv` sidecar path.
- Dependency install orchestration.
- App data layout conventions.
- Process supervision patterns.
- Startup preload orchestration.
- Health aggregation.
- Model progress event conventions.

STT ownership:

- Audio-to-text transcription.
- STT worker process and newline-delimited protocol.
- Whisper or STT model allowlist.
- STT model preload and download behavior.
- Transcript output validation.
- STT readiness diagnostics.
- STT-specific worker failures.

TTS ownership:

- Text-to-audio synthesis.
- TTS worker process and protocol.
- TTS model allowlist.
- TTS model preload and download behavior.
- Audio output path and playback policy.
- TTS readiness diagnostics.
- TTS-specific worker failures.

STT and TTS should not share one worker process. Separate workers keep failure handling, model loading, and future protocol changes readable.

## 7. Shared Python Bootstrap

The default bootstrap path should use one app-local Python environment at `APP_DATA_DIR/python/.venv`.

The shared environment remains appropriate because it avoids duplicated Python setup and matches the current plugin model. Engine-specific dependency lock files can still exist inside the plugin source tree if they are clearer than one combined lock file.

Recommended bootstrap behavior:

- `bootstrap_voice` ensures the shared Python environment exists.
- It installs the selected dependency set for configured engines.
- It initializes only engines enabled by configuration.
- It returns readiness details for each configured engine.
- It does not implicitly enable TTS before the TTS runtime is selected.

If the chosen TTS runtime conflicts with STT dependencies, the LLD should revisit this decision and split the environment by engine. That split should be treated as an exception, not the default architecture.

## 8. Startup Preload Behavior

Startup preload remains non-blocking.

Plugin setup should return quickly after state construction and task scheduling. Background preload should then run for configured engines:

- STT preload can remain enabled by default to preserve current STT startup behavior.
- TTS preload should be configurable and should remain disabled until the TTS runtime/model choice is approved.
- Each engine tracks its own startup preload state.
- Voice-level readiness aggregates only the engines configured for preload or use.

Manual bootstrap and model download must coordinate with startup preload so the same engine does not spawn duplicate workers or report contradictory state.

## 9. Readiness Model

`voice_health` is the canonical readiness entry point.

It should report:

- Aggregate voice status.
- STT readiness.
- TTS readiness.
- Per-engine lifecycle state.
- Per-engine diagnostics.
- Shared bootstrap diagnostics.
- Startup preload state per engine.

Lifecycle states should remain simple:

- `uninitialized`
- `initializing`
- `ready`
- `failed`

Aggregate voice readiness should be ready only when all required configured engines are ready. If TTS is not configured, TTS should not block aggregate readiness.

STT-dependent commands must check STT readiness. TTS-dependent commands must check TTS readiness. Record/transcribe pipeline setup must check STT readiness, not aggregate voice readiness, because TTS is unrelated to that flow.

## 10. Progress Events

Model progress events should gain an explicit engine discriminator before TTS is added.

Recommended fields:

- `contractVersion`
- `eventId`
- `emittedAtMs`
- `engine`
- `model`
- `phase`
- `state`
- `percent`
- `filename`
- `error`

The event channel should move to voice naming as part of the clean break. A readable target is `tauri-voice-plugin://model-progress`.

The current phase and state values remain usable:

- `phase`: `download`, `preload`
- `state`: `start`, `in_progress`, `complete`, `failed`

STT and TTS workers may emit partial engine-local progress, but Rust should publish the final public event shape so frontend consumers receive one stable contract.

## 11. TTS Contract Direction

The first TTS API should use a minimal file-output contract.

Recommended initial behavior:

- Input: text, optional model ID, optional voice ID, optional output path or output directory.
- Output: generated audio file path, duration when available, model ID, and voice ID when available.
- No direct playback in the first contract.
- No streaming in the first contract.
- No conversational turn orchestration in the first contract.

File output is the smallest useful boundary. It keeps TTS separate from playback policy, avoids tying synthesis to UI state, and mirrors the current STT file-in/file-out shape.

Runtime/model selection is intentionally deferred to the dedicated TTS runtime task. This HLD only defines where the selected runtime will plug in.

## 12. Recorder Bridge Boundaries

The recorder bridge remains a pass-through bridge, not a recorder rewrite.

Allowed changes:

- Move recorder bridge commands from `plugin:stt|...` to `plugin:voice|...`.
- Move recorder permissions from `stt:*` to `voice:*`.
- Keep command names recorder-compatible.
- Keep recorder event payloads unchanged unless a later recorder task requires otherwise.
- Keep `setup_record_transcribe_pipeline` STT-gated.

Out of scope:

- Redesigning recorder internals.
- Changing recorder event channels.
- Making recorder depend on TTS.
- Adding conversational pipeline orchestration.
- Moving recorder ownership into STT or TTS engines.

## 13. Component Impact

### `tauri-plugin-stt/Cargo.toml`

Rename package metadata, crate metadata, and links metadata to voice naming during implementation.

### `tauri-plugin-stt/src/lib.rs`

Rename the plugin namespace from `stt` to `voice`, rename public extension types, and register the approved voice command surface.

### `tauri-plugin-stt/src/desktop.rs`

Split runtime state into STT and TTS ownership while keeping shared bootstrap and startup coordination readable. Preserve existing STT worker behavior.

### `tauri-plugin-stt/src/bootstrap_manager.rs`

Rename bootstrap responsibility to voice-level bootstrap and keep shared environment setup as the default.

### `tauri-plugin-stt/src/models.rs`

Add engine-aware request, health, and progress models. Keep STT request/response models readable and add TTS models only after the TTS contract is approved.

### `tauri-plugin-stt/src/commands.rs`

Expose voice namespace commands and remove legacy STT namespace assumptions.

### `tauri-plugin-stt/guest-js/index.ts`

Rename package-facing functions, update invoke strings, and expose engine-aware model progress and health types.

### Permissions and generated schemas

Regenerate around the `voice:*` namespace after command names are finalized in the LLD.

### Example app and Steno

Treat both as consumers. Migrate them after the plugin rename implementation plan is approved.

## 14. Risks and Mitigations

- Risk: Clean break temporarily breaks Steno.
  - Mitigation: Make Steno migration an explicit follow-up implementation step with migration notes.
- Risk: TTS dependencies conflict with STT dependencies.
  - Mitigation: Start with shared Python environment, but allow LLD fallback to separate engine environments.
- Risk: Aggregate readiness hides engine-specific failure.
  - Mitigation: Require per-engine diagnostics in `voice_health`.
- Risk: Progress events become ambiguous after TTS.
  - Mitigation: Add `engine` before TTS progress events exist.
- Risk: Recorder bridge scope expands during rename.
  - Mitigation: Keep recorder behavior pass-through and only update namespace/permissions.
- Risk: Voice-level commands become too generic.
  - Mitigation: Use voice-level names only for aggregate lifecycle, and keep domain names for STT and TTS behavior.

## 15. Validation Expectations

- HLD resolves clean breaking rename versus compatibility aliases.
- LLD defines file-level rename and engine ownership changes before implementation.
- Plugin public surface uses `tauri-plugin-voice`, `tauri_plugin_voice`, `plugin:voice|...`, and `voice:*`.
- No legacy `plugin:stt|...` aliases remain in the target contract.
- STT transcription behavior remains functionally intact after migration.
- `voice_health` distinguishes aggregate, STT, and TTS readiness.
- Model progress events include `engine`.
- Startup preload remains non-blocking and per-engine.
- Recorder bridge commands remain pass-through and STT-gated where needed.
- Example app and Steno migration are validated after the rename implementation.

## 16. Exit Criteria

- This HLD is approved as the architecture boundary for the rename.
- The next task writes an LLD that translates these decisions into file-level edits.
- No TTS runtime implementation begins until the TTS API contract and runtime choice are approved.
