import { invoke } from '@tauri-apps/api/core'

export interface StartRecordingOptions {
  outputDir?: string
  fileNamePrefix?: string
}

export interface StartRecordingResult {
  sessionId: string
  status: string
  startedAt: string
}

export interface StopRecordingResult {
  status: string
  path: string
  durationMs: number
}

export async function startRecording(
  options?: StartRecordingOptions
): Promise<StartRecordingResult> {
  // Keep payload camelCase for Rust serde compatibility.

  const payload = options
    ? {
      outputDir: options.outputDir,
      fileNamePrefix: options.fileNamePrefix,
    }
    : {}

  return await invoke<StartRecordingResult>(
    'plugin:tauri-plugin-stt|start_recording',
    { payload }
  )
}

export async function stopRecording(
  sessionId: string
): Promise<StopRecordingResult> {
  // Keep wrapper thin over invoke command.

  return await invoke<StopRecordingResult>(
    'plugin:tauri-plugin-stt|stop_recording',
    {
      payload: { sessionId },
    }
  )
}
