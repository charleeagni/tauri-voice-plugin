# High Level Design: Enforce startup-preloaded STT lifecycle contract (CODIN-255)

## 1. Objective
Refine and formally enforce the explicit lifecycle contract for the Speech-to-Text (STT) plugin. The contract guarantees that the model is preloaded into memory at startup, allowing transcribe calls to strictly assume the STT model is ready, eliminating lazy initialization overhead.

## 2. Current State vs Required State
**Current:**
- The plugin automatically bootstraps at startup via `.setup()` in `lib.rs` -> `auto_bootstrap`.
- The `sttHealth` command returns a binary `Ready` or `NotReady` based on whether the uv environment exists and the worker process has started.
- Transcribe calls fail if the worker is not running, but the lifecycle states aren't well documented or formally typed in the API.

**Required:**
- The API explicitly exposes formal lifecycle states: `uninitialized`, `initializing`, `startup_ready`, and `transcribe_ready`.
- Documentation reflects that applications MUST wait for the model to be loaded before issuing transcription requests.
- The transcription path assumes a preloaded runtime and uniformly fails with a typed error if the lifecycle state is not ready.

## 3. Proposed Changes

### 3.1. Explicit Lifecycle States Data Model
Enhance the internal state and the TypeScript/Rust API to clearly surface the lifecycle states rather than just `Ready`/`NotReady`. We will define an explicit `LifecycleState` enum:
- `uninitialized`: Initial state prior to bootstrap.
- `initializing`: Environment is being configured, model is downloading, or model is preloading into memory.
- `startup_ready` / `transcribe_ready`: Worker is spawned and model is loaded into memory.
- `failed`: An error occurred during startup preload.

### 3.2. Typescript / Guest API Updates (`guest-js/index.ts`)
1. Introduce a `LifecycleState` enum/type to the contract.
2. Update `HealthResponse` to include `lifecycleState`.
3. Improve JSDoc comments to explicitly declare the preload requirement for the `transcribeFile` function, noting it will fail immediately with a `notReady` error if the worker is not preloaded.

### 3.3. Rust Implementation (`models.rs` and `desktop.rs`)
1. Extend `HealthResponse` in `models.rs` to include the explicit `lifecycle_state`.
2. Ensure `stt_health()` categorizes its output correctly (e.g., if `download_in_progress` is true, state is `Initializing`, if `startup_error` is Some, state is `Failed`, else if `worker` is Some, state is `TranscribeReady`).
3. In `transcribe_file_internal()`, upon `stt_health()` returning not ready, return a typed `Error::NotReady`. (This behaviour largely exists but will be strictly coupled to the explicit lifecycle state).

### 3.4. Documentation (`USAGE.md`)
Add a section titled `Lifecycle Contract` outling the states and the paradigm:
- STT does not use lazy initialization for transcriptions.
- The consumer should query `sttHealth` or listen for `PROGRESS` events to determine when the STT plugin transitions to readiness.

## 4. Verification
- `sttHealth` will be polled during startup to confirm the lifecycle transitions from `initializing` -> `startup_ready`.
- `transcribeFile` will be invoked before `startup_ready`, validating that it rejects immediately with a `notReady` error instead of implicitly starting a python context.
