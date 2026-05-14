import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export const CONTRACT_VERSION = "0.1.0";

// =============================================================================
// TYPES & INTERFACES
// =============================================================================

// -------------------------
// STT Models
// -------------------------

export interface BootstrapRequest {}
export type BootstrapResponse =
  | { status: "ready"; details: string }
  | { status: "alreadyReady"; details: string };

export type Engine = "stt" | "tts";

export interface DownloadModelRequest {
  /** Voice engine that owns the model request. */
  engine: Engine;
  /** Allowlisted Whisper model ID to download and load on demand. */
  modelId: string;
}

export interface DownloadModelResponse {
  /** Voice engine that loaded the model. */
  engine: Engine;
  /** Effective model now loaded in the worker. */
  modelId: string;
  /** True if the worker was already loaded with this model. */
  alreadyActive: boolean;
}

export type ModelProgressPhase = "download" | "preload";
export type ModelProgressState = "start" | "in_progress" | "complete" | "failed";

export interface ModelProgressEvent {
  contractVersion: string;
  eventId: string;
  emittedAtMs: number;
  engine: Engine;
  model: string;
  phase: ModelProgressPhase;
  state: ModelProgressState;
  /** Download fraction 0.0–1.0; null when unavailable. */
  percent: number | null;
  /** Active filename during download; null during preload. */
  filename: string | null;
  /** Error message; present only on "failed" state. */
  error: string | null;
}

export interface TranscribeRequest {
  path: string;
  modelId?: string;
}
export interface TranscribeResponse {
  text: string;
}

export interface SynthesizeSpeechRequest {
  text: string;
  modelId?: string;
  voiceId?: string;
  languageCode?: string;
  speed?: number;
  outputPath?: string;
  outputDir?: string;
  fileNamePrefix?: string;
}

export interface SynthesizeSpeechResponse {
  audioPath: string;
  modelId: string;
  voiceId: string;
  languageCode?: string | null;
  durationMs?: number | null;
  format: string;
  sampleRateHz?: number | null;
  createdAtMs: number;
}

// -------------------------
// TTS Streaming Models
// -------------------------

export interface StreamSpeechRequest {
  text: string;
  modelId?: string;
  voiceId?: string;
  languageCode?: string;
  speed?: number;
  /** PCM chunk length in milliseconds (50–1000; default 200). */
  chunkDurationMs?: number;
}

export interface StreamSpeechResponse {
  synthesisId: string;
  modelId: string;
  voiceId: string;
  languageCode?: string | null;
  /** Always "pcm_s16le" for MVP. */
  format: string;
  sampleRateHz: number;
  channels: number;
}

export interface CancelSpeechRequest {
  synthesisId: string;
}

export interface CancelSpeechResponse {
  synthesisId: string;
  /** True when an active stream was cancelled. */
  cancelled: boolean;
}

export type TtsStreamEventType = "start" | "chunk" | "complete" | "error" | "cancelled";

export interface TtsStreamEvent {
  contractVersion: string;
  eventId: string;
  emittedAtMs: number;
  synthesisId: string;
  sequence: number;
  type: TtsStreamEventType;
  modelId?: string | null;
  voiceId?: string | null;
  languageCode?: string | null;
  sampleRateHz?: number | null;
  channels?: number | null;
  format?: string | null;
  audioBase64?: string | null;
  durationMs?: number | null;
  /** True for the last chunk before complete. */
  final: boolean;
  error?: string | null;
}

/** Handle returned by playStreamedSpeech to cancel active playback. */
export interface StreamPlaybackHandle {
  cancel: () => Promise<void>;
}

export interface SetupRecordTranscribePipelineRequest {
  toggleShortcut: string;
  recorderConfig?: RecorderConfig;
  modelId?: string;
  showFinalTranscript?: boolean;
}

export interface SetupRecordTranscribePipelineResponse {
  contractVersion: string;
  active: boolean;
}

export interface HealthRequest {}
export interface DiagnosticEntry {
  name: string;
  ready: boolean;
  reason?: string;
}

export type LifecycleState = "uninitialized" | "initializing" | "ready" | "failed";

export interface EngineHealth {
  engine: Engine;
  lifecycleState: LifecycleState;
  ready: boolean;
  required: boolean;
  diagnostics: DiagnosticEntry[];
}

export type HealthResponse =
  | { status: "ready"; lifecycleState: LifecycleState; engines: EngineHealth[]; diagnostics: DiagnosticEntry[] }
  | { status: "notReady"; lifecycleState: LifecycleState; reason: string; engines: EngineHealth[]; diagnostics: DiagnosticEntry[] };

