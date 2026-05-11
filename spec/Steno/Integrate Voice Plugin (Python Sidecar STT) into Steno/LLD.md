# Low Level Design: Integrate Voice Plugin (Python Sidecar STT) into Steno (CODIN-301)

## 1. Objective
Implement the Steno-side Track 2 integration for MLX by adding `mlx` as a transcription runtime that uses `tauri-plugin-voice` only as a host-driven file transcription backend. Steno keeps ownership of recording, overlay, runtime phases, and output flow.

## 2. Parent Context
- Parent task: CODIN-301
- HLD: [spec/Steno/Integrate Voice Plugin (Python Sidecar STT) into Steno/HLD.md](/Users/karthik/merge_conflicts/voice_plugin/spec/Steno/Integrate%20Voice%20Plugin%20%28Python%20Sidecar%20STT%29%20into%20Steno/HLD.md)

## 3. Scope Boundaries
- In scope:
  - Add `mlx` to Steno runtime selection.
  - Persist `runtime_selection = mlx` and `mlx_model_id`.
  - Proactively bootstrap and health-check the voice plugin when MLX is selected.
  - Route final post-stop transcription through the plugin.
  - Integrate MLX models into Steno's existing Models section and settings flow.
  - Map plugin `PROGRESS` events into Steno's existing model-download snapshot surface.
  - Surface plugin failures through Steno's existing runtime error path.
- Out of scope:
  - Any `voice_plugin` overlay, recorder, or recorder-bridge takeover work.
  - Any plugin-side Rust or Python bridge repair work.
  - Queue or cancel support for MLX downloads.
  - A parallel MLX-specific settings page or runtime state machine.

## 4. Fixed Decisions

### 4.1 Integration ownership
- Steno integrates the plugin from the Rust host side, not from the React layer.
- The frontend continues talking to Steno commands and Steno events.
- Steno re-emits or adapts plugin behavior into existing Steno state instead of exposing plugin-native events directly to the UI.

### 4.2 Plugin dependency shape
- Add `tauri-plugin-voice` to the Steno Rust host with `default-features = false`.
- This keeps the recorder bridge disabled so the integration stays tightly scoped to file transcription only.
- Add `tauri-plugin-shell` and register it before the voice plugin because the plugin relies on the `uv` sidecar.
- Update `tauri.conf.json` to package the `uv` external binary and set the plugin config default model to `tiny.en`.

### 4.3 Runtime behavior for `mlx`
- `mlx` is a final-only runtime in V1.
- Steno keeps its existing recording lifecycle and phase transitions.
- When `runtime_selection == mlx`, Steno must not start interim preview transcription, live-word decoding, or incremental chunk commit behavior.
- Final transcription happens only after recording stops.

### 4.4 MLX readiness model
- Add non-persisted MLX readiness fields to `RuntimeState`:
  - `mlx_lifecycle_state`
  - `mlx_status_reason`
- These fields describe plugin environment readiness only. They do not replace model readiness from the Models section.
- When the persisted runtime is `mlx`, Steno should trigger an asynchronous MLX bootstrap and health refresh during runtime initialization.
- When the user switches runtime to `mlx`, Steno should persist the selection first, then trigger the same asynchronous refresh.
- Readiness failures should emit Steno runtime errors and update the MLX lifecycle fields to `failed`.

### 4.5 Model selection contract
- Persist `mlx_model_id` alongside the existing runtime settings.
- Default `mlx_model_id` is `tiny.en`.
- The selected MLX model is managed in Steno settings just like Parakeet and Moonshine selections.

### 4.6 Repo-driven plugin gap handling
- The current plugin `transcribeFile` path gates on worker readiness, but does not switch the active worker from `payload.modelId`.
- Steno therefore cannot rely on `transcribeFile(path, modelId)` alone for model selection.
- Before each final MLX transcription, Steno must call `downloadModel(selected_mlx_model)` to ensure the active worker matches the selected model.
- If the selected model is already active, this call should resolve quickly through the plugin's `alreadyActive` path.

### 4.7 Models UI adaptation strategy
- Reuse the existing `ModelDownloadsSnapshot`, `list_model_downloads`, and `start_model_download` flow.
- Add MLX catalog entries into the existing snapshot instead of creating a second model-management surface.
- MLX entries use only these visible statuses:
  - `not_downloaded`
  - `downloading`
  - `ready`
  - `failed`
