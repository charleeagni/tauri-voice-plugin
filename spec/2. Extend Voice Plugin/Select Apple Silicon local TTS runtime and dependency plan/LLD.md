# Low Level Design: Apple Silicon TTS Runtime and Dependency Plan (CODIN-345)

## 1. Objective

Select the first local TTS runtime for Apple Silicon and document the dependency strategy needed to add it to the voice plugin without destabilizing the current STT bootstrap path.

This plan is documentation-only. It does not add the TTS worker, change bootstrap code, update public commands, or modify checked-in lock files.

## 2. Decision Summary

- Use `mlx-audio` as the first local TTS runtime.
- Use `mlx-community/Kokoro-82M-bf16` as the default TTS model.
- Use Kokoro preset `af_heart` as the default voice.
- Keep English and American English as the v1 quality baseline.
- Generate WAV files first.
- Do not require `ffmpeg` for v1.
- Keep one shared app-local Python environment by default.
- Split TTS into a separate Python environment only if an install or resolver conflict is proven.
- Do not add streaming, direct playback, voice cloning, or conversational orchestration in this ticket.

## 3. Current Runtime Baseline

The voice plugin currently bootstraps Python through the bundled `uv` sidecar.

Current runtime paths:

- `APP_DATA_DIR/python/.venv`
- `APP_DATA_DIR/python/.venv/bin/python`
- `APP_DATA_DIR/python/cache`
- `APP_DATA_DIR/python/tools`

Current source lock file:

- `tauri-plugin-voice/requirements/requirements-stt.lock.txt`

Current bootstrap code installs one lock file from `BootstrapManager`.

Current STT dependency anchors:

- `mlx==0.31.0`
- `mlx-metal==0.31.0`
- `mlx-whisper==0.4.3`
- `huggingface-hub==1.5.0`
- `numpy==2.4.2`
- `torch==2.10.0`

The bootstrap code pins Python `3.12`. The current STT lock header says it was compiled with Python `3.14`, so the TTS lock generation task should normalize the compile command to the bootstrap Python version.

## 4. Resolver Validation

`mlx-audio` was resolved against the current STT lock as a validation step before recommending a shared environment.

Validation commands:

- `uv pip compile --python-version 3.14 -o /private/tmp/voice-plugin-tts-resolve.lock /private/tmp/voice-plugin-tts-resolve.in`
- `uv pip compile --python-version 3.12 -o /private/tmp/voice-plugin-tts-resolve-py312.lock /private/tmp/voice-plugin-tts-resolve.in`

Validation result:

- Python `3.14` resolution succeeded with 70 packages.
- Python `3.12` resolution succeeded with 66 packages.
- `mlx-audio==0.4.2` resolved with the existing STT dependency set.
- The existing `mlx`, `mlx-metal`, and `mlx-whisper` pins remained unchanged.
- No dependency conflict was observed.

Additional packages introduced by `mlx-audio` include:

- `mlx-audio==0.4.2`
- `mlx-lm==0.31.1`
- `librosa==0.11.0`
- `miniaudio==1.71`
- `sounddevice==0.5.3`
- `soundfile==0.13.1`
- `transformers==5.8.0`
- `tokenizers==0.22.2`
- `safetensors==0.7.0`
- `sentencepiece==0.2.1`
- `protobuf==7.34.1`
- `pyloudnorm==0.2.0`

## 5. Dependency Strategy

The first TTS implementation should keep the shared Python environment and move from an STT-only lock to a voice lock.

Recommended lock file target:

- `tauri-plugin-voice/requirements/requirements-voice.lock.txt`

Recommended source requirements inputs:

- Keep STT requirements anchored by the existing STT dependency set.
- Add only `mlx-audio` as the direct TTS dependency.
- Do not add `ffmpeg`, playback libraries, streaming libraries, or optional quantized model dependencies in v1.

Recommended generation rule:

- Generate the voice lock with Python `3.12`, matching `BootstrapManager`.
- Keep `mlx==0.31.0`, `mlx-metal==0.31.0`, and `mlx-whisper==0.4.3` pinned unless a later resolver run proves a necessary change.
- Review the resolver output before replacing the bootstrap lock path.

Recommended bootstrap change for the implementation ticket:

- Change the bootstrap lock path from the STT lock to the voice lock when TTS is enabled.
- Keep one install step into `APP_DATA_DIR/python/.venv`.
- Do not install a TTS lock after an STT lock because the second install could silently alter shared packages.

Fallback if conflicts appear:

- Keep `requirements-stt.lock.txt` for STT.
- Add `requirements-tts.lock.txt` for TTS.
- Create `APP_DATA_DIR/python/stt/.venv` and `APP_DATA_DIR/python/tts/.venv`.
- Route each worker to its engine-specific Python path.
- Document the exact conflict that forced the split.

Because the resolver validation did not find a conflict, the split environment is not recommended for the first implementation.

## 6. Model Strategy

Default model:

- `mlx-community/Kokoro-82M-bf16`

Default voice:

- `af_heart`