// -------------------------
// Recorder Models
// -------------------------

export type DeliveryMode = "chunks" | "file" | "both";
export type TriggerSource =
  | "manual"
  | "shortcut_push_to_talk"
  | "shortcut_toggle";
export type DestinationPolicy = "app_data" | "custom_path" | "temp";

export interface OutputDestination {
  policy: DestinationPolicy;
  baseDir?: string | null;
  subdir?: string | null;
  filePattern?: string | null;
}

export interface EffectiveOutputDestination {
  policy: DestinationPolicy;
  baseDir?: string | null;
  subdir?: string | null;
  filePattern?: string | null;
  resolvedBaseDir?: string | null;
}

export type Phase = "idle" | "recording" | "transcribing" | "error";
export type OverlayMode =
  | { type: "default" }
  | { type: "consumer"; consumerUrl: string }
  | { type: "disabled" };
export type PermissionState = "unknown" | "granted" | "denied";

export interface RuntimeState {
  phase: Phase;
  micPermission: PermissionState;
}

export type AggregateStatus = "ready" | "degraded" | "blocked";

export interface Check {
  checkId: string;
  scope: string;
  status: string;
  code: string;
  guidance?: string | null;
}

export interface Issue {
  code: string;
  message: string;
  guidance?: string | null;
}

export interface Readiness {
  aggregateStatus: AggregateStatus;
  recordingReady: boolean;
  shortcutReady: boolean;
  checks: Check[];
  issues: Issue[];
}

export interface RecordingResult {
  recordingWavPath: string;
  durationMs: number;
  sampleRateHz: number;
  channels: number;
  bitDepth: number;
  deliveryMode: DeliveryMode;
  triggerSource: TriggerSource;
}

export interface InitializeRuntimeResponse {
  contractVersion: string;
  state: RuntimeState;
  readiness: Readiness;
  shortcutError?: any | null;
}

export interface StartRecordingResponse {
  contractVersion: string;
  sessionId: string;
  triggerSource: TriggerSource;
  state: RuntimeState;
  effectiveOutputDestination: EffectiveOutputDestination;
}

export interface StopRecordingResponse {
  contractVersion: string;
  result: RecordingResult;
}

export interface GetRuntimeStateResponse {
  contractVersion: string;
  state: RuntimeState;
  readiness: Readiness;
}

export interface SetHotkeyBindingsResponse {
  contractVersion: string;
  state: RuntimeState;
}

export type CaptureHotkeyStatus = "captured" | "cancelled" | "timeout";

export interface CaptureHotkeyResponse {
  contractVersion: string;
  status: CaptureHotkeyStatus;
  shortcut?: string | null;
}

export interface RecorderConfig {
  outputDir?: string | null;
  fileNamePrefix?: string | null;
}

export interface HotkeyBindings {
  pushToTalk?: string | null;
  toggle?: string | null;
}

export interface ConfigDestinationResponse {
  contractVersion: string;
  outputDestination: OutputDestination;
}

export interface OverlayModeResponse {
  contractVersion: string;
  overlayMode: OverlayMode;
}

// =============================================================================
// COMMANDS
// =============================================================================

// -------------------------
// STT Commands
// -------------------------

/**
 * Bootstraps the Speech-to-Text engine.
 *
 * :param payload: Configuration for bootstrap.
 * :return: Status of the bootstrap operation.
 */
export async function bootstrapVoice(
  payload: BootstrapRequest = {},
): Promise<BootstrapResponse> {
  return await invoke("plugin:voice|bootstrap_voice", { payload });
}

/**
 * Downloads and loads a Whisper model on demand.
 *
 * Blocks until the model is fully ready to transcribe.
 * Subscribe to CHANNELS.PROGRESS before calling to receive live progress events.
 *
 * :param payload: Model ID to download. Must be an allowlisted model.
 * :return: Active model ID and whether it was already loaded.
 * :raises Error: If a download is already in progress or model ID is invalid.
 */
export async function downloadModel(
  payload: DownloadModelRequest,
): Promise<DownloadModelResponse> {
  return await invoke("plugin:voice|download_model", { payload });
}

/**
 * Transcribes an audio file using STT.
 *
 * **Lifecycle Contract:** The STT engine must be in the `ready` state (i.e. startup preloaded)
 * before calling this. If the worker is not preloaded, it will fail immediately with a
 * `notReady` error.
 *
 * :param payload: File path and model ID.
 * :return: The transcribed text.
 */