- MLX does not participate in queue semantics.
- `queue` remains empty for MLX downloads and `cancel_model_download` remains unsupported for MLX entries.
- The UI must not render cancel affordances for MLX models.

### 4.8 MLX readiness persistence for model catalog entries
- The plugin does not expose a model-list or cache-introspection API for all MLX models.
- Steno therefore needs its own persisted MLX readiness ledger for the Models section.
- Store the set of successfully downloaded MLX model ids in a small Steno-owned config file managed by `model_download.rs`.
- On successful MLX download, mark the model `ready` and persist the ledger.
- If a later MLX activation or transcription fails with a plugin not-ready or model-missing condition, demote that model back out of `ready` and persist the correction.

### 4.9 Progress mapping
- Use plugin `PROGRESS` events only for MLX model downloads.
- Map plugin phase and state into Steno status as follows:
  - any `start` or `in_progress` event keeps the model in `downloading`
  - `download` phase `complete` does not mark the model ready yet
  - `preload` phase `complete` marks the model `ready`
  - any `failed` event marks the model `failed`
- Because plugin events report percentage rather than bytes, MLX entries should use synthetic `downloaded_bytes` and `total_bytes` values derived from percent to preserve the existing progress bar without reshaping the snapshot contract.

## 5. Files in Scope

| File | Planned change |
|---|---|
| `/Users/karthik/merge_conflicts/steno/src-tauri/Cargo.toml` | Add `tauri-plugin-shell` and `tauri-plugin-voice` host dependencies; disable voice recorder bridge features |
| `/Users/karthik/merge_conflicts/steno/src-tauri/tauri.conf.json` | Add `bundle.externalBin` entry for `bin/uv`; add `plugins.voice.modelId = tiny.en` |
| `/Users/karthik/merge_conflicts/steno/src-tauri/src/lib.rs` | Register shell and voice plugins; add `set_mlx_model_id` command; keep existing Steno command surface as the frontend boundary |
| `/Users/karthik/merge_conflicts/steno/src-tauri/src/runtime.rs` | Extend runtime/settings state with MLX fields; add bootstrap and health refresh path; branch final transcription to plugin; disable interim/live/incremental behavior for MLX |
| `/Users/karthik/merge_conflicts/steno/src-tauri/src/model_download.rs` | Add MLX catalog entries, MLX readiness ledger, plugin progress listener bridge, and MLX-specific `start_model_download` handling |
| `/Users/karthik/merge_conflicts/steno/src/types.ts` | Extend runtime and model types for MLX runtime, selected MLX model, and MLX lifecycle fields |
| `/Users/karthik/merge_conflicts/steno/src/tauri.ts` | Add `setMlxModelId`; keep frontend bound to Steno commands only |
| `/Users/karthik/merge_conflicts/steno/src/App.tsx` | Add `mlx` runtime option, MLX model selector, MLX readiness callouts, MLX models group, and MLX-specific UI branching for unsupported cancel/interim behavior |

## 6. `src-tauri/Cargo.toml` Changes
- Add `tauri-plugin-shell`.
- Add `tauri-plugin-voice` from the sibling local path.
- Disable `tauri-plugin-voice` default features so Steno does not compile in recorder-bridge ownership it does not intend to use.
- Leave existing native transcriber dependencies intact because Whisper, Parakeet, and Moonshine stay supported.

## 7. `src-tauri/tauri.conf.json` Changes
- Add `bundle.externalBin` with the packaged `uv` sidecar entry.
- Add `plugins.voice.modelId` set to `tiny.en`.
- Do not add recorder-plugin-specific configuration for this task.

## 8. `src-tauri/src/lib.rs` Changes

### Plugin registration
- Register `tauri_plugin_shell::init()` before `tauri_plugin_voice::init()`.
- Keep Steno's own runtime controller and model download manager registered as today.

### Command surface
- Add a `set_mlx_model_id` Tauri command mirroring the current runtime-setting commands.
- Keep the frontend calling Steno commands rather than plugin commands directly.

## 9. `src-tauri/src/runtime.rs` Changes

### 9.1 Runtime and settings structs
- Extend `TranscriptionRuntime` with `Mlx`.
- Extend `RuntimeState` with:
  - `mlx_model_id`
  - `mlx_lifecycle_state`
  - `mlx_status_reason`
