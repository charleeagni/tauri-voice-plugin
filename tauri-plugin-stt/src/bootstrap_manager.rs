use std::path::PathBuf;
use tauri::{AppHandle, Manager, Runtime};
use std::fs;

use crate::models::{BootstrapRequest, BootstrapResponse};
use crate::{Error, Result};
use crate::sidecar_uv::UvSidecarRunner;

const PYTHON_VERSION: &str = "3.14";
const LOCK_FILE_PATH: &str = "tauri-plugin-stt/requirements/requirements-stt.lock.txt";

pub struct BootstrapManager;

impl BootstrapManager {
    pub async fn bootstrap_stt<R: Runtime>(app: &AppHandle<R>, _payload: BootstrapRequest) -> Result<BootstrapResponse> {
        let app_data_dir = app.path().app_data_dir().map_err(|e| Error::BootstrapFailed(format!("Failed to get app data dir: {}", e)))?;
        let python_dir = app_data_dir.join("python");
        
        if !python_dir.exists() {
            fs::create_dir_all(&python_dir).map_err(|e| Error::BootstrapFailed(format!("Failed to create python dir: {}", e)))?;
        }

        let venv_dir = python_dir.join(".venv");
        let python_bin = venv_dir.join("bin").join("python");
        
        // Ensure path where we started from to find lock file relative
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let lock_file = current_dir.join(LOCK_FILE_PATH);

        if !lock_file.exists() {
            return Err(Error::NotReady(format!("Lock file missing at: {}", lock_file.display())));
        }

        // 1. Create venv
        UvSidecarRunner::create_venv(app, &venv_dir, PYTHON_VERSION).await?;

        // 2. Install dependencies
        UvSidecarRunner::pip_install(app, &python_bin, &lock_file).await?;

        Ok(BootstrapResponse::Ready {
            details: "Bootstrap complete".into(),
        })
    }
}