export async function transcribeFile(
  payload: TranscribeRequest,
): Promise<TranscribeResponse> {
  return await invoke("plugin:voice|transcribe_file", { payload });
}

/**
 * Synthesizes speech to a local WAV file using TTS.
 *
 * :param payload: Text, voice, model, and optional output location.
 * :return: Generated audio file details.
 */
export async function synthesizeSpeech(
  payload: SynthesizeSpeechRequest,
): Promise<SynthesizeSpeechResponse> {
  return await invoke("plugin:voice|synthesize_speech", { payload });
}

/**
 * Starts a streaming TTS synthesis request.
 *
 * Returns immediately with a synthesisId. Audio chunks arrive via the
 * tauri-voice-plugin://tts-stream event channel until complete, error, or
 * cancellation.
 *
 * :param payload: Text and synthesis parameters.
 * :return: Synthesis session metadata including the synthesisId.
 */
export async function streamSpeech(
  payload: StreamSpeechRequest,
): Promise<StreamSpeechResponse> {
  return await invoke("plugin:voice|stream_speech", { payload });
}

/**
 * Cancels an active streaming synthesis by ID.
 *
 * :param payload: synthesisId to cancel.
 * :return: Whether an active stream was cancelled.
 */
export async function cancelSpeech(
  payload: CancelSpeechRequest,
): Promise<CancelSpeechResponse> {
  return await invoke("plugin:voice|cancel_speech", { payload });
}

/**
 * Subscribes to raw TTS stream events from the Rust bridge.
 *
 * :param onEvent: Callback for each incoming TtsStreamEvent.
 * :return: A teardown function that removes the listener.
 */
export async function listenToTtsStream(
  onEvent: (event: TtsStreamEvent) => void,
): Promise<() => void> {
  return await listen<TtsStreamEvent>(CHANNELS.TTS_STREAM, (e) => {
    onEvent(e.payload);
  });
}

/** Initial guest buffer before playback starts (milliseconds). */
const STREAM_BUFFER_START_MS = 400;

/**
 * Synthesizes a Kokoro phrase and plays it through Web Audio.
 *
 * Buffers at least STREAM_BUFFER_START_MS of PCM before scheduling playback.
 * Exposes a cancel() handle that stops further scheduling and calls cancelSpeech.
 *
 * :param request: Text and synthesis options.
 * :return: A handle with a cancel() function.
 * :raises Error: When streamSpeech fails to start.
 */
export async function playStreamedSpeech(
  request: StreamSpeechRequest,
): Promise<StreamPlaybackHandle> {
  const ctx = new AudioContext();
  const response = await streamSpeech(request);
  const { synthesisId, sampleRateHz, channels } = response;

  // Ordered chunk queue for scheduling against AudioContext clock.
  const chunkQueue: Float32Array<ArrayBuffer>[] = [];
  let nextStartTime = 0;
  let bufferedMs = 0;
  let playbackStarted = false;
  let done = false;
  let cancelled = false;

  function decodePcm16(base64: string): Float32Array<ArrayBuffer> {
    // Decode base64 → raw bytes → PCM16 int16 → float32 normalised.
    const binary = atob(base64);
    const raw = new ArrayBuffer(binary.length);
    const bytes = new Uint8Array(raw);
    for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
    const int16 = new Int16Array(raw);
    const float32 = new Float32Array(int16.length);
    for (let i = 0; i < int16.length; i++) float32[i] = int16[i] / 32768;
    return float32;
  }

  function scheduleChunk(samples: Float32Array<ArrayBuffer>) {
    const buffer = ctx.createBuffer(channels, samples.length, sampleRateHz);
    buffer.copyToChannel(samples, 0);
    const source = ctx.createBufferSource();
    source.buffer = buffer;
    source.connect(ctx.destination);

    // Schedule back-to-back; restart from now if underrun.
    const startAt = Math.max(ctx.currentTime, nextStartTime);
    source.start(startAt);
    nextStartTime = startAt + buffer.duration;
  }

  function flushQueue() {
    for (const chunk of chunkQueue) scheduleChunk(chunk);
    chunkQueue.length = 0;
  }

  const unlisten = await listenToTtsStream((event) => {
    if (event.synthesisId !== synthesisId || cancelled) return;

    if (event.type === "chunk" && event.audioBase64) {
      const samples = decodePcm16(event.audioBase64);
      const chunkMs = (samples.length / sampleRateHz) * 1000;
      bufferedMs += chunkMs;

      if (!playbackStarted) {
        chunkQueue.push(samples);
        // Start once enough is buffered or the final chunk has arrived.
        if (bufferedMs >= STREAM_BUFFER_START_MS || event.final) {
          playbackStarted = true;
          nextStartTime = ctx.currentTime;
          flushQueue();
        }
      } else {
        scheduleChunk(samples);
      }
    }

    if (event.type === "complete" || event.type === "error" || event.type === "cancelled") {
      done = true;
      unlisten();
    }
  });

  return {
    async cancel() {
      if (done || cancelled) return;
      cancelled = true;
      unlisten();
      ctx.suspend();
      await cancelSpeech({ synthesisId });
    },
  };
}

