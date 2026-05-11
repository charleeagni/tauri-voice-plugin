# High Level Design: Integrate Voice Plugin (Python Sidecar STT) into Steno (CODIN-301)

## 1. Objective
Add `mlx` as a new Steno transcription runtime that uses `voice_plugin` only as a host-driven file transcription backend for Python MLX inference. Steno must keep its existing recording lifecycle, overlay ownership, and output flow.

## 2. Scope
In scope:
- Add `mlx` to Steno runtime selection.
- Persist `runtime_selection = mlx` and a dedicated `mlx_model_id`.
- Proactively bootstrap `voice_plugin` readiness when `mlx` is selected.
- Route final post-stop transcription through `voice_plugin.transcribeFile(path, modelId?)`.
- Integrate MLX models into Steno's existing Models UI and settings flow.
- Surface MLX model download progress with the existing Models experience.
- Map `voice_plugin` command failures into Steno's existing error pathways.

Out of scope:
- Overlay fixes or redesign inside `voice_plugin`.
- Rust-Python download bridge repair inside `voice_plugin`.
- Switching Steno recording, hotkeys, overlay, clipboard, or output orchestration to `voice_plugin`.
- Using `setupRecordTranscribePipeline` or any recorder-driven plugin path for this ticket.
- Queue or cancel support for MLX model downloads in V1.

## 3. Current Context
Steno currently owns recording, runtime phase changes, overlay updates, and final output delivery. Its runtime settings and model management support `whisper`, `parakeet`, and `moonshine`, while `voice_plugin` already exposes `bootstrapVoice()`, `voiceHealth()`, `downloadModel()`, and `transcribeFile(...)` for MLX-backed transcription.

The agreed direction is to preserve Steno's runtime harness and replace only the final file-transcription backend when the selected runtime is `mlx`.

## 4. Proposed Architecture

### 4.1 Runtime Selection and Settings
- Extend Steno's runtime enum and persisted settings with `mlx`.
- Add a persisted `mlx_model_id` field.
- Use `tiny.en` as the canonical default MLX model.
- Keep existing Whisper, Parakeet, and Moonshine settings unchanged.

### 4.2 Readiness Ownership
- When the user selects `mlx`, Steno proactively calls `bootstrapVoice()` and checks `voiceHealth()`.
- Steno owns the readiness UX and determines whether the MLX runtime is selectable or actionable.
- Readiness failures are surfaced through Steno's existing runtime error reporting instead of a new plugin-specific state machine.

### 4.3 Final-Only Transcription Path
- Steno continues to record audio with its current recorder flow.
- After stop, Steno passes the captured file path to `voice_plugin.transcribeFile(...)`.
- The MLX V1 contract is final transcription only.
- Interim transcription, live-word output, and recorder-driven plugin events remain disabled or ignored for `mlx`.

### 4.4 Model Management Integration
- Add an `mlx` group inside Steno's existing Models section.
- Populate that group from the full allowlisted MLX model set exposed by `voice_plugin`.
- Adapt MLX downloads into Steno's current models surface without creating a separate settings flow.
- V1 visible statuses for MLX models are `not_downloaded`, `downloading`, `ready`, and `failed`.
- `voice_plugin` `PROGRESS` events are used only for model download progress.

### 4.5 Event and Error Mapping
- Steno remains the source of truth for runtime phases such as recording and transcribing.
- Ignore `voice_plugin` `STATE`, `COMPLETE`, and `LIVE` events for runtime-state purposes.
- Use `PROGRESS` only to update MLX model download state in the Models UI.
- Treat failures from `bootstrapVoice`, `voiceHealth`, `downloadModel`, and `transcribeFile` as Steno runtime or action errors.

## 5. High-Level Components
- Steno frontend settings:
  Extends runtime selection and model selection UI to include `mlx` and `mlx_model_id`.
- Steno runtime settings store:
  Persists the selected MLX runtime and model alongside existing runtime settings.
- Steno MLX runtime adapter:
  Encapsulates bootstrap, health, and `transcribeFile` calls so the existing runtime flow can branch cleanly by runtime.
- Steno MLX model adapter:
  Projects plugin model metadata and progress events into Steno's existing Models UI state shape.
- `voice_plugin` command surface:
  Remains an external backend contract used for readiness, download, and file transcription only.

## 6. Primary Flows

### 6.1 Runtime Selection Flow
1. User selects `mlx` in Steno settings.
2. Steno persists `runtime_selection = mlx`.
3. Steno bootstraps `voice_plugin` and checks readiness.
4. Steno shows readiness or error using its existing runtime feedback path.

### 6.2 Model Download Flow
1. User opens the existing Models section.
2. Steno shows the `mlx` model group beside existing runtime models.
3. User starts an MLX model download from the same Models experience.
4. Steno listens to plugin `PROGRESS` events and updates the mapped MLX model status.
5. Final state resolves to `ready` or `failed`.

### 6.3 Record and Transcribe Flow
1. Steno records audio with the current recorder pipeline.
2. On stop, Steno transitions into its existing transcribing phase.
3. If runtime selection is `mlx`, Steno sends the captured file to `voice_plugin.transcribeFile(...)` with the selected `mlx_model_id`.
4. Steno receives the final transcript and continues its normal output flow.

## 7. Risks and Mitigations
- Model-management mismatch:
  Steno currently expects queue and cancel semantics, while MLX V1 does not support them. The mitigation is an adapter that narrows MLX to the supported V1 statuses without forcing plugin contract changes.
- Readiness ambiguity:
  Delaying bootstrap until first use would make failures harder to explain. The mitigation is proactive bootstrap when `mlx` is selected.
- Runtime-state duplication:
  Reusing plugin recorder events would create a parallel state machine. The mitigation is to keep Steno as the only runtime-state owner.

## 8. Success Criteria
- Steno exposes `mlx` as a persisted runtime option.
- `tiny.en` is the default MLX model on first use.
- Selecting `mlx` proactively checks plugin readiness.
- Final post-stop transcription runs through `voice_plugin.transcribeFile(...)`.
- MLX models appear in Steno's existing Models UI with V1 download statuses.
- No Track 1 plugin overlay or bridge-repair work is pulled into this task.
