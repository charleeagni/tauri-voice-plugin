# Low Level Design: Voice Plugin Rename and TTS Preparation (CODIN-341)

## 1. Objective

Translate the approved HLD for CODIN-340 into an implementation-ready plan for renaming the current STT plugin to `tauri-plugin-voice` and preparing the runtime for future TTS support.

This LLD does not implement TTS model execution. It defines the file-level edits, public command contract, runtime ownership split, validation plan, and rollback plan needed before implementation starts.

## 2. Approved HLD Decisions

- Rename the primary public surface to `tauri-plugin-voice`.
- Use `plugin:voice|...` as the only Tauri command namespace.
- Use `voice:*` as the only permission namespace.
- Do not keep legacy `plugin:stt|...` command aliases.
- Do not keep legacy Rust or TypeScript compatibility aliases.
- Keep STT language where it describes STT behavior.
- Add TTS as a separate engine with its own state ownership.
- Keep one shared app-local Python environment by default.
- Add an explicit `engine` discriminator to model download and model progress contracts.
- Keep recorder bridge behavior pass-through and scoped to record/transcribe flows.

## 3. Scope

### In Scope

- Rename package, crate, plugin namespace, extension trait, extension method, and guest package surface.
- Replace aggregate STT lifecycle commands with voice lifecycle commands.
- Keep STT behavior intact under voice naming.
- Add engine-aware models for bootstrap, health, downloads, and progress.
- Split runtime ownership into explicit STT and TTS state holders inside the plugin.
- Add TTS readiness placeholders without starting a TTS worker.
- Regenerate permissions and schemas for the `voice:*` namespace.
- Update first-party example app references and tests as part of the implementation task.
- Document migration from old names to new names.

### Out of Scope

- TTS model/runtime selection.
- TTS dependency installation.
- TTS worker implementation.
- Audio playback policy.
- Streaming TTS.
- Conversational orchestration.
- Long-lived compatibility aliases.
- Recorder bridge redesign.
- Updating unrelated historical specs.
- Updating external consumer apps before the plugin rename implementation is ready.

## 4. Target Public Contract

### Rust Crate Surface

| Current | Target | Notes |
| --- | --- | --- |
| `tauri-plugin-stt` | `tauri-plugin-voice` | Rename package metadata and consumer dependency key. |
| `tauri_plugin_stt` | `tauri_plugin_voice` | Rename crate imports. |
| `TauriPluginStt` | `TauriPluginVoice` | Managed plugin state type. |
| `TauriPluginSttExt` | `TauriPluginVoiceExt` | Public extension trait. |
| `tauri_plugin_stt()` | `tauri_plugin_voice()` | Public extension method. |
| `init()` | `init()` | Keep default Tauri plugin initializer name. |

No legacy Rust aliases should remain.

### Tauri Command Surface

All commands move to the `plugin:voice|...` namespace.

| Current command | Target command | Ownership |
| --- | --- | --- |
| `bootstrap_stt` | `bootstrap_voice` | Shared bootstrap plus configured engine bootstrap. |
| `stt_health` | `voice_health` | Aggregate readiness plus per-engine diagnostics. |
| `download_model` | `download_model` | Engine-aware model management. |
| `transcribe_file` | `transcribe_file` | STT engine. |
| `setup_record_transcribe_pipeline` | `setup_record_transcribe_pipeline` | Recorder bridge, STT-gated. |
| Recorder bridge commands | Same command names | Recorder bridge, voice namespace only. |

TTS command names are reserved for later work. `synthesize_speech` should not be registered until the TTS command and worker contract task is approved.

### TypeScript Surface

| Current export | Target export | Notes |
| --- | --- | --- |
| `bootstrapStt` | `bootstrapVoice` | Calls `plugin:voice|bootstrap_voice`. |
| `sttHealth` | `voiceHealth` | Calls `plugin:voice|voice_health`. |
| `downloadModel` | `downloadModel` | Adds required `engine`. |
| `transcribeFile` | `transcribeFile` | STT-specific behavior remains clear. |
| Recorder bridge exports | Same names | Invoke namespace changes to voice. |

No `bootstrapStt`, `sttHealth`, or legacy package exports should remain.

### Permission Surface

