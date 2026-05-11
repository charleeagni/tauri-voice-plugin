# High Level Design: Kokoro Streaming TTS Playback MVP

## 1. Objective

Add an explicit streaming playback mode for Kokoro TTS while preserving the existing file-output synthesis contract.

This design keeps synthesis in the Python TTS worker, event forwarding in Rust, and playback in guest JavaScript through Web Audio. It does not add Python speaker playback, Rust audio output, conversational turn orchestration, or recorder/STT behavior changes.

## 2. Current Baseline

The voice plugin already has a file-output TTS path:

- Rust exposes `synthesize_speech` under `plugin:voice|synthesize_speech`.
- `SynthesizeSpeechRequest` resolves text, Kokoro model, voice, language, speed, and output path.
- `tts_worker.py` loads `mlx-community/Kokoro-82M-bf16` and writes one WAV file per request.
- Guest JS exposes `synthesizeSpeech` and returns the generated file details.
- TTS readiness remains engine-specific through the TTS worker and health diagnostics.

Streaming must build beside this path. It must not weaken file output or make readiness depend on playback support.

## 3. Product Decision

Streaming is an explicit caller-selected TTS mode.

Supported output modes:

- `file`: existing stable fallback path.
- `stream`: emit audio chunks for frontend Web Audio playback.

Do not add `both` in this MVP. Persisting while streaming adds file lifecycle, cleanup, and partial-output semantics. A later task can add `both` after the stream-only path is reliable.

## 4. Chunk Format Decision

Use base64-encoded PCM16 little-endian mono chunks inside JSON events for the MVP.

Reasons:

- JSON lines match the current worker protocol.
- Rust can forward payloads through Tauri events without binary channel work.
- Guest JS can decode PCM into `AudioBuffer` without WAV header parsing per chunk.
- PCM16 keeps payloads smaller and simpler than WAV-framed chunks.

Initial audio metadata:

- `format`: `pcm_s16le`
- `sampleRateHz`: `24000` unless the worker reports a different Kokoro sample rate.
- `channels`: `1`

The worker may internally receive generated float audio from `mlx-audio`, convert it to PCM16, and chunk by frame count before emitting JSON.

## 5. Public Streaming Contract

Add a separate streaming request rather than overloading file-output fields heavily.

Rust command:

- `stream_speech`

Guest JS helper:

- `streamSpeech`
- `playStreamedSpeech`

The command starts synthesis and returns a `synthesisId`. Audio delivery then happens through Tauri events.

The request should include:

- `text`
- `modelId`
- `voiceId`
- `languageCode`
- `speed`
- `chunkDurationMs`

The response should include:

- `synthesisId`
- `modelId`
- `voiceId`
- `languageCode`
- `format`
- `sampleRateHz`
- `channels`

The existing `synthesizeSpeech` function remains the documented fallback for file output.

## 6. Event Lifecycle

Use one public Tauri event channel:

- `tauri-voice-plugin://tts-stream`

Each event includes:

- `contractVersion`
- `eventId`
- `emittedAtMs`
- `synthesisId`
- `sequence`
- `type`

Lifecycle types:

- `start`: stream accepted and metadata is available.
- `chunk`: ordered audio payload.
- `complete`: worker has finished generation and no more chunks will arrive.
- `error`: generation or forwarding failed.
- `cancelled`: cancellation was accepted and playback should stop.

Chunk events include:

- `sampleRateHz`
- `channels`
- `format`
- `audioBase64`
- `durationMs`
- `final`

The `complete` event is the authoritative completion signal. The `final` chunk flag exists only to let frontend code drain the last audio buffer predictably.

## 7. Buffering Policy

Guest JS should buffer before starting playback.

Initial MVP policy:

- Default chunk target: 200 ms.
- Start playback after at least 400 ms buffered or after the first final chunk.
- Keep scheduling buffers against the current `AudioContext.currentTime`.
- If the buffer underruns, resume playback when at least one chunk is available.

This favors simple, low-latency behavior while avoiding immediate stutter on normal generation jitter.

## 8. Cancellation Model

Cancellation is synthesis-ID based.

Add a Rust command:

- `cancel_speech`

Cancellation flow:

1. Guest JS calls `cancelSpeech({ synthesisId })`.
2. Rust marks the active synthesis as cancelled.
3. Rust sends a cancel message to the TTS worker if the worker is still active.
4. The worker stops emitting chunks for that synthesis as soon as it observes cancellation.
5. Rust drops any later chunks for the cancelled synthesis.
6. Rust emits a `cancelled` event.
7. Guest JS stops scheduled audio and closes or resets its playback state.

If the underlying model call cannot be interrupted mid-call, the worker should suppress delivery after cancellation and finish cleanup before accepting the next request.

## 9. Worker Protocol

Keep newline-delimited JSON for worker communication.

Streaming request line:

- `id`
- `mode`: `stream`
- `text`
- `model`
- `voice`
- `language`
- `speed`
- `chunk_duration_ms`

Worker output lines:

- `status`: `stream_start`, `stream_chunk`, `stream_complete`, `stream_error`, or `stream_cancelled`
- `id`
- `sequence`
- audio metadata for start and chunks
- base64 PCM payload for chunks

Cancellation request line:

- `id`
- `command`: `cancel`

The worker continues processing one active synthesis at a time in this MVP.

## 10. Rust Ownership

Rust owns:

- request validation
- synthesis ID generation
- one active streaming synthesis record
- worker lifecycle
- event shape normalization
- cancellation state
- stale chunk suppression

Do not add a generic event bus or broad engine registry. Keep the streaming logic local to the TTS implementation in `desktop.rs` unless the code becomes unreadable.

## 11. Guest JS Playback

Guest JS owns Web Audio playback.

Responsibilities:

- subscribe to `tauri-voice-plugin://tts-stream`
- filter events by `synthesisId`
- decode base64 PCM16 into `AudioBuffer`
- queue and schedule buffers
- expose a cancellation handle
- stop playback on `complete`, `error`, or `cancelled`

The helper should be optional. Applications can still subscribe to stream events directly.

## 12. Persistence Policy

Streaming does not persist generated audio in this MVP.

Persist audio only when callers use `synthesizeSpeech`. This keeps file cleanup and partial-stream failure behavior outside the streaming path.

## 13. Health and Readiness

Basic TTS readiness remains engine-specific and model-centric.

Streaming support does not add a separate readiness requirement. If the TTS worker can load Kokoro and accept synthesis requests, the plugin can report TTS readiness. Streaming errors should be request-level errors, not health failures unless the worker process itself fails.

## 14. Out of Scope

- Python speaker playback.
- Native Rust audio playback.
- Rust audio output dependencies.
- Conversational turn orchestration.
- STT or recorder behavior changes.
- Streaming plus file persistence in one request.
- Concurrent streaming synthesis requests.
- Browser-independent audio output guarantees beyond Web Audio.

## 15. Validation

Validation should cover:

- streaming event payload serialization
- chunk event metadata
- command response payload shape
- cancellation suppresses further chunk delivery
- guest JS decode and scheduling helpers
- file-output synthesis remains available
- health readiness does not require streaming playback
