# LLD: CODIN-251 Implement macOS overlay runtime via tauri-nspanel

## Scope
Implement a direct, readable port of Steno overlay runtime into `tauri-plugin-recorder`, with plugin-managed default UI and consumer-provided UI override mode. The key gap versus a plain Steno port is NSPanel (non-activating panel) configuration via `tauri-nspanel` so the overlay never steals keyboard focus.

## Resolved Decisions

### Window lifecycle
The plugin spawns and owns the NSPanel WebviewWindow dynamically at runtime. The host app does not declare an overlay window in `tauri.conf.json`. The plugin creates the window, configures it as an NSPanel, and manages its full lifecycle (show/hide/position/destroy).

### Consumer mode UI contract
- The plugin owns: NSPanel lifecycle, state machine, emitting phase events into the webview.
- The consumer provides: a URL to their own route + per-phase visual components.
- The plugin's React helper (running inside the consumer's webview route) subscribes to state events and swaps the correct component when the phase changes.
- `set_overlay_mode` in Consumer mode accepts a `consumer_url: String` that the plugin navigates the NSPanel webview to.

## Source Port References
- `/Users/karthik/merge_conflicts/steno/src-tauri/src/overlay_runtime.rs`
- `/Users/karthik/merge_conflicts/steno/src-tauri/src/runtime.rs` (`emit_state` + phase mapping)
- `/Users/karthik/merge_conflicts/steno/src/OverlayApp.tsx` (default UI behavior)

---

## Implementation Status

| Phase | Task | Status |
|-------|------|--------|
| Phase 1 | Dependency & macOS Gating | ⚠️ Partial — `#[cfg(target_os = "macos")]` guards exist; `tauri-nspanel` dep missing |
| Phase 2 | NSPanel Window Management | ❌ Not done — plain `WebviewWindow` used (steals focus) |
| Phase 3 | State Machine to UI Mapping | ✅ Done — `sync_overlay_shell` + `sync_overlay_runtime_for_state` |
| Phase 4 | Consumer Override & Degradation | ✅ Done — `OverlayMode`, `set_overlay_mode`/`get_overlay_mode`, graceful no-op |

---

## Phase 1: Dependency & macOS Gating (CODIN-263)

### Cargo.toml change
Add to `[target.'cfg(target_os = "macos")'.dependencies]`:

```
tauri-nspanel = { git = "https://github.com/ahkohd/tauri-nspanel", branch = "v2" }
```

Existing `#[cfg(target_os = "macos")]` guards on `overlay_runtime.rs` already satisfy the gating requirement.

---

## Phase 2: NSPanel Window Management (CODIN-264)

### Target file: `src/overlay_runtime.rs`

The "overlay" WebviewWindow must be converted to a Non-Activating Panel (NSPanel) exactly once after it is obtained. This conversion:
- Prevents the overlay from stealing keyboard focus when shown.
- Sets the window level to `Floating` (always above normal windows).
- Sets collection behavior to `CanJoinAllSpaces` (visible on all desktops/workspaces).

### Approach
- The plugin spawns the `"overlay"` `WebviewWindow` dynamically on first show (lazy init). No host-app `tauri.conf.json` window declaration required.
- On spawn: apply `to_panel()` and configure NSPanel properties immediately before showing.
- Default mode: load the plugin's bundled default indicator HTML.
- Consumer mode: navigate to the `consumer_url` provided via `set_overlay_mode`.
- Use a `OnceLock<WebviewWindow>` or `Arc<Mutex<Option<WebviewWindow>>>` in `overlay_runtime.rs` to hold the spawned window across phase transitions.

### NSPanel API (tauri-nspanel v2)

```rust
use tauri_nspanel::WebviewWindowExt;
use tauri_nspanel::cocoa::appkit::NSWindowCollectionBehavior;

// Convert to panel and configure.
let panel = overlay_window.to_panel().unwrap();
panel.set_level(tauri_nspanel::NSNormalWindowLevel + 1); // floating
panel.set_style_mask(tauri_nspanel::NSWindowStyleMask::NonActivatingPanel);
panel.set_collection_behaviour(
    NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
    | NSWindowCollectionBehavior::NSWindowCollectionBehaviorStationary,
);
```

### Graceful degradation
- If `to_panel()` returns an error: log with `eprintln!`, skip NSPanel configuration, continue with plain `WebviewWindow`. Show/hide still functions; only non-activating property is lost.
- No panic, no recording lifecycle interruption.

---

## Phase 3: State Machine to UI Mapping (CODIN-265) — DONE

Already implemented in `src/overlay_runtime.rs`:

- `Recording` | `Transcribing` → `position_overlay_window` + `show()`
- `Idle` | `Error` → `hide()`

`sync_overlay_runtime_for_state` in `src/desktop.rs` bridges recorder `Phase` to `OverlayRuntimePhase`.

No changes needed unless NSPanel conversion (Phase 2) reveals show/hide ordering issues.

---

## Phase 4: Consumer Override & Degradation (CODIN-266) — NEEDS UPDATE

Existing logic (`OverlayMode` enum, `set_overlay_mode`/`get_overlay_mode`, graceful no-op) remains valid but must be extended for the resolved decisions:

### Changes required

- `OverlayMode::Consumer` variant must carry `consumer_url: String`.
- `set_overlay_mode(Consumer { consumer_url })` → plugin navigates the spawned NSPanel webview to `consumer_url`. Plugin still owns show/hide/positioning.
- `OverlayMode::Default` → plugin shows NSPanel with its bundled default HTML; plugin owns show/hide/positioning.
- `OverlayMode::Disabled` → plugin does not spawn NSPanel; existing window (if any) is destroyed or hidden permanently.

### Model change in `src/models.rs`

`OverlayMode::Consumer` becomes a tuple/struct variant:

```
Consumer { consumer_url: String }
```

### React helper contract (guest-js)
- Plugin ships a React helper that accepts a `{ idle, recording, transcribing, error }` component map.
- On mount: calls `set_overlay_mode(Consumer { consumer_url: window.location.href })`, subscribes to `STATE` channel.
- On state update: renders the matching per-phase component with current `RuntimeState` as props.
- On unmount: removes event subscription.

---

## Runtime Model (already in models.rs)

### `Phase`
- `idle`
- `recording`
- `transcribing`
- `error`

### `OverlayMode`
- `default`
- `consumer`
- `disabled`

---

## Files To Change (remaining work)

| File | Change |
|------|--------|
| `tauri-plugin-recorder/Cargo.toml` | Add `tauri-nspanel` git dep under macOS target |
| `tauri-plugin-recorder/src/overlay_runtime.rs` | Dynamic window spawn + NSPanel configuration; hold spawned window in state |
| `tauri-plugin-recorder/src/models.rs` | `OverlayMode::Consumer` → struct variant with `consumer_url: String` |
| `tauri-plugin-recorder/src/desktop.rs` | Pass `consumer_url` into overlay runtime on `set_overlay_mode(Consumer)` |
| `tauri-plugin-recorder/guest-js/index.ts` | Update `OverlayMode` type for new Consumer shape |
| `tauri-plugin-recorder/guest-js/` | Add React helper component (new file) |

---

## Validation Plan
- Phase transition test: `idle -> recording -> transcribing -> idle`.
- Confirm overlay visible during recording/transcribing, hidden during idle/error.
- Confirm focus does not transfer to overlay when it appears (NSPanel behavior).
- Overlay failure injection: if `to_panel()` errors, recording lifecycle must continue.
- Consumer mode test: default UI disabled, state events still fired.
- Non-macOS build: `cargo build --target x86_64-unknown-linux-gnu` must compile clean.
