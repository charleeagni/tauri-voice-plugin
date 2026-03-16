# HLD — CODIN-256: Persistent STT Runtime Session Contract

## Problem

The transcription plugin spawns a new Python process for every `transcribe_file` call in the naive (pre-existing) design. This couples model initialization latency to every transcription request. The goal of this work item is to **define and document the canonical contract** for a warm-runtime STT session where model initialization is a one-time cost and the worker process is long-lived.

A persistent worker exists in the current codebase (`SttWorker` in `desktop.rs`), but the session lifecycle — including degraded, restart, and shutdown semantics — is not formally specified anywhere.

---

## Goals

- Define the six worker session states and their legal transitions.
- Specify the wire protocol between the Rust host and the Python worker.
- Specify what triggers each lifecycle transition and who is responsible.
- Define how `stt_health` maps onto session state.
- Establish this document as the canonical reference for warm-runtime transcription behavior.

---

## Session Lifecycle

The worker session moves through six states:

```
Startup → Ready → Active ↔ Ready
                    ↓
                Degraded → Restarting → Ready
                    ↓
                Shutdown
```

### State Definitions

| State | Description |
|---|---|
| `Startup` | Worker process has been spawned; waiting for `{"status": "ready"}` from stdout. Model is being downloaded and loaded into memory. |
| `Ready` | Worker emitted `{"status": "ready"}`; model is in memory; stdin is writable; the session can accept transcription requests. |
| `Active` | A transcription request has been written to stdin; the worker mutex is held; waiting for the response line on `rx`. |
| `Degraded` | A stdin write or flush failed, or the `rx` channel returned a timeout/disconnect error during `Active`. The worker process is presumed dead. `stt_health` returns `NotReady { stt_worker: false }`. |
| `Restarting` | The current `SttWorker` value has been dropped (replacing `Option<SttWorker>` with `None`) and a new `spawn_worker` call is in flight. Triggered explicitly by `download_model` with a different model ID, or implicitly on future restart policy. |
| `Shutdown` | The `TauriPluginStt` struct is dropped; the `Arc<Mutex<Option<SttWorker>>>` reference count reaches zero; the child process is dropped and the OS reclaims it. |

---

## Startup Sequence

1. `bootstrap_stt` or `download_model` calls `spawn_worker(model_id)`.
2. Rust spawns the Python process with `--model <repo>` and captures `stdin`/`stdout`.
3. A background reader thread drains `stdout`:
   - Lines with `"status": "progress"` are parsed and re-emitted as `tauri-audio-plugin://model-progress` Tauri events.
   - All other lines are forwarded to an `mpsc::channel` (`rx`).
4. `spawn_worker` blocks on `rx.recv_timeout(300s)`, consuming and skipping unrecognised lines until it sees `{"status": "ready"}`.
5. On receipt of `ready`, `spawn_worker` returns `SttWorker { child, stdin, rx, model_id }`.
6. The worker is stored in `Arc<Mutex<Option<SttWorker>>>`.

**Startup timeout**: 300 seconds. If `ready` is not received within this window, `spawn_worker` returns `Error::not_ready("STT worker startup timeout")`.

---

## Wire Protocol

All messages are newline-delimited JSON. The worker reads from `stdin` and writes to `stdout`.

### Rust → Python (requests)

```
{"id": "<req_id>", "audio": "<absolute_path>"}
```

- `id`: Caller-generated opaque string (format: `req-<unix_ms>`).
- `audio`: Absolute path to a WAV or compatible audio file.

### Python → Rust (responses)

Success:
```
{"id": "<req_id>", "text": "<transcript>", "error": null}
```

Failure:
```
{"id": "<req_id>", "text": null, "error": "<message>"}
```

### Python → Rust (startup and progress)

Ready signal (one-time, after model load):
```
{"status": "ready", "model": "<model_repo>"}
```

