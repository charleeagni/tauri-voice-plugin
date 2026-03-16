# Low Level Design: Non-Blocking Model Preloading at Application Startup (CODIN-269)

## 1. Objective
Implement CODIN-269 by moving startup preload off the blocking plugin setup path while preserving the existing strict readiness gate for transcription.

## 2. Parent Context
- Parent task: CODIN-269
- HLD: [spec/tauri-plugin-stt/CODIN-269/HLD.md](/Users/karthik/merge_conflicts/voice_plugin/spec/tauri-plugin-stt/CODIN-269/HLD.md)

## 3. Files in Scope
- [tauri-plugin-stt/src/lib.rs](/Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/src/lib.rs)
- [tauri-plugin-stt/src/desktop.rs](/Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/src/desktop.rs)
- [tauri-plugin-stt/src/models.rs](/Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/src/models.rs) only if diagnostic naming or lifecycle mapping requires a narrow contract update
- [tauri-plugin-stt/src/commands.rs](/Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/src/commands.rs) only if recorder-facing command wrappers need readiness gating aligned with `desktop.rs`

## 4. State Changes

### Startup preload status
Add a dedicated startup preload status field to `TauriPluginStt`. This should be stored independently from:
- `worker`
- `download_in_progress`
- `startup_error`

The status needs four values:
- `Idle`
- `Running`
- `Ready`
- `Failed`

This state is specifically for automatic startup preload. It should not be overloaded with explicit `download_model` progress.

### Startup error ownership
Keep `startup_error`, but make it the persisted failure reason for the startup preload path only. When startup preload or a later explicit bootstrap succeeds, clear the stored startup error.

## 5. `lib.rs` Changes

### Setup flow
In plugin setup:
- create the plugin state as today
- read the configured model id
- schedule startup preload using `tauri::async_runtime::spawn`
- return from setup immediately
- manage the plugin state without waiting for preload completion

The spawned task should call a dedicated startup preload method on `TauriPluginStt` instead of reusing the current blocking `auto_bootstrap` shape.

## 6. `desktop.rs` Changes

### Replace `auto_bootstrap`
Replace the current synchronous `auto_bootstrap(&self, config: &Config)` behavior with a startup preload entry point designed for background execution.

Required flow:
1. Resolve effective `model_id`.
2. Transition startup preload status from `Idle` to `Running`.
3. Clear any previous startup error before work begins.
4. Run bootstrap manager setup asynchronously.
5. On bootstrap failure:
   set startup preload status to `Failed`
   store the formatted error in `startup_error`
   stop
6. Spawn the worker for the target model.
7. On worker spawn failure:
   set startup preload status to `Failed`
   store the formatted error in `startup_error`
   stop
8. On success:
   store the worker
   clear `startup_error`
   set startup preload status to `Ready`

### Duplicate work prevention
The startup preload method should no-op when preload is already `Running` or `Ready`. This prevents duplicate background runs during repeated setup-like call paths.

### Recovery behavior
If explicit `bootstrap_stt` or explicit worker creation succeeds after a startup preload failure, clear `startup_error` and transition startup preload status to `Ready`. This keeps `stt_health` from reporting a stale failure after recovery.

### `bootstrap_stt` interaction
`bootstrap_stt` currently bootstraps the environment and spawns the default worker if none exists. Keep that behavior, but align state updates with the startup preload state machine:
- if bootstrap begins while startup preload is running, do not corrupt status
- if bootstrap succeeds and a worker exists, mark startup preload as `Ready`
- if bootstrap fails through the explicit command path, do not overwrite startup failure unless the failure belongs to the startup task

### `download_model` interaction
`download_model` should continue to manage explicit model changes through `download_in_progress`. If it succeeds, startup preload status should end in `Ready` because the worker is again memory-resident. If it fails, existing explicit command behavior remains unchanged unless the failure occurred during startup preload.

## 7. `stt_health` Changes

### Diagnostic shape
Add a startup preload diagnostic entry before worker readiness evaluation. Suggested diagnostic name:
- `startup_preload`

Expected reasons:
- running: startup preload is still in progress
- failed: startup preload failed with stored error
- idle/uninitialized: startup preload has not produced a ready worker yet
- ready: no reason

### Lifecycle mapping
Use startup preload status and existing checks together:
- `Failed` status returns `NotReady` with lifecycle `failed`
- `Running` status returns `NotReady` with lifecycle `initializing`
- all diagnostics ready and worker present returns `Ready` with lifecycle `ready`
- everything else returns `NotReady` with lifecycle `uninitialized`

### Worker diagnostic text
Update the `stt_worker` failure reason so it no longer tells the caller to invoke `bootstrapStt` as the primary path. The startup contract is now automatic background preload, so the reason should explain that the worker is not ready yet or startup preload has not completed.

### App-facing readiness usage
Treat `stt_health` as the app-facing readiness API for CODIN-269. The frontend should be able to call it and make a deterministic decision:
- `initializing`: show loading and keep STT-dependent actions disabled
- `failed`: show failure or retry affordance and keep STT-dependent actions disabled
- `ready`: enable STT-dependent flows
- `uninitialized`: treat as not available and keep STT-dependent actions disabled

No separate preload-status command is required in this task if `stt_health` exposes the full decision surface.

## 8. Pipeline and Hotkey Gating

### `setup_record_transcribe_pipeline`
Keep the existing readiness check in `setup_record_transcribe_pipeline`, but make its intent explicit in implementation and validation:
- the command must refuse activation while startup preload is `initializing`, `failed`, or otherwise not ready
- the command should only register and activate the record-transcribe pipeline once STT health is ready

This keeps the pipeline from arming a path that will fail on first use.

### Hotkey-triggered execution
Hotkey-triggered record-and-transcribe behavior should remain blocked until STT is ready. For CODIN-269, that means:
- do not treat UI interactivity as pipeline readiness
- do not allow STT-dependent hotkey execution to proceed based only on recorder readiness
- rely on the same centralized STT readiness gate used by `transcribe_file`

If command wrappers in `commands.rs` expose hotkey or pipeline actions without sufficient guarding, add narrow readiness checks there rather than introducing a new abstraction.

## 9. Validation Plan
- Launch the example app and confirm UI startup is immediate even on a cold environment.
- Poll `stt_health` during startup and verify `notReady` with lifecycle `initializing`.
- Wait for preload success and verify `ready` with all diagnostics passing.
- Force a startup bootstrap or worker failure and verify `notReady` with lifecycle `failed` plus persisted reason.
- Verify the frontend can derive disabled/loading/ready behavior from `stt_health` without extra state.
- Call `setup_record_transcribe_pipeline` during background preload and verify it refuses activation.
- Verify hotkey-triggered record-transcribe behavior cannot proceed before STT preload is ready.
- Call `transcribe_file` during background preload and verify immediate typed not-ready behavior.
- After successful preload, call `transcribe_file` and verify no lazy setup occurs in the request path.

## 10. Out of Scope for This LLD
- New progress event contracts
- Retry loops
- Cancellation controls for startup preload
- Frontend subscription logic beyond consuming `stt_health`
