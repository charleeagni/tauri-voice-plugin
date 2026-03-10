use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{Error, Result};
use crate::models::{StartRecordingResponse, StopRecordingResponse};

pub struct ActiveRecordingSession {
    pub session_id: String,
    pub started_at: String,
    pub output_path: PathBuf,
    pub process: Child,
}

pub struct RecorderManager {
    pub active_session: Mutex<Option<ActiveRecordingSession>>,
}

impl RecorderManager {
    pub fn new() -> Self {
        Self {
            active_session: Mutex::new(None),
        }
    }

    pub fn start_recording(
        &self,
        output_dir: Option<String>,
        file_name_prefix: Option<String>,
        default_dir: PathBuf,
    ) -> Result<StartRecordingResponse> {
        let mut session_guard = self.active_session.lock().unwrap();

        if session_guard.is_some() {
            return Err(Error::recording_in_progress(
                "A recording is already active",
            ));
        }

        let dir = if let Some(dir_str) = output_dir {
            let p = PathBuf::from(dir_str);
            if !p.is_absolute() {
                return Err(Error::invalid_input("outputDir must be absolute"));
            }
            p
        } else {
            default_dir
        };

        if !dir.exists() {
            std::fs::create_dir_all(&dir).map_err(|e| {
                Error::recording_start_failed(format!("Failed to create output dir: {}", e))
            })?;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let prefix = file_name_prefix.unwrap_or_else(|| "recording".to_string());

        let file_name = format!("{}-{}.wav", prefix, now);
        let output_path = dir.join(file_name);

        // macOS recording requires ffmpeg or similar in PATH
        let process = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "PATH=$PATH:/opt/homebrew/bin:/usr/local/bin exec ffmpeg -fflags nobuffer -probesize 32 -analyzeduration 0 -flags low_delay -f avfoundation -i \":default\" -y \"{}\"",
                output_path.display()
            ))
            .spawn()
            .map_err(|e| Error::recording_start_failed(format!("Failed to start ffmpeg: {}", e)))?;

        let session_id = format!("ses-{}", now);
        let started_at = format!("{}", now);

        let res = StartRecordingResponse {
            session_id: session_id.clone(),
            status: "recording".to_string(),
            started_at: started_at.clone(),
        };

        *session_guard = Some(ActiveRecordingSession {
            session_id,
            started_at,
            output_path,
            process,
        });

        Ok(res)
    }

    pub fn stop_recording(&self, session_id: String) -> Result<StopRecordingResponse> {
        let mut session_guard = self.active_session.lock().unwrap();

        let mut session = session_guard
            .take()
            .ok_or_else(|| Error::recording_not_active("No active session exists"))?;

        if session.session_id != session_id {
            let _ = std::mem::replace(&mut *session_guard, Some(session));
            return Err(Error::invalid_input("Session ID mismatch"));
        }

        // Stop the ffmpeg process gracefully using SIGINT
        // Since we used 'exec' in the shell command, the child process IS ffmpeg.
        let _ = Command::new("kill")
            .arg("-INT")
            .arg(session.process.id().to_string())
            .status();

        // Wait for FFmpeg to finish closing the file and exit gracefully
        let _ = session.process.wait();

        let now_millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let start_millis: u64 = session.started_at.parse().unwrap_or(0);
        let duration_ms = (now_millis as u64).saturating_sub(start_millis);

        if !session.output_path.exists() {
            return Err(Error::recording_stop_failed("Output file was not created"));
        }

        // Wait a small amount to let FFmpeg flush
        std::thread::sleep(std::time::Duration::from_millis(50));

        let meta = std::fs::metadata(&session.output_path).unwrap();
        if meta.len() == 0 {
            return Err(Error::recording_stop_failed("Output file is empty"));
        }

        Ok(StopRecordingResponse {
            status: "stopped".to_string(),
            path: session.output_path.to_string_lossy().into_owned(),
            duration_ms,
        })
    }
}