/**
 * Gets the health status of the STT engine.
 *
 * :param payload: Health check request.
 * :return: Health status and diagnostics.
 */
export async function voiceHealth(
  payload: HealthRequest = {},
): Promise<HealthResponse> {
  return await invoke("plugin:voice|voice_health", { payload });
}

/**
 * Sets up a one-time hotkey-driven record->transcribe pipeline.
 *
 * :param payload: Pipeline setup configuration.
 * :return: Active pipeline status.
 */
export async function setupRecordTranscribePipeline(
  payload: SetupRecordTranscribePipelineRequest,
): Promise<SetupRecordTranscribePipelineResponse> {
  return await invoke("plugin:voice|setup_record_transcribe_pipeline", { payload });
}

// -------------------------
// Recorder Commands
// -------------------------

/**
 * Initializes the recorder runtime engine.
 *
 * :return: Runtime initialization response.
 */
export async function initializeRecorderRuntime(): Promise<InitializeRuntimeResponse> {
  return await invoke("plugin:voice|initialize_recorder_runtime");
}

/**
 * Starts an audio recording session.
 *
 * :param outputDir: Custom directory for recording output.
 * :param fileNamePrefix: Prefix for the generated file name.
 * :return: Initial recording state and destination info.
 */
export async function startRecording(
  outputDir?: string,
  fileNamePrefix?: string,
): Promise<StartRecordingResponse> {
  return await invoke("plugin:voice|start_recording", {
    outputDir,
    fileNamePrefix,
  });
}

/**
 * Stops an active audio recording session.
 *
 * :param sessionId: ID of the session to stop.
 * :return: Result containing the recorded file path and stats.
 */
export async function stopRecording(
  sessionId?: string,
): Promise<StopRecordingResponse> {
  return await invoke("plugin:voice|stop_recording", { sessionId });
}

/**
 * Sets new hotkey bindings for recording flow.
 *
 * :param bindings: Hotkey mapping config.
 * :param recorderConfig: Audio recorder configuration.
 * :return: The updated runtime state.
 */
export async function setHotkeyBindings(
  bindings: HotkeyBindings,
  recorderConfig: RecorderConfig,
): Promise<SetHotkeyBindingsResponse> {
  return await invoke("plugin:voice|set_hotkey_bindings", {
    bindings,
    recorderConfig,
  });
}

/**
 * Captures the next hotkey sequence pressed by the user.
 *
 * :param timeoutMs: Maximum time to wait for a keypress.
 * :return: Details of the captured hotkey.
 */
export async function captureHotkey(
  timeoutMs?: number,
): Promise<CaptureHotkeyResponse> {
  return await invoke("plugin:voice|capture_hotkey", { timeoutMs });
}

/**
 * Retrieves the current runtime state.
 *
 * :return: State containing readiness and phase.
 */
export async function getRuntimeState(): Promise<GetRuntimeStateResponse> {
  return await invoke("plugin:voice|get_runtime_state");
}

/**
 * Configures output file destination policy.
 *
 * :param policy: Destination policy type.
 * :param baseDir: Custom base directory for output.
 * :param subdir: Optional subdirectory.
 * :param filePattern: Optional file naming pattern.
 * :return: Newly configured destination.
 */
export async function setOutputDestination(
  policy: DestinationPolicy,
  baseDir?: string,
  subdir?: string,
  filePattern?: string,
): Promise<ConfigDestinationResponse> {
  return await invoke("plugin:voice|set_output_destination", {
    policy,
    baseDir,
    subdir,
    filePattern,
  });
}

/**
 * Retrieves current destination policy.
 *
 * :return: Current destination configuration.
 */
