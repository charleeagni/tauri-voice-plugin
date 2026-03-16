# High Level Design: Non-Blocking Model Preloading at Application Startup (CODIN-269)

## 1. Objective
Change the STT plugin startup contract so application launch is never blocked by Python environment setup, model download, or model preload. The plugin should initialize immediately, schedule preload work in the background, and remain not ready until that work completes successfully.

## 2. Current State
The current startup path in [tauri-plugin-stt/src/lib.rs](/Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/src/lib.rs) calls `auto_bootstrap` during the plugin `setup()` hook. The current `auto_bootstrap` implementation in [tauri-plugin-stt/src/desktop.rs](/Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/src/desktop.rs) blocks while:
- creating or validating the Python environment
- spawning the worker
- waiting for the worker ready signal

That behavior matches CODIN-254, but it conflicts with CODIN-269 because the UI cannot become interactive until preload finishes.

## 3. Required Behavior
- Plugin setup returns without waiting for preload.
- Startup preload begins automatically in a background task after plugin initialization.
- `stt_health` remains the source of truth for readiness.
- `stt_health` reports startup progress through lifecycle state and diagnostics.
- The preload state exposed by `stt_health` must be usable by the frontend to render loading, failure, and ready UI states.
- Background preload failures are persisted and visible through `stt_health`.
- `transcribe_file` does not trigger lazy bootstrap or lazy worker creation.
- Recorder and hotkey-driven flows must not start the record-transcribe pipeline until STT preload is fully ready.

## 4. Scope
In scope:
- Replace blocking startup preload with background startup preload.
- Track startup preload state explicitly in plugin-managed state.
- Refine `stt_health` so readiness reflects background preload progress and failure.
- Make preload readiness explicit enough for frontend decision-making.
- Gate pipeline setup and hotkey-triggered execution on STT readiness.
- Keep transcription gated on successful preload completion.

Out of scope:
- New frontend UI flows.
- New public commands for preload control.
- Automatic retries or restart loops after startup preload failure.
- Redesign of worker wire protocol or recorder bridge behavior.

## 5. Proposed Solution

### 5.1 Startup Task Model
Plugin setup should only construct state and schedule a background preload task. The background task performs the same two major steps as today:
- environment bootstrap
- worker spawn and model preload

The difference is execution timing. The task runs after setup returns instead of inside setup.

### 5.2 Explicit Startup Preload State
The plugin should maintain a dedicated preload status instead of inferring startup progress only from `download_in_progress`, worker presence, or a startup error string. A dedicated state is needed because:
- startup preload is different from an explicit `download_model` request
- the app may be initializing in the background while no worker exists yet
- failure must remain visible after the background task exits

The state should cover four cases:
- idle: startup preload has not started yet
- running: startup preload is currently executing
- ready: startup preload completed and worker is memory-resident
- failed: startup preload ended with an error

### 5.3 Health Contract
`stt_health` remains the canonical diagnostic entry point.

Expected mapping:
- preload running: `NotReady` with lifecycle `initializing`
- preload failed: `NotReady` with lifecycle `failed`
- preload ready and all diagnostics pass: `Ready` with lifecycle `ready`
- preload not started or worker absent with no active preload: `NotReady` with lifecycle `uninitialized`

Diagnostics should explicitly include startup preload state so callers can distinguish:
- environment not ready yet
- preload still in progress
- preload failed
- worker unavailable after preload

This health contract is not only for diagnostics. It is also the app-facing readiness contract that the frontend can poll before:
- enabling STT-dependent actions
- arming hotkey-driven recording flows
- deciding whether to show loading, retry, or ready UX

### 5.4 Transcription Contract
`transcribe_file` should continue to fail fast when `stt_health` reports not ready. No lazy worker spawn should be introduced. CODIN-269 changes startup timing, not the readiness gate itself.

### 5.5 Pipeline and Hotkey Gating
The startup contract must also block recorder-adjacent flows that would eventually depend on STT:
- `setup_record_transcribe_pipeline` must not activate the pipeline unless STT is ready.
- Hotkey-triggered record-and-transcribe behavior must remain effectively blocked until STT preload completes.
- The app must be able to inspect readiness before enabling or listening to hotkeys that would immediately enter the pipeline.

This avoids a degraded UX where the app appears interactive but the first hotkey-triggered flow fails because background preload is still running.

### 5.6 Compatibility with Existing Commands
- `bootstrap_stt` remains available as an explicit command path.
- `download_model` remains available for explicit model switching.
- Background startup preload should not race unsafely with those commands; shared state must prevent duplicate startup work or contradictory status reporting.

## 6. Component Impact

### [tauri-plugin-stt/src/lib.rs](/Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/src/lib.rs)
- Stop blocking inside plugin `setup()`.
- Schedule startup preload on the async runtime after plugin state is created.

### [tauri-plugin-stt/src/desktop.rs](/Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/src/desktop.rs)
- Replace blocking startup logic with background-safe startup orchestration.
- Introduce explicit startup preload status storage.
- Persist startup failure reason separately from transient activity flags.
- Refine health reporting based on preload status.
- Ensure pipeline setup and STT-dependent hotkey activation remain gated on ready health.

### [tauri-plugin-stt/src/models.rs](/Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/src/models.rs)
- No major contract expansion is required if the current lifecycle enum remains sufficient.
- Diagnostic naming may need a narrow update to make startup preload state visible.

## 7. Risks and Mitigations
- Race between background preload and manual bootstrap:
  Use shared startup status and existing worker mutexes to ensure only one effective startup worker creation path succeeds at a time.
- Ambiguous health during background work:
  Add a dedicated preload diagnostic instead of relying only on `download_in_progress`.
- Stale failure state after recovery:
  Clear startup failure when a later preload or explicit bootstrap succeeds.
- Hidden readiness regression:
  Keep `transcribe_file` dependent on `stt_health` so behavioral gating stays centralized.
- Frontend makes incorrect enablement decisions:
  Treat `stt_health` as the canonical app-facing readiness contract and require pipeline/hotkey code to use it.

## 8. Success Criteria
- The application window and UI can launch immediately even when no environment or model is ready yet.
- Startup preload begins automatically without frontend action.
- `stt_health` reports `initializing` while preload runs.
- `stt_health` reports `failed` with a clear reason if preload fails.
- `stt_health` reports `ready` only after the worker is alive and all diagnostics pass.
- Frontend can reliably use `stt_health` to decide whether STT-dependent controls should be enabled.
- Pipeline setup and hotkey-driven execution stay blocked until preload reaches ready.
- `transcribe_file` continues to reject requests until preload finishes successfully.
