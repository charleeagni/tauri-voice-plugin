# LLD — CODIN-256: Persistent STT Runtime Session Contract

## Scope

One file changes. Everything else is already contract-compliant.

- `tauri-plugin-stt/src/desktop.rs` — fix degraded slot clearing in `transcribe_file_internal`.

Validation is done through the example application (`tauri-plugin-stt/examples/tauri-app`), not automated tests.

---

## 1. `desktop.rs` — Degraded Slot Clearing

### Problem

`transcribe_file_internal` uses `?` to propagate I/O errors from within the worker mutex guard scope. `?` exits the enclosing block immediately, leaving `*worker_guard` as `Some(broken_worker)`. `stt_health` then reports `stt_worker: true` for a dead process.

### Location

`TauriPluginStt::transcribe_file_internal` — the inner block starting at `let text = { let mut worker_guard = self.worker.lock()... }`.

### Three Degraded Triggers

All three are inside the locked block where `worker_guard: MutexGuard<Option<SttWorker>>` is held.

**Trigger 1 — stdin write failure**
`writeln!(worker.stdin, "{}", req_line)` returns `Err`.

**Trigger 2 — stdin flush failure**
`worker.stdin.flush()` returns `Err`.

**Trigger 3 — rx recv timeout or disconnect**
`worker.rx.recv_timeout(Duration::from_secs(120))` returns `Err`.

### Required Change

For each trigger, replace the `?`-propagation pattern with an explicit result check:

- Evaluate the fallible operation into a local `Result`.
- On `Err`: set `*worker_guard = None` to evict the broken handle, then return the mapped error.
- On `Ok`: continue as before.

The mutex guard must still be held when the slot is cleared so no concurrent caller can observe a stale `Some`.

The `worker.rx.recv_timeout` trigger has an additional nuance: the `rx` channel closes when the reader thread exits (which happens when the worker process dies). This error is therefore also an `Err` on disconnect, not just a timeout. Both cases are already covered by `recv_timeout` returning `Err(RecvTimeoutError::Disconnected)` — the clear-and-return logic applies identically.

### No Change to `spawn_worker` or `download_model_inner`

`download_model_inner` already sets the slot to `None` before calling `spawn_worker`. No adjustment needed there.

### No Change to `stt_health`

`stt_health` already reports `NotReady { stt_worker: false }` when the slot is `None`. Once the slot is cleared on degraded, the health check becomes accurate with no further changes.

---

## 2. Validation via Example Application

Run `pnpm tauri dev` from `tauri-plugin-stt/examples/tauri-app`.

### Scenario A — Normal warm session

1. Call `bootstrapStt()`. Verify `stt_health` returns `Ready` with all five diagnostics passing, including `stt_worker: true`.
2. Call `transcribeFile(path)` on a valid WAV file. Verify transcript is returned.
3. Call `transcribeFile(path)` a second time on the same session. Verify transcript is returned without re-bootstrapping (model is reused).

### Scenario B — Degraded slot cleared on worker death

1. Call `bootstrapStt()` to load the worker.
2. Kill the Python worker process externally (e.g. `kill <pid>`).
3. Call `transcribeFile(path)`. Verify it returns a `generation_failed` error.
4. Call `sttHealth()` immediately after. Verify `stt_worker` diagnostic is `ready: false` — not `true`.

### Scenario C — Recovery after degraded

1. Follow Scenario B steps 1–4.
2. Call `bootstrapStt()` again. Verify it spawns a fresh worker and returns `Ready`.
3. Call `transcribeFile(path)`. Verify transcript is returned.

---

## Affected Symbols

| Symbol | File | Change |
|---|---|---|
| `TauriPluginStt::transcribe_file_internal` | `desktop.rs` | Replace three `?` I/O exits with explicit slot-clear-then-return |

---

## Unchanged

| Item | Reason |
|---|---|
| `models.rs` | All required types already exist (`HealthResponse`, `DiagnosticEntry`). |
| `worker.py` | Already implements the correct wire protocol. |
| `stt_health` | Already correct once slot is cleared on degraded. |
| `spawn_worker` | Startup contract already complete. |
| `download_model_inner` | Restart contract already correct. |
| `bootstrap_stt` | No session lifecycle change needed. |
