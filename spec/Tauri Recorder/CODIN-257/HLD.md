# HLD — CODIN-257: Expose Model Download & Preload Progress Events

## Problem

`worker.py` silently downloads model weights from HuggingFace and loads them into memory before emitting `{"status": "ready"}`. This phase can take minutes. The UI has no visibility into what is happening or how far along it is.

Additionally, the current design ties model selection to `bootstrap_stt`, which means there is no way to switch models after initial setup.

## Goals

- Bootstrap always installs `tiny.en`; model choice at install time is removed.
- A new `download_model` command allows on-demand install of any allowlisted model.
- Both paths emit structured progress events so the UI can display download and preload status.

---

## Architecture: Three-Layer Propagation Chain

```
worker.py stdout (JSON lines)
  → Rust reader thread (desktop.rs)
    → Tauri event: tauri-audio-plugin://model-progress
      → guest-js listener (index.ts)
```

---

## Commands

### Existing: `bootstrap_stt`

- Hardcodes `tiny.en`; `model_id` field is **dropped** from `BootstrapRequest`.
- Semantics unchanged: blocks until worker is ready to transcribe.
- Emits progress events during worker startup.

### New: `download_model(model_id)`

- Validates `model_id` against the allowlist; returns error if invalid.
- Fail-fast if a download/load is already in progress — returns a clear error to the caller.
- Tears down the existing worker if it is loaded with a different model; spawns a new worker with `model_id`.
- Returns only after the worker emits `{"status": "ready"}` (Option A — blocks until fully ready to transcribe).
- Idempotent: if the worker is already loaded with the same `model_id`, returns success immediately.
- Emits progress events during worker startup.

---

## Progress Event Contract

**Channel**: `tauri-audio-plugin://model-progress`

```
phase:  "download" | "preload"
state:  "start" | "in_progress" | "complete" | "failed"
```

- `percent`: `0.0–1.0` during active download; `null` when unavailable (e.g. cached model).
- `filename`: the file currently being fetched; `null` during preload phase.
- `error`: present only on `"failed"` state; `null` otherwise.
- Behavior is deterministic when metadata is unavailable: emits `start` then `complete` with `percent: null`.

---

## Layer Responsibilities

### Python (`worker.py`)

- Intercepts `tqdm` used internally by `huggingface_hub` via a custom subclass that serializes progress to stdout as JSON lines with `"status": "progress"`.
- Emits explicit `download.start` / `download.complete` markers around the download phase.
- Emits `preload.start` / `preload.complete` markers around the memory-load phase.
- Fallback: if model is cached and no tqdm activity fires, still emits `start` + `complete` for each phase.

### Rust (`desktop.rs`)

- Reader thread already reads worker stdout line by line.
- Lines with `"status": "progress"` are parsed and emitted as `tauri-audio-plugin://model-progress` Tauri events; they are **not** forwarded to the `rx` channel.
- All other lines (including `{"status": "ready"}`) continue to `rx` unchanged.
- `bootstrap_stt` always passes `"tiny.en"` to `spawn_worker`.
- `download_model` acquires a download-in-progress lock; fails fast if already held. Tears down and respawns worker if model differs.

### TypeScript (`guest-js/index.ts`)

- `DownloadModelRequest` / `DownloadModelResponse` types.
- `downloadModel(modelId)` command wrapper.
- `ModelProgressEvent` interface.
- `CHANNELS.PROGRESS` constant.
- `listenModelProgress(callback)` helper.

---

## Key Decisions

| Decision | Choice |
|---|---|
| `BootstrapRequest.model_id` | Dropped |
| Bootstrap default model | Always `tiny.en` |
| `download_model` completion | Blocks until worker ready (Option A) |
| Concurrent `download_model` calls | Fail fast with "already downloading" error |
| Progress when model is cached | Emit `start` + `complete` with `percent: null` |
