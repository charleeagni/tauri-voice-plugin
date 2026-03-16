# Low Level Design: Load STT model into memory at app startup (CODIN-254)

## 1. Objective
Enable automated model loading at plugin initialization, ensuring the model is resident in memory before the app transitions to its ready state.

## 2. Parent Context
- Parent Task: CODIN-254
- HLD: `spec/Tauri Recorder/CODIN-254/HLD.md`

## 3. Schema Changes

### `tauri-plugin-stt/src/models.rs`

Define a plugin configuration struct:

```rust
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    /// Whisper model ID to load at startup. Defaults to "tiny.en".
    pub model_id: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model_id: Some("tiny.en".to_string()),
        }
    }
}
```

## 4. Internal Logic Changes

### `tauri-plugin-stt/src/desktop.rs`

1. **Update `TauriPluginStt` State**:
   - Add `startup_error: Arc<Mutex<Option<String>>>` to the struct to persist any error that occurs during the automated startup bootstrap.

2. **Implement `auto_bootstrap`**:
   - Signature: `pub fn auto_bootstrap(&self, config: &Config)`
   - Implementation:
     - Use `tauri::async_runtime::block_on` to call `crate::bootstrap_manager::BootstrapManager::bootstrap_stt`.
     - If bootstrap fails, log the error and store it in `self.startup_error`.
     - If bootstrap succeeds, call `self.spawn_worker(&model_id)` where `model_id` is derived from config or `DEFAULT_MODEL`.
     - If spawning fails, log the error and store it in `self.startup_error`.
     - If spawning succeeds, store the worker in `self.worker`.

3. **Refine `stt_health`**:
   - Check `self.startup_error` at the beginning of the diagnostics aggregation.
   - If an error exists, return `HealthResponse::NotReady` with the startup error message.

### `tauri-plugin-stt/src/lib.rs`

1. **Update `init` function**:
   - Change `Builder::new` to `Builder::<R, Config>::new`.
   - Update the `setup` hook:
     - Access `api.config()`.
     - Call `tauri_plugin_stt.auto_bootstrap(config)`.
     - This will block the app startup until `auto_bootstrap` finishes (either successfully or with an error).

2. **Update `desktop::init`**:
   - Update signature to accept `PluginApi<R, Config>`.

## 5. Error Handling & Diagnostics
- **Startup Errors**: Persisted in `TauriPluginStt` state and reported via `stt_health`.
- **Timeouts**: `spawn_worker` already has a timeout (300s). This will block startup for up to 5 minutes if the model download is extremely slow or hung, which is acceptable under the "ready only after resident" requirement.

## 6. Implementation Plan

### Phase 1: Models and Config
- Add `Config` struct to `models.rs`.
- Ensure it is exported.

### Phase 2: Desktop Refinement
- Update `TauriPluginStt` struct in `desktop.rs`.
- Implement `auto_bootstrap` in `desktop.rs`.
- Update `stt_health` in `desktop.rs`.

### Phase 3: Plugin Lifecycle
- Update `init` and `setup` hook in `lib.rs`.
- Update `desktop::init` signature.

### Phase 4: Validation
- Start the example app.
- Verify that it takes some time to start (if model needs download).
- Verify that `stt_health` returns `Ready` immediately.
- Verify that `transcribe_file` works without loading delay.