Progress events (during `Startup` and `Restarting`):
```
{"status": "progress", "phase": "download"|"preload",
 "state": "start"|"in_progress"|"complete"|"failed",
 "model": "<model_repo>", "percent": 0.0–1.0|null,
 "filename": "<file>"|null, "error": "<msg>"|null}
```

Progress lines are intercepted by the reader thread and **never forwarded to `rx`**.

---

## Degraded Behavior

The worker enters an implicit `Degraded` state when any of the following occur during `Active`:

- `writeln!` to `worker.stdin` returns an error.
- `worker.stdin.flush()` returns an error.
- `worker.rx.recv_timeout(120s)` returns `Err` (timeout or channel closed).

**Current behavior**: The caller receives `Error::generation_failed(...)` and the `Option<SttWorker>` in the mutex is left populated with a now-broken handle. Subsequent `stt_health` calls will still report `stt_worker: true` even though the worker is dead.

**Contracted behavior** (to be implemented): On any `Degraded` trigger, the `Option<SttWorker>` must be set to `None` before returning the error. This ensures `stt_health` correctly reflects `NotReady { stt_worker: false }` and the caller can reason about session state.

---

## Restart Semantics

A restart is initiated by replacing the worker slot with a new `SttWorker`:

1. Acquire the worker mutex.
2. Drop the current `SttWorker` (child process is dropped; OS reaps the process).
3. Set the slot to `None`.
4. Call `spawn_worker(model_id)` to produce a new `SttWorker`.
5. Store the new worker in the slot.

**Explicit restart trigger**: `download_model(model_id)` when the requested model differs from the currently loaded one.

**Implicit restart trigger**: Future policy — not in scope for this work item. Callers can recover by calling `bootstrap_stt` or `download_model` again after receiving a `not_ready` error.

**Concurrent restart guard**: `download_in_progress: Arc<Mutex<bool>>` prevents two concurrent `download_model` calls from racing on the worker slot.

---

## Shutdown Semantics

Shutdown is implicit and unordered:

- When `TauriPluginStt` is dropped (plugin teardown), the `Arc` reference count on the worker slot falls to zero.
- The `Option<SttWorker>` is dropped, which drops `SttWorker`, which drops `child`.
- The Python worker process receives `SIGCHLD`/EOF on its stdin and exits its request loop.

There is no graceful shutdown message sent to the worker before drop. The Python process will exit when stdin reaches EOF.

---

## Health Mapping

`stt_health` maps diagnostic checks to session state as follows:

| Diagnostic | Session Implication |
|---|---|
| `app_data_dir: false` | Cannot reach `Startup`; plugin environment broken. |
| `uv_sidecar: false` | Cannot reach `Startup`; bootstrap not yet run. |
| `python_venv: false` | Cannot reach `Startup`; bootstrap not yet run. |
| `python_binary: false` | Cannot reach `Startup`; bootstrap partially complete. |
| `stt_worker: false` | Session is `None` — either never started, or entered `Degraded`. |
| `stt_worker: true` | Session is `Ready` or `Active`. No distinction between the two at health-check time. |

`HealthResponse::Ready` is only returned when all five checks pass, meaning the session is in `Ready` or `Active` state.

---

## Key Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Worker process lifecycle | One persistent process per active model | Eliminates per-request model init latency |
| Concurrency model | Mutex serializes requests; one in-flight request at a time | Avoids stdout interleaving; simplest correct design |
| Degraded detection | Errors on stdin write or rx recv | Process death surfaces via I/O failure |
| Degraded slot clearing | Set `Option<SttWorker>` to `None` on any I/O error | Health check must reflect actual session state |
| Restart trigger | `download_model` with different model ID | Explicit caller-initiated; no automatic restart policy |
| Shutdown | Drop-based; no graceful protocol | Sufficient for desktop plugin; OS reaps process |
| Progress event routing | Reader thread intercepts; never reaches `rx` | Keeps `rx` as a clean transcription-response-only channel |
| Startup timeout | 300 seconds | Accommodates cold download of large models on slow connections |
| Per-request timeout | 120 seconds | Bounds inference time; prevents indefinite hang on large files |
