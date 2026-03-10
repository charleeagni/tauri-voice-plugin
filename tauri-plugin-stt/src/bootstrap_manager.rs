use tauri::{AppHandle, Manager, Runtime};
use std::fs;

use crate::models::{BootstrapRequest, BootstrapResponse};
use crate::{Error, Result};
use crate::sidecar_uv::UvSidecarRunner;

const PYTHON_VERSION: &str = "3.14";
const LOCK_FILE_PATH: &str = "requirements/requirements-stt.lock.txt";

pub struct BootstrapManager;

impl BootstrapManager {
    pub async fn bootstrap_stt<R: Runtime>(app: &AppHandle<R>, _payload: BootstrapRequest) -> Result<BootstrapResponse> {
        let app_data_dir = app.path().app_data_dir().map_err(|e| Error::bootstrap_failed(format!("Failed to get app data dir: {}", e)))?;
        let python_dir = app_data_dir.join("python");
        
        if !python_dir.exists() {
            fs::create_dir_all(&python_dir).map_err(|e| Error::bootstrap_failed(format!("Failed to create python dir: {}", e)))?;
        }

        let venv_dir = python_dir.join(".venv");
        let python_bin = venv_dir.join("bin").join("python");
        
        // Resolve lock file from plugin crate source, not host app current directory.
        let lock_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(LOCK_FILE_PATH);

        if !lock_file.exists() {
            return Err(Error::not_ready(format!("Lock file missing at: {}", lock_file.display())));
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