export async function getOutputDestination(): Promise<ConfigDestinationResponse> {
  return await invoke("plugin:voice|get_output_destination");
}

/**
 * Sets the overlay mode.
 *
 * :param overlayMode: Type of overlay to use.
 * :return: Confirming response.
 */
export async function setOverlayMode(
  overlayMode: OverlayMode,
): Promise<OverlayModeResponse> {
  return await invoke("plugin:voice|set_overlay_mode", {
    overlayMode,
  });
}

/**
 * Gets the current overlay mode.
 *
 * :return: Confirming response with current mode.
 */
export async function getOverlayMode(): Promise<OverlayModeResponse> {
  return await invoke("plugin:voice|get_overlay_mode");
}

// =============================================================================
// EVENTS & OVERLAYS
// =============================================================================

export const CHANNELS = {
  STATE: "tauri-audio-plugin://state",
  ERROR: "tauri-audio-plugin://error",
  COMPLETE: "tauri-audio-plugin://complete",
  LIVE: "tauri-audio-plugin://live",
  PROGRESS: "tauri-voice-plugin://model-progress",
  TTS_STREAM: "tauri-voice-plugin://tts-stream",
} as const;

export interface StateEvent {
  contractVersion: string;
  eventId: string;
  emittedAtMs: number;
  state: RuntimeState;
  readiness: Readiness;
}

export interface ErrorEvent {
  contractVersion: string;
  eventId: string;
  emittedAtMs: number;
  error: any;
}

export interface CompleteEvent {
  contractVersion: string;
  eventId: string;
  sessionId: string;
  triggerSource: TriggerSource;
  emittedAtMs: number;
  result: RecordingResult;
}

export interface LiveEvent {
  contractVersion: string;
  eventId: string;
  sessionId?: string | null;
  seq: number;
  emittedAtMs: number;
  text: string;
}

export interface OverlayRenderContext {
  state: RuntimeState;
}

export interface OverlayPhaseViews<T> {
  idle?: T;
  recording?: T;
  transcribing?: T;
  error?: T;
  fallback?: T;
}

export function resolveOverlayView<T>(
  state: RuntimeState | null | undefined,
  views: OverlayPhaseViews<T>,
): T | null {
  if (!state) {
    return views.fallback ?? null;
  }

  if (state.phase === "recording") {
    return views.recording ?? views.fallback ?? null;
  }

  if (state.phase === "transcribing") {
    return views.transcribing ?? views.fallback ?? null;
  }

  if (state.phase === "error") {
    return views.error ?? views.fallback ?? null;
  }

  return views.idle ?? views.fallback ?? null;
}

export interface OverlayStateSubscriptionOptions {
  overlayMode?: OverlayMode;
  includeInitialState?: boolean;
}

/**
 * Subscribes to backend overlay state updates.
 *
 * :param onState: Callback upon state change.
 * :param options: Subscription options.
 * :return: A teardown function.
 */
export async function subscribeOverlayState(
  onState: (state: RuntimeState) => void,
  options: OverlayStateSubscriptionOptions = {},
): Promise<() => void> {
  const mode = options.overlayMode ?? { type: "consumer", consumerUrl: "http://localhost:1420#overlay" };
  const includeInitial = options.includeInitialState ?? true;

  await setOverlayMode(mode);

  if (includeInitial) {
    const snapshot = await getRuntimeState();
    onState(snapshot.state);
  }

  return await listen<StateEvent>(CHANNELS.STATE, (event) => {
    onState(event.payload.state);
  });
}

// -----------------------------------------------------------------------------
// Final Transcript Display Helper
// -----------------------------------------------------------------------------

/**
 * A lightweight utility to represent final transcript text visibility window (2 seconds).
 * Call `showTranscript(text)` to make the transcript available for 2 seconds.
 * It will call the `onUpdate` callback with the current text (or null when it hides).
 *
 * :param onUpdate: Callback to execute when transcript text updates.
 * :return: transcript operations handler.
 */
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

export function createTranscriptDisplayState(
  onUpdate: (text: string | null) => void,
) {
  let displayTimer: number | null = null;

  return {
    showTranscript(text: string) {
      if (displayTimer !== null) {
        window.clearTimeout(displayTimer);
      }
      onUpdate(text);
      displayTimer = window.setTimeout(() => {
        onUpdate(null);
        displayTimer = null;
      }, 2000);
    },
    clear() {
      if (displayTimer !== null) {
        window.clearTimeout(displayTimer);
      }
      onUpdate(null);
      displayTimer = null;
    },
  };
}
