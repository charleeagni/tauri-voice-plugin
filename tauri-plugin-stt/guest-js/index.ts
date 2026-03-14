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

export interface TranscribeRequest {
  path: string;
  modelId?: string;
}
export interface TranscribeResponse {
  text: string;
}

export interface HealthRequest {}
export interface DiagnosticEntry {
  name: string;
  ready: boolean;
  reason?: string;
}
export type HealthResponse =
  | { status: "ready"; diagnostics: DiagnosticEntry[] }
  | { status: "notReady"; reason: string; diagnostics: DiagnosticEntry[] };

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
export type OverlayMode = "default" | "consumer" | "disabled";
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
export async function bootstrapStt(
  payload: BootstrapRequest = {},
): Promise<BootstrapResponse> {
  return await invoke("plugin:stt|bootstrap_stt", { payload });
}

/**
 * Transcribes an audio file using STT.
 *
 * :param payload: File path and model ID.
 * :return: The transcribed text.
 */
export async function transcribeFile(
  payload: TranscribeRequest,
): Promise<TranscribeResponse> {
  return await invoke("plugin:stt|transcribe_file", { payload });
}

/**
 * Gets the health status of the STT engine.
 *
 * :param payload: Health check request.
 * :return: Health status and diagnostics.
 */
export async function sttHealth(
  payload: HealthRequest = {},
): Promise<HealthResponse> {
  return await invoke("plugin:stt|stt_health", { payload });
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
  return await invoke("plugin:stt|initialize_recorder_runtime");
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
  return await invoke("plugin:stt|start_recording", {
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
  return await invoke("plugin:stt|stop_recording", { sessionId });
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
  return await invoke("plugin:stt|set_hotkey_bindings", {
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
  return await invoke("plugin:stt|capture_hotkey", { timeoutMs });
}

/**
 * Retrieves the current runtime state.
 *
 * :return: State containing readiness and phase.
 */
export async function getRuntimeState(): Promise<GetRuntimeStateResponse> {
  return await invoke("plugin:stt|get_runtime_state");
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
  return await invoke("plugin:stt|set_output_destination", {
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
  return await invoke("plugin:stt|get_output_destination");
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
  return await invoke("plugin:stt|set_overlay_mode", {
    overlayMode,
  });
}

/**
 * Gets the current overlay mode.
 *
 * :return: Confirming response with current mode.
 */
export async function getOverlayMode(): Promise<OverlayModeResponse> {
  return await invoke("plugin:stt|get_overlay_mode");
}

// =============================================================================
// EVENTS & OVERLAYS
// =============================================================================

export const CHANNELS = {
  STATE: "tauri-audio-plugin://state",
  ERROR: "tauri-audio-plugin://error",
  COMPLETE: "tauri-audio-plugin://complete",
  LIVE: "tauri-audio-plugin://live",
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
  const mode = options.overlayMode ?? "consumer";
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