| Current | Target |
| --- | --- |
| `stt:default` | `voice:default` |
| `stt:allow-bootstrap-stt` | `voice:allow-bootstrap-voice` |
| `stt:allow-stt-health` | `voice:allow-voice-health` |
| `stt:allow-download-model` | `voice:allow-download-model` |
| `stt:allow-transcribe-file` | `voice:allow-transcribe-file` |
| Recorder bridge permissions under `stt:*` | Recorder bridge permissions under `voice:*` |

Generated permission files should be regenerated from command metadata rather than manually rewritten where the Tauri plugin workflow supports generation.

## 5. Command Contract Changes

### `bootstrap_voice`

Request:

- Keep the existing empty request acceptable.
- Add optional engine configuration only if the implementation already needs it for STT preload control.
- Do not add TTS enablement fields until the TTS runtime task approves them.

Response:

- Preserve tagged `status` values: `ready` and `alreadyReady`.
- Return voice-level details.
- Include per-engine bootstrap details if added without obscuring the simple status.

Behavior:

- Ensure the shared Python environment exists at `APP_DATA_DIR/python/.venv`.
- Install only the currently approved STT dependency set.
- Start or reuse the STT worker with the default model.
- Leave TTS uninitialized unless a later task explicitly enables it.

### `voice_health`

Request:

- Keep the existing empty request.

Response:

- Return aggregate voice status.
- Return STT readiness with lifecycle state and diagnostics.
- Return TTS readiness as uninitialized or disabled until TTS is configured.
- Return shared bootstrap diagnostics.
- Return startup preload state per engine.

Behavior:

- Aggregate readiness is ready when all required configured engines are ready.
- STT is required for current transcription and recorder pipeline flows.
- TTS does not block aggregate readiness while it is unconfigured.
- STT-dependent commands must gate on STT readiness, not aggregate readiness.

### `download_model`

Request:

- Add required `engine`.
- Keep `modelId`.
- Accept `engine = "stt"` in this implementation.
- Reject `engine = "tts"` until the TTS runtime/model task is approved.

Response:

- Keep `modelId`.
- Keep `alreadyActive`.
- Add `engine`.

Behavior:

- Route `stt` requests to the existing STT model allowlist and worker replacement path.
- Reject unsupported engines with a typed invalid-input error.
- Avoid any generic registry or dispatcher unless implementation clarity requires it later.

### `transcribe_file`

Request:

- Keep `path`.
- Keep optional `modelId`.

Response:

- Keep `text`.

Behavior:

- Continue to require STT readiness.
- Continue to validate that the audio path exists and is a file.
- Continue to use the persistent STT worker protocol.
- Do not depend on TTS readiness.

### Recorder Bridge Commands

Command names remain unchanged and move only by namespace to `plugin:voice|...`.

`setup_record_transcribe_pipeline` remains STT-gated. It must check STT readiness, not aggregate voice readiness, because TTS is unrelated to recording and transcription.

Recorder event payloads and recorder command behavior stay unchanged.

## 6. State Ownership Design

### Shared Voice State

The managed plugin state should become `TauriPluginVoice`. It should own:

- App handle.
- Shared bootstrap status.
- Shared Python environment diagnostics.
- STT engine state.
- TTS engine state placeholder.
- Recorder pipeline state.

This keeps one public plugin state while making engine ownership explicit.

### STT Engine State

STT state should own:

- Persistent STT worker slot.
- STT model download/preload guard.
- STT startup preload status.
- STT startup error.
- Active STT model ID.
- STT model allowlist and model repo mapping.

Existing STT worker behavior should move behind this ownership boundary with minimal behavior changes.

### TTS Engine State Placeholder

TTS state should own:

- TTS lifecycle state.
- TTS startup preload status.
- TTS startup error.
- Optional active TTS model ID.

It should not own a worker process yet. The TTS command and runtime task will add the worker protocol, dependency lock file, and model allowlist later.

### Recorder Pipeline State

Recorder pipeline state stays in the voice plugin because it coordinates recorder completion with STT transcription.

It must remain separate from both engine worker state and TTS readiness.

## 7. File-Level Implementation Plan

### `tauri-plugin-stt/`

Rename the source package folder to `tauri-plugin-voice` during implementation. Update local path references in the example app and migration notes at the same time.

If the folder rename creates a large diff, perform it as a mechanical step separate from behavioral edits.

### `tauri-plugin-stt/Cargo.toml`

