# High Level Design: Re-expose Recorder Capabilities via Voice Plugin (CODIN-252)

## 1. Objective

Enable target apps to depend on `tauri-plugin-stt` and still access recorder features required for hotkey-driven record-and-transcribe workflows.

## 2. Confirmed Product Requirements

- App can use hotkeys to start/stop recording and then transcribe.
- App can expose register-hotkey functionality.
- App can configure recording output destination, with existing defaults preserved.
- Overlay is enabled by default and can be disabled.
- Overlay shows `recording`, `transcribing`, then final transcribed text for 2 seconds.

## 3. Decisions Locked From Planning

- Re-exposure style: pass-through behavior, not recorder rewrite.
- API naming: keep recorder names unchanged.
- Event channels: keep existing recorder channels unchanged.
- Dependency strategy: recorder integration remains optional at compile-time via feature flag.

## 4. Scope

In scope:
- Add optional recorder bridge inside `tauri-plugin-stt`.
- Register recorder plugin dynamically when recorder bridge feature is enabled.
- Re-expose recorder command surface through `tauri-plugin-stt` guest API.
- Preserve recorder event channel names and payloads.
- Add overlay text propagation for final transcript display window (2 seconds).

Out of scope:
- Rewriting recorder internals in `tauri-plugin-stt`.
- Renaming recorder contracts.
- Cross-platform overlay redesign.
- New overlay event channels.

## 5. Architecture Intent

### 5.1 Optional bridge, stable consumer surface

`tauri-plugin-stt` becomes a composition plugin:
- Core STT commands remain native to `tauri-plugin-stt`.
- Recorder features are available when `recorder-bridge` feature is enabled.
- When feature is disabled, recorder-facing stubs fail with typed `feature_disabled` errors.

### 5.2 Runtime plugin composition

At plugin setup:
- `tauri-plugin-stt` initializes its own STT runtime.
- If `recorder-bridge` is enabled, it dynamically registers `tauri-plugin-recorder` using `AppHandle::plugin(...)`.

### 5.3 Pass-through contract ownership

- Command names remain recorder-compatible.
- Event channels remain recorder-compatible.
- Target app uses only `tauri-plugin-stt` frontend package for both STT and recorder actions.

### 5.4 Overlay transcript UX extension

Overlay phases remain recorder-driven:
- `recording`
- `transcribing`
- `idle/error`

Additional behavior:
- On successful transcription completion, overlay shows final text for 2 seconds, then returns to normal phase rendering.

## 6. Risk and Mitigation

- Risk: optional feature causes partial API mismatch.
  - Mitigation: keep API always generated and return typed `feature_disabled` when bridge is off.
- Risk: dynamic recorder plugin registration ordering.
  - Mitigation: register recorder during `tauri-plugin-stt` setup before command usage.
- Risk: overlay text timing race conditions.
  - Mitigation: explicit 2-second timer and deterministic last-write wins policy.

## 7. Validation Expectations

- Recorder commands callable through voice-plugin package with recorder-compatible names.
- Recorder channels unchanged and consumable from target app.
- Hotkey workflow supports record and stop-to-transcribe.
- Output destination override works; default path remains unchanged when not overridden.
- Overlay default enabled; disable path works.
- Final transcribed text appears for ~2 seconds and clears deterministically.

## 8. Exit Criteria

- `tauri-plugin-stt` can be the only plugin dependency for target app integration.
- Recorder features are available through voice-plugin surface when bridge is enabled.
- Contract behavior matches locked requirements without recorder API renames.