Allowed model behavior:

- Accept an optional `modelId`.
- Default to `mlx-community/Kokoro-82M-bf16` when omitted.
- Reject unsupported models with a typed error.
- Do not silently route to non-Kokoro models.

Allowed voice behavior:

- Accept an optional `voiceId`.
- Default to `af_heart` when omitted.
- Support only Kokoro preset voices in v1.
- Reject unsupported voices with a typed error.

Allowed language behavior:

- Accept an optional Kokoro language code.
- Default to English or American English behavior.
- Pass a language code through the worker contract only when supplied.
- Do not claim broad multilingual quality until validated.

Optional future model variants:

- Consider 8-bit or 4-bit Kokoro variants only if memory use, startup time, or cold preload cost requires it.
- Do not include quantized variants in the first dependency or allowlist.

## 7. Worker Contract

The TTS worker should be a separate persistent Python process from the STT worker.

Request fields:

- `id`
- `text`
- Optional `modelId`
- Optional `voiceId`
- Optional `languageCode`
- Optional `speed`
- Optional `outputPath`
- Optional `outputDir`

Response fields:

- `id`
- `audioPath`
- `modelId`
- `voiceId`
- Optional `languageCode`
- Optional `durationMs`
- Optional typed `error`

Typed errors:

- `unsupported_model`
- `unsupported_voice`
- `unsupported_language`
- `invalid_input`
- `generation_failed`

Output behavior:

- Write a local WAV file.
- Prefer an explicit `outputPath` when provided.
- Otherwise create a file under the configured output directory.
- If no output directory is supplied, use a TTS-specific app data output directory.
- Return the generated path after the file is fully written.

## 8. Download and Preload Behavior

The TTS worker should mirror the STT worker's observable lifecycle where practical.

Download phase:

- Download the Hugging Face model snapshot through the worker.
- Use anonymous public model access.
- Emit model progress with `engine = tts`.
- Reuse the app-local cache conventions already used by bootstrap.

Preload phase:

- Load the Kokoro model before reporting worker readiness.
- Emit preload start, complete, and failed states.
- Treat preload failure as TTS readiness failure only.
- Do not block STT readiness on TTS preload failure unless TTS is configured as required.

Readiness behavior:

- `voice_health` should include TTS readiness separately from STT readiness.
- STT commands should remain STT-gated.
- TTS commands should be TTS-gated.
- Aggregate voice readiness should depend only on configured required engines.

## 9. Bootstrap Impact

Shared bootstrap remains the default.

When TTS implementation begins:

- Ensure the voice lock is installed into `APP_DATA_DIR/python/.venv`.
- Initialize STT and TTS workers independently.
- Keep STT worker failure and TTS worker failure separate.
- Keep startup preload configurable per engine.
- Start TTS preload only when TTS is enabled by configuration or explicitly requested.

The bootstrap response should remain readable and should not expose resolver or installation internals unless an error occurs.

## 10. Expected Performance

Kokoro is a small TTS model compared with large conversational models, and `mlx-audio` runs on Apple's MLX stack for Apple Silicon. Expected behavior for v1:

- First use may include dependency installation and model snapshot download.
- First synthesis after download may include model preload cost.
- Subsequent synthesis should reuse the persistent TTS worker and loaded model.
- WAV output avoids additional encoding overhead.
- Apple Silicon should be the supported performance baseline.

The implementation ticket should measure cold preload time, warm synthesis time, and generated audio duration once the worker exists.

## 11. Licensing Constraints

Current licensing decision:

- `mlx-audio` is MIT licensed.
- `mlx-community/Kokoro-82M-bf16` is Apache-2.0 licensed.
- Upstream Kokoro model metadata is Apache-2.0 licensed.

Implementation notes:

- Preserve model and runtime attribution in documentation.
- Avoid bundling downloaded model weights into the plugin source.
- Download model weights at runtime into the app-local cache.
- Recheck license metadata before shipping a distributable application.

## 12. Validation Plan

Dependency validation:

- Regenerate the combined voice lock using Python `3.12`.
- Confirm `uv pip compile` succeeds.
- Confirm the resulting lock keeps current STT pins unless intentionally changed.
- Confirm bootstrap installs the combined lock into a clean app-local venv.

Worker validation:

- Start STT and TTS workers independently.
- Confirm STT transcription still works after installing the voice lock.
- Confirm TTS generates a WAV file with the default model and `af_heart`.
- Confirm unsupported model, voice, and language inputs return typed errors.
- Confirm `voice_health` reports STT and TTS readiness independently.

Model validation:

- Confirm first model download emits `engine = tts` progress.
- Confirm warm synthesis reuses the loaded model.
- Confirm generated WAV files are readable without `ffmpeg`.

## 13. Out of Scope

- Streaming audio responses.
- Direct playback.
- Voice cloning.
- Conversational orchestration.
- Non-Kokoro model support.
- Quantized Kokoro variants.
- Public multilingual quality claims.
- Consumer app migration.
- Runtime code changes in this ticket.