- Extend `PersistedSettings` with `mlx_model_id`.
- Default `mlx_model_id` to `tiny.en`.
- `mlx_lifecycle_state` and `mlx_status_reason` are runtime-only fields and should not be persisted.

### 9.2 Initialization flow
- After settings are loaded, if `runtime_selection == mlx`, start an asynchronous MLX readiness refresh.
- The refresh should:
  - set `mlx_lifecycle_state = initializing`
  - clear `mlx_status_reason`
  - call plugin `bootstrap_voice`
  - call plugin `voice_health`
  - update lifecycle fields from the health response
  - emit state changes
  - publish a runtime error if the plugin reports not ready or the bootstrap call fails

### 9.3 Runtime selection setter
- `set_runtime_selection` should persist the new selection first.
- If the new selection is `mlx`, it should trigger the same asynchronous readiness refresh instead of blocking the setter.
- If the new selection is a native runtime, reset MLX lifecycle fields back to `uninitialized` with no error reason.

### 9.4 MLX model setter
- Add `set_mlx_model_id`.
- Reject empty model ids.
- Persist the selected model id and emit updated runtime state.
- Do not auto-download on selection change.
- Do not auto-switch workers on selection change.
- Model activation remains part of MLX final transcription and explicit model-download actions.

### 9.5 Start-recording path
- Keep the existing Steno recording entry flow.
- When `runtime_selection == mlx`, do not start the interim transcription thread.
- When `runtime_selection == mlx`, do not initialize live-word output work for that session.
- Keep the overlay phase flow driven by Steno's own `phase` transitions.

### 9.6 Stop-and-transcribe path
- Keep current stop flow, file finalization, clipboard flow, and completion emission.
- Branch the final transcription path before the native `transcriber_core` route:
  - native runtimes continue using the existing path unchanged
  - `mlx` uses a plugin-backed final transcription branch

### 9.7 MLX final transcription branch
- Reuse the existing captured WAV artifact path produced by Steno.
- Before transcription:
  - verify the selected MLX model entry is `ready` in the Steno model snapshot
  - run the MLX readiness refresh if current lifecycle is not `ready`
  - call plugin `download_model(selected_mlx_model)` to activate the selected worker model
- Then call plugin `transcribe_file` with the WAV path.
- Post-process the returned transcript through the same Steno post-processing path used today.
- Preserve existing `TranscriptionResult` and output flow.
- Set `runtime_used = mlx` and `model_id = mlx_model_id`.

### 9.8 Error mapping
- Map plugin bootstrap, health, model activation, and transcription failures into Steno `RuntimeError`.
- Use existing `publish_error` and recoverable error flows.
- On MLX activation or transcription failure that indicates the selected model is not actually available, instruct `ModelDownloadManager` to demote that model out of `ready`.

### 9.9 Interim and live output suppression
- When `runtime_selection == mlx`, skip:
  - interim transcription emission
  - live-word decoding
  - sentence-boundary incremental commit logic
- This keeps V1 aligned with the agreed final-only behavior.

## 10. `src-tauri/src/model_download.rs` Changes

### 10.1 Catalog extension
- Add MLX catalog entries for the full allowlisted plugin model set:
  - `tiny`
  - `tiny.en`
  - `base`
  - `base.en`
  - `small`
  - `small.en`
  - `medium`
  - `medium.en`
  - `large`
  - `large-v1`
  - `large-v2`
  - `large-v3`
  - `large-v3-turbo`
  - `turbo`
- Use stable keys such as `mlx-<model-id>`.

### 10.2 Internal backend split
- Keep one public snapshot shape.
- Internally distinguish native-download models from MLX plugin-backed models so `start_model_download` can branch by backend.

### 10.3 MLX readiness ledger
- Add a small persisted MLX-ready set under the Steno config directory.
- Load it when the manager initializes.
- Use it to seed MLX entries as `ready` or `not_downloaded` at startup.

### 10.4 MLX `start_model_download`
- For MLX keys:
  - reject if another MLX download is already active
  - mark the entry `downloading`
  - set `active_model_key` to that MLX key
  - keep `queue` empty
  - register a temporary listener for the plugin progress channel
  - spawn the blocking plugin `download_model` call on an async task
  - update the snapshot from incoming `PROGRESS` events
  - on success, mark the model `ready`, persist the ledger, clear `active_model_key`, and emit state
  - on failure, mark the model `failed`, store the error, clear `active_model_key`, and emit state