- Rename package metadata to `tauri-plugin-voice`.
- Rename `links` metadata to `tauri-plugin-voice`.
- Keep features and dependencies unchanged unless a renamed dependency path requires an update.
- Do not add TTS dependencies in this task.

### `tauri-plugin-stt/src/lib.rs`

- Rename imported desktop state to `TauriPluginVoice`.
- Rename extension trait to `TauriPluginVoiceExt`.
- Rename extension method to `tauri_plugin_voice`.
- Change `Builder::new("stt")` to `Builder::new("voice")`.
- Register `bootstrap_voice` and `voice_health`.
- Keep `download_model`, `transcribe_file`, recorder bridge commands, and `setup_record_transcribe_pipeline`.
- Remove old command registrations for `bootstrap_stt` and `stt_health`.
- Keep startup preload non-blocking.

### `tauri-plugin-stt/src/commands.rs`

- Rename command wrappers from `bootstrap_stt` to `bootstrap_voice`.
- Rename command wrappers from `stt_health` to `voice_health`.
- Route wrappers through `app.tauri_plugin_voice()`.
- Keep recorder bridge conversion behavior.
- Update recorder bridge comments from STT plugin language to voice plugin language.
- Do not add TTS command wrappers.

### `tauri-plugin-stt/src/desktop.rs`

- Rename `TauriPluginStt` to `TauriPluginVoice`.
- Introduce explicit STT and TTS state fields inside the plugin state.
- Move current worker, download guard, startup error, and startup preload status into STT state.
- Add TTS lifecycle placeholder state without worker spawning.
- Rename `bootstrap_stt` to `bootstrap_voice`.
- Rename `stt_health` to `voice_health`.
- Keep STT helper names when they describe STT behavior, such as `SttWorker`.
- Add engine-aware progress event emission with `engine = "stt"`.
- Rename progress event channel to `tauri-voice-plugin://model-progress`.
- Keep recorder state, live event behavior, and complete event listener behavior unchanged except for voice naming.

### `tauri-plugin-stt/src/bootstrap_manager.rs`

- Rename `bootstrap_stt` to `bootstrap_voice`.
- Keep the shared Python environment path at `APP_DATA_DIR/python/.venv`.
- Keep `requirements/requirements-stt.lock.txt` as the STT dependency lock file.
- Do not install TTS dependencies yet.
- Return voice-level bootstrap wording.

### `tauri-plugin-stt/src/models.rs`

- Add an `Engine` enum with `stt` and `tts`.
- Add `engine` to `DownloadModelRequest`.
- Add `engine` to `DownloadModelResponse`.
- Add `engine` to `ModelProgressEvent`.
- Update health response models to represent aggregate voice readiness and per-engine readiness.
- Keep `TranscribeRequest` and `TranscribeResponse` unchanged.
- Keep recorder bridge models unchanged.
- Add TTS placeholder health fields only; no synthesize request/response yet.

### `tauri-plugin-stt/guest-js/index.ts`

- Rename package-facing lifecycle functions to `bootstrapVoice` and `voiceHealth`.
- Update all invoke strings from `plugin:stt|...` to `plugin:voice|...`.
- Add `engine` to `DownloadModelRequest`, `DownloadModelResponse`, and `ModelProgressEvent`.
- Update the model progress channel to `tauri-voice-plugin://model-progress`.
- Keep recorder bridge function names.
- Remove legacy `bootstrapStt` and `sttHealth` exports.

### `tauri-plugin-stt/package.json`

- Rename package to the approved voice API package name.
- Prefer `tauri-plugin-voice-api` unless package publishing rules require a scoped name.
- Update metadata text from STT-only language to voice plugin language.

### `tauri-plugin-stt/build.rs`

- Replace `bootstrap_stt` with `bootstrap_voice`.
- Replace `stt_health` with `voice_health`.
- Keep recorder bridge commands and `transcribe_file`.
- Regenerate permission metadata after command list changes.

### `tauri-plugin-stt/permissions/`

- Regenerate autogenerated command permissions.
- Replace default permission set with `voice:*` identifiers.
- Remove generated `bootstrap_stt` and `stt_health` permission entries.
- Add generated `bootstrap_voice` and `voice_health` permission entries.
- Keep recorder bridge permission names under the `voice` namespace.

