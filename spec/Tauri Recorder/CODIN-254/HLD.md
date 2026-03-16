# High Level Design: Load STT model into memory at app startup (CODIN-254)

## 1. Objective
The transcriber plugin must load the configured STT model into memory during app startup. The app startup sequence should transition to "ready" only after the model is memory-resident.

## 2. Parent Context
- Module: Transcriber Plugin
- Parent Task: CODIN-253 (Implicitly, as this is a refinement of the startup/initialization flow).
- Acceptance Criteria:
    - Startup sequence transitions to ready only after model is memory-resident.
    - First transcription request does not perform initial model load.
    - Startup failure state is explicit when in-memory model load cannot complete.
- Clarification:
    - Model artifacts should be pre-downloaded (or downloaded during bootstrap).
    - Runtime must use a long-lived STT worker session.
    - Model must be loaded once at startup and reused.

## 3. Problem Context
The current implementation of the STT plugin initializes with no active worker. The worker (and thus the model) is only spawned and loaded when the `bootstrap_stt` or `download_model` commands are explicitly called by the frontend. This results in:
1. Lazy loading: the first transcription request might be delayed if the frontend didn't call bootstrap earlier.
2. Inconsistent startup state: the app might appear "ready" but the STT capability is not yet memory-resident.

## 4. Scope
In scope:
- Plugin configuration support for `model_id` in `tauri.conf.json`.
- Automatic execution of bootstrap and worker spawning during Tauri's plugin `setup` hook.
- Blocking startup until the worker is "ready" (model loaded).
- Enhanced `stt_health` to report explicit startup failure states.

Out of scope:
- Re-implementing `BootstrapManager` or `spawn_worker` (will reuse/refactor existing logic).
- Frontend UI for startup progress (handled by Tauri's native splash screen or similar if used, but plugin logic is backend-focused).

## 5. Proposed Solution
1. **Plugin Configuration**:
   - Define a `Config` struct that allows specifying `model_id`.
   - Use Tauri's plugin configuration mechanism to read this from `tauri.conf.json`.

2. **Auto-Bootstrap Logic**:
   - Add an `auto_bootstrap` method to `TauriPluginStt`.
   - This method will:
     - Call `BootstrapManager::bootstrap_stt` to ensure venv and dependencies are present.
     - Call `spawn_worker` with the configured `model_id` (defaulting to `tiny.en` if not specified).
     - Store any initialization error in a new `startup_error` field in `TauriPluginStt`.

3. **Tauri Plugin Setup Hook**:
   - In `lib.rs`, modify the `Builder::setup` hook to call `auto_bootstrap`.
   - Since `setup` is called during app initialization, waiting for `auto_bootstrap` to complete will satisfy the requirement that the startup sequence transitions to ready only after the model is resident.

4. **Health Check Refinement**:
   - Update `stt_health` to check the `startup_error` field.
   - If `startup_error` is set, report it as a `NotReady` state with an explicit reason.

## 6. High-Level Component View
- `Config`: Data structure for `tauri.conf.json` plugin settings.
- `TauriPluginStt`: Extended with `startup_error: Arc<Mutex<Option<String>>>` and `auto_bootstrap` method.
- `init`: Updated to use the `setup` hook for automated model loading.

## 7. Deliverables for CODIN-254
- `tauri-plugin-stt/src/models.rs`: Added `Config` struct.
- `tauri-plugin-stt/src/lib.rs`: Updated `init` and `setup` hook.
- `tauri-plugin-stt/src/desktop.rs`: Implemented `auto_bootstrap` and updated `TauriPluginStt` state.
- `tauri-plugin-stt/src/error.rs`: (If needed) Added specific startup error variants.

## 8. Non-Functional Expectations
- **Reliability**: Ensure timeouts are handled so a hung worker doesn't block the app forever (though the requirement says it *must* load, a timeout is still necessary for robustness).
- **Readability**: Maintain clean separation between the bootstrap manager (setup) and the worker lifecycle (memory residency).

## 9. Risks and Mitigations
- **Risk**: Slow first-time startup if the model needs to be downloaded.
  - **Mitigation**: This is an inherent part of the requirement ("transitions to ready only after model is memory-resident"). We will ensure the loading is explicit and errors are clear.
- **Risk**: Blocking the main thread.
  - **Mitigation**: Tauri's `setup` hook runs during the initialization phase. While it blocks the app from being "ready" to the user, this is exactly what the requirement asks for.

## 10. Success Criteria
- Upon app startup, a Python worker is automatically spawned and the STT model is loaded into memory.
- The app only becomes "ready" (e.g., window shows up or frontend can call commands) after the worker is in its "ready" state.
- Calling `transcribe_file` immediately after startup succeeds without further delay.
- If model loading fails, `stt_health` reports the specific error encountered during startup.
