# LLD — CODIN-257: Expose Model Download & Preload Progress Events

## Files Modified

| File | Change |
|---|---|
| `src/models.rs` | Add `ModelProgressEvent`, `DownloadModelRequest`, `DownloadModelResponse`; drop `model_id` from `BootstrapRequest` |
| `src/desktop.rs` | Hardcode `tiny.en` in `bootstrap_stt`; add `download_model`; add download-in-progress lock; update reader thread to intercept progress lines |
| `src/commands.rs` | Add `download_model` command handler |
| `src/lib.rs` | Register `download_model` in invoke handler |
| `build.rs` | Add `"download_model"` to `COMMANDS` |
| `scripts/worker.py` | Add tqdm intercept; emit JSON progress lines for download and preload phases |
| `guest-js/index.ts` | Add `ModelProgressEvent`, `CHANNELS.PROGRESS`, `DownloadModelRequest/Response`, `downloadModel()`, `listenModelProgress()` |

---

## `src/models.rs`

### Drop `model_id` from `BootstrapRequest`

```
BootstrapRequest { }    // was: model_id: Option<String>
```

### New: `DownloadModelRequest` / `DownloadModelResponse`

```
DownloadModelRequest {
    model_id: String,           // required; validated against allowlist
}

DownloadModelResponse {
    model_id: String,           // effective model now loaded in worker
    already_active: bool,       // true if worker was already loaded with this model
}
```

### New: `ModelProgressEvent`

```
ModelProgressEvent {
    contract_version: String,
    event_id: String,
    emitted_at_ms: u64,
    model: String,
    phase: ModelProgressPhase,      // enum: Download | Preload
    state: ModelProgressState,      // enum: Start | InProgress | Complete | Failed
    percent: Option<f32>,           // 0.0–1.0; None when unavailable
    filename: Option<String>,       // active file; None during preload phase
    error: Option<String>,          // present only on Failed
}

enum ModelProgressPhase { Download, Preload }
// serde: "download" | "preload"

enum ModelProgressState { Start, InProgress, Complete, Failed }
// serde: "start" | "in_progress" | "complete" | "failed"
```

---

## `src/desktop.rs`

### New constant

```
const PROGRESS_CHANNEL: &str = "tauri-audio-plugin://model-progress";
```

### New field on `TauriPluginStt`

```
download_in_progress: Arc<Mutex<bool>>,
```

Initialised to `false` in `init()`.

### `bootstrap_stt` — remove `model_id` handling

- Drop the `model_id` extraction from `payload`.
- Pass the literal `DEFAULT_MODEL` (`"tiny"` → change to `"tiny.en"`) to `spawn_worker`.
- `DEFAULT_MODEL` constant changes from `"tiny"` to `"tiny.en"`.

### New: `download_model`

Pseudocode:

```
fn download_model(payload: DownloadModelRequest) -> Result<DownloadModelResponse>:

    // 1. Validate model against allowlist.
    if model_id not in ALLOWLISTED_MODELS → Err(invalid_input)

    // 2. Fail fast if download already in progress.
    acquire download_in_progress lock
    if *lock == true:
        return Err(not_ready("A model is already downloading, please wait"))
    set *lock = true
    drop lock

    defer: set download_in_progress = false on all exit paths

    // 3. Check if worker already loaded with same model.
    acquire worker lock
    if worker.is_some() && worker.model_id == model_id:
        return Ok(DownloadModelResponse { model_id, already_active: true })

    // 4. Tear down existing worker if different model.
    if worker.is_some():
        drop existing SttWorker (child is dropped, process exits)
        set worker = None
    drop worker lock

    // 5. Spawn new worker; progress events fire from reader thread during startup.
    let new_worker = self.spawn_worker(&model_id)?
    set worker = Some(new_worker)

    Ok(DownloadModelResponse { model_id, already_active: false })
```

### `spawn_worker` — update reader thread to intercept progress lines

The background thread currently sends every stdout line to `tx`. Change it to:

```
for line in reader.lines():
    if line contains "\"status\":\"progress\"":
        parse as ModelProgressEvent and emit to PROGRESS_CHANNEL via app_handle clone
    else:
        send to tx (existing behaviour)
```

The `AppHandle` clone must be moved into the thread. Add `app: AppHandle<R>` parameter to the thread closure (already available as `self.app` before the thread spawn).

The `spawn_worker` signature gains no new parameters; the app handle is captured from `&self.app` before the thread is spawned, same pattern as the existing `PipelineRuntimeState` closure.

---

## `src/commands.rs`

### New command

```rust
#[tauri::command]
pub async fn download_model(
    app: AppHandle<impl Runtime>,
    payload: DownloadModelRequest,
) -> Result<DownloadModelResponse> {
    app.tauri_plugin_stt().download_model(payload)
}
```

Pattern is identical to the existing `bootstrap_stt` command.

---

## `build.rs`

Add `"download_model"` to the `COMMANDS` slice.

---

## `src/lib.rs`

Add `commands::download_model` to the `generate_handler!` macro list.

---

## `scripts/worker.py`

### Worker stdout JSON progress line format