### `tauri-plugin-stt/tests/`

- Update smoke contract tests to use `tauri_plugin_voice`.
- Add serialization coverage for `engine` on model download and progress events.
- Add health serialization coverage for aggregate voice readiness and per-engine readiness.
- Keep STT request and recorder bridge shape tests.

### Example App

- Update Rust dependency and imports to `tauri-plugin-voice` and `tauri_plugin_voice`.
- Update capability permissions to `voice:default`.
- Update TypeScript dependency/import name.
- Update generated schemas after the plugin rename.
- Keep example behavior unchanged except for naming.

### Documentation

- Update current README and usage docs to describe `tauri-plugin-voice`.
- Add a migration note listing old-to-new replacements.
- State that no legacy aliases are available.
- Leave historical specs unchanged unless they are current migration guidance.

## 8. Compatibility Alias Handling

No compatibility aliases are approved.

Implementation must not keep:

- `plugin:stt|...` command namespace.
- `stt:*` permission namespace.
- `tauri_plugin_stt` crate alias.
- `TauriPluginSttExt` trait alias.
- `tauri_plugin_stt()` extension method alias.
- `bootstrapStt` TypeScript export.
- `sttHealth` TypeScript export.

Migration should fail clearly at compile time or permission validation time for consumers still using old names.

## 9. Validation Plan

### Static Validation

- Run Rust formatting for the renamed crate.
- Run TypeScript formatting or type checks if configured.
- Run `rg` for legacy public names after implementation.
- Confirm no `plugin:stt|...` invoke strings remain in current source or active docs.
- Confirm no `stt:*` permissions remain in active capabilities or generated plugin permissions.
- Confirm no `bootstrapStt` or `sttHealth` exports remain.

### Contract Validation

- Run crate tests.
- Verify serialization tests cover `engine`.
- Verify `voice_health` response distinguishes aggregate, STT, and TTS readiness.
- Verify `download_model` rejects unsupported engines.
- Verify `download_model` with `engine = "stt"` preserves current STT model behavior.

### Runtime Smoke Validation

- Start the example app with the renamed plugin.
- Confirm `bootstrapVoice` completes.
- Confirm `voiceHealth` reports STT readiness after bootstrap.
- Confirm `transcribeFile` still returns text for a known audio file.
- Confirm `setupRecordTranscribePipeline` remains STT-gated.
- Confirm model progress events include `engine = "stt"` and publish on the voice progress channel.

### Consumer Migration Validation

- Update Steno in a separate consumer migration step after the plugin implementation passes local validation.
- Regenerate Steno schemas after switching capabilities to `voice:default`.
- Verify Steno compile and transcription flow separately from this plugin LLD task.

## 10. Rollback Plan

### Before Consumer Migration

- Revert the plugin rename commit or branch if the example app or crate tests fail.
- Keep consumers pinned to the existing `tauri-plugin-stt` path until the voice plugin validates.
- Do not partially migrate Steno before the renamed plugin compiles and passes smoke validation.

### After Plugin Rename, Before TTS Work

- Roll back by restoring the previous package folder, crate metadata, plugin namespace, command names, permissions, and guest package name together.
- Regenerate schemas after rollback rather than manually editing generated files.
- Preserve the public contract as one consistent set; do not mix `voice` crate names with `stt` permissions.

### After Consumer Migration

- Roll back consumers and plugin together.
- Restore old consumer dependencies, imports, capability permissions, and generated schemas.
- Re-run consumer compile and transcription smoke tests after rollback.

## 11. Implementation Order

1. Rename crate and package metadata.
2. Rename public Rust state, trait, and extension method.
3. Rename Tauri namespace and lifecycle commands.
4. Add engine-aware model contracts.
5. Split STT and TTS state ownership in `desktop.rs`.
6. Update guest JS exports and invoke strings.
7. Regenerate permissions and schemas.
8. Update tests.
9. Update example app.
10. Update current docs and migration notes.
11. Validate crate, example app, and public contract scans.

## 12. Exit Criteria

- LLD is present under the `2. Extend Voice Plugin` module spec folder.
- File-level rename and state ownership changes are clear enough for implementation.
- Compatibility alias policy is explicit.
- Validation and rollback plans are documented.
- Architecture diagrams are available in the same spec folder.
- No implementation source files are changed by this LLD task.