### 10.5 MLX `cancel_model_download`
- Return a typed unsupported error for MLX keys.
- Leave native queue and cancel behavior unchanged.

### 10.6 Snapshot behavior
- Keep the existing `ModelDownloadsSnapshot` shape.
- For MLX entries:
  - `queue` should remain empty
  - `active_model_key` may point to the current MLX download while active
  - `downloaded_bytes` and `total_bytes` should be derived from plugin percent
  - `speed_bytes_per_sec` should remain zero in V1.

## 11. `src/types.ts` Changes
- Extend `TranscriptionRuntime` with `"mlx"`.
- Extend `RuntimeState` with:
  - `mlx_model_id`
  - `mlx_lifecycle_state`
  - `mlx_status_reason`
- Keep `ModelDownloadsSnapshot` unchanged.
- Keep existing result and error types unchanged except for allowing `runtime_used = "mlx"`.

## 12. `src/tauri.ts` Changes
- Add `setMlxModelId(modelId: string)`.
- Keep all MLX operations flowing through Steno commands.
- Do not add direct frontend wrappers for plugin commands in this task.

## 13. `src/App.tsx` Changes

### 13.1 Runtime selector
- Add `mlx` to the runtime dropdown.

### 13.2 MLX settings block
- Add an MLX model selector that appears only when `runtime_selection == mlx`.
- Populate it from the `mlx` subset of `modelDownloads.models`.
- If the selected MLX model is not `ready`, show the existing “Open Models Section” warning pattern.

### 13.3 MLX readiness messaging
- Show a Steno-native readiness callout when `runtime_selection == mlx` and `mlx_lifecycle_state != ready`.
- Use `mlx_status_reason` as the message body when present.
- Do not add a new plugin-specific panel or event stream.

### 13.4 Interim/live preview behavior
- Hide interim preview and live-word preview panels when `runtime_selection == mlx`.
- Replace them with no additional MLX-specific preview surface in V1.

### 13.5 Models section
- Include MLX models in the existing Models section.
- Render MLX cards in the same grid as other models.
- Adjust labels so MLX entries read as model ids rather than as Whisper profiles or non-MLX “version” labels.
- Do not render cancel buttons for MLX entries.
- Continue using the existing retry and download buttons.

### 13.6 Command follow-up
- After runtime or model-setting commands, continue refreshing `runtimeState` from `getRuntimeState`.
- After MLX model-download actions, refresh both `modelDownloads` and `runtimeState` so MLX readiness callouts stay current.

## 14. Implementation Sequence
1. Wire the voice plugin into the Steno Rust host and package configuration.
2. Extend Steno runtime and persisted settings with `mlx` and `mlx_model_id`.
3. Add MLX lifecycle fields and asynchronous readiness refresh behavior.
4. Extend the model-download manager with MLX catalog entries, progress adaptation, and the readiness ledger.
5. Add the `set_mlx_model_id` command and frontend bindings.
6. Update the settings UI and Models UI for MLX selection, status display, and unsupported cancel behavior.
7. Branch the final transcription path to use plugin activation plus plugin file transcription for `mlx`.
8. Suppress interim, live-word, and incremental chunk behavior for `mlx`.
9. Run validation focused on persistence, readiness, download status mapping, and final-only transcription behavior.

## 15. Validation Plan
- Select `mlx` in the UI and verify Steno persists the runtime selection and begins MLX readiness refresh immediately.
- Restart Steno with persisted `mlx` selection and verify readiness refresh runs during initialization.
- Verify the default selected MLX model is `tiny.en` on first use.
- Verify the Models section shows the full MLX allowlist.
- Start an MLX model download and verify statuses move through `downloading` to `ready` or `failed`.
- Verify MLX entries do not show queue or cancel behavior.
- Record with `mlx` selected and verify no interim preview, live-word output, or incremental chunk commits occur.
- Stop recording with `mlx` selected and verify Steno activates the selected MLX model, transcribes through the plugin, and emits the normal completion flow.
- Force plugin bootstrap or transcription failure and verify Steno surfaces a recoverable runtime error.
- Verify native runtimes still use the existing Rust-native transcription path unchanged.

## 16. Out of Scope for This LLD
- Any edits inside `/Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-voice`
- Overlay redesign or overlay bug fixes
- Recorder takeover inside Steno
- Plugin event mapping beyond MLX download progress
- Queue, multi-download orchestration, or cancel support for MLX