```json
{"status": "progress", "phase": "download"|"preload", "state": "start"|"in_progress"|"complete"|"failed", "model": "<repo>", "percent": 0.0–1.0|null, "filename": "<file>"|null, "error": "<msg>"|null}
```

### Emit helper

```python
def emit_progress(phase, state, model, percent=None, filename=None, error=None):
    print(json.dumps({
        "status": "progress", "phase": phase, "state": state,
        "model": model, "percent": percent,
        "filename": filename, "error": error,
    }), flush=True)
```

### Custom tqdm subclass

```python
class ProgressTqdm(tqdm.tqdm):
    """Intercepts tqdm updates; emits JSON progress lines to stdout."""

    def __init__(self, *args, model_repo=None, phase="download", **kwargs):
        self._model_repo = model_repo
        self._phase = phase
        super().__init__(*args, **kwargs)
        emit_progress(phase, "start", model_repo, percent=None,
                      filename=getattr(self, "desc", None))

    def update(self, n=1):
        super().update(n)
        total = self.total or 0
        percent = (self.n / total) if total > 0 else None
        emit_progress(self._phase, "in_progress", self._model_repo,
                      percent=percent, filename=getattr(self, "desc", None))

    def close(self):
        if not self._closed:
            emit_progress(self._phase, "complete", self._model_repo, percent=1.0)
        super().close()
```

### `main()` update

Replace:
```python
load_models.load_model(args.model)
```

With:
```python
import tqdm as _tqdm_module

# Patch tqdm for huggingface_hub download phase.
_orig_tqdm = _tqdm_module.tqdm
_tqdm_module.tqdm = lambda *a, **kw: ProgressTqdm(*a, model_repo=args.model, phase="download", **kw)

emit_progress("download", "start", args.model)
try:
    load_models.load_model(args.model)
except Exception as exc:
    emit_progress("download", "failed", args.model, error=str(exc))
    raise
finally:
    # Restore tqdm.
    _tqdm_module.tqdm = _orig_tqdm

emit_progress("preload", "start", args.model)
# load_model also loads weights into memory; signal completion after return.
emit_progress("preload", "complete", args.model, percent=1.0)
```

**Note**: `load_models.load_model` performs both download and in-memory load in one call. The tqdm patch covers the download network phase. The preload phase (memory load) emits only `start` + `complete` with no intermediate steps, as mlx does not expose sub-step callbacks.

**Deterministic fallback**: if the model is already cached and no tqdm callbacks fire, the `download.start` and `download.complete` lines are still emitted explicitly (see above). `percent` will be `null` on the start line and `1.0` on the complete line.

---

## `guest-js/index.ts`

### New types

```ts
export type ModelProgressPhase = "download" | "preload";
export type ModelProgressState = "start" | "in_progress" | "complete" | "failed";

export interface ModelProgressEvent {
  contractVersion: string;
  eventId: string;
  emittedAtMs: number;
  model: string;
  phase: ModelProgressPhase;
  state: ModelProgressState;
  percent: number | null;
  filename: string | null;
  error: string | null;
}

export interface DownloadModelRequest {
  modelId: string;
}

export interface DownloadModelResponse {
  modelId: string;
  alreadyActive: boolean;
}
```

### Update `CHANNELS`

Add to the existing `CHANNELS` object:
```ts
PROGRESS: "tauri-audio-plugin://model-progress",
```

### New command wrapper

```ts
/**
 * Downloads and loads a Whisper model on demand.
 *
 * :param payload: Model ID to download. Must be an allowlisted model.
 * :return: The active model ID and whether it was already loaded.
 * :raises Error: If a download is already in progress or model_id is invalid.
 */
export async function downloadModel(
  payload: DownloadModelRequest,
): Promise<DownloadModelResponse> {
  return await invoke("plugin:stt|download_model", { payload });
}
```

### New event listener helper

```ts
/**
 * Subscribes to model download and preload progress events.
 *
 * :param onProgress: Callback invoked on each progress event.
 * :return: A teardown function to unsubscribe.
 */
export async function listenModelProgress(
  onProgress: (event: ModelProgressEvent) => void,
): Promise<() => void> {
  return await listen<ModelProgressEvent>(CHANNELS.PROGRESS, (event) => {
    onProgress(event.payload);
  });
}
```

### Remove `modelId` from `BootstrapRequest`

Drop the `modelId?: string` field from the `BootstrapRequest` interface.

---

## Edge Cases

| Scenario | Behaviour |
|---|---|
| `download_model` called while `bootstrap_stt` is running | `download_in_progress` lock is held; second call returns "already downloading" error |
| `download_model` called twice with same `model_id` | Second call hits `already_active` early-return path (no lock contention, worker not torn down) |
| Model is cached; no network download occurs | `download.start` and `download.complete` emitted; `percent: null` on start line |
| Worker process crashes mid-download | `spawn_worker` timeout fires after 300 s; `download_in_progress` flag is cleared in defer |
| `bootstrap_stt` called after `download_model` already loaded a model | Worker is already `Some`; `bootstrap_stt` skips `spawn_worker` (existing idempotency guard) |
