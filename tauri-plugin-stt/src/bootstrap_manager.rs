use tauri::{AppHandle, Manager, Runtime};
use std::fs;
use std::collections::HashMap;

use crate::models::{BootstrapRequest, BootstrapResponse};
use crate::{Error, Result};
use crate::sidecar_uv::UvSidecarRunner;

const PYTHON_VERSION: &str = "3.12";
const LOCK_FILE_PATH: &str = "requirements/requirements-stt.lock.txt";

pub struct BootstrapManager;

impl BootstrapManager {
    pub async fn bootstrap_stt<R: Runtime>(app: &AppHandle<R>, _payload: BootstrapRequest) -> Result<BootstrapResponse> {
        let app_data_dir = app.path().app_data_dir().map_err(|e| Error::bootstrap_failed(format!("Failed to get app data dir: {}", e)))?;
        let python_dir = app_data_dir.join("python");
        
        if !python_dir.exists() {
            println!("Creating python directory at: {:?}", python_dir);
            fs::create_dir_all(&python_dir).map_err(|e| Error::bootstrap_failed(format!("Failed to create python dir: {}", e)))?;
        }

        let venv_dir = python_dir.join(".venv");
        let python_bin = venv_dir.join("bin").join("python");
        
        // Resolve lock file from plugin crate source, not host app current directory.
        let lock_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(LOCK_FILE_PATH);

        if !lock_file.exists() {
            return Err(Error::not_ready(format!("Lock file missing at: {}", lock_file.display())));
        }

        let cache_dir = python_dir.join("cache");
        let tool_dir = python_dir.join("tools");
        
        if !cache_dir.exists() {
            println!("Creating cache directory at: {:?}", cache_dir);
            fs::create_dir_all(&cache_dir).map_err(|e| Error::bootstrap_failed(format!("Failed to create cache dir: {}", e)))?;
        }
        if !tool_dir.exists() {
            println!("Creating tools directory at: {:?}", tool_dir);
            fs::create_dir_all(&tool_dir).map_err(|e| Error::bootstrap_failed(format!("Failed to create tool dir: {}", e)))?;
        }

        let mut envs = HashMap::new();
        envs.insert("UV_CACHE_DIR".to_string(), cache_dir.to_string_lossy().to_string());
        envs.insert("UV_TOOL_DIR".to_string(), tool_dir.to_string_lossy().to_string());
        envs.insert("UV_PYTHON_INSTALL".to_string(), "1".to_string());
        envs.insert("UV_PYTHON_DOWNLOADS".to_string(), "auto".to_string());

        // 1. Create venv
        println!("Creating python venv (version {})...", PYTHON_VERSION);
        UvSidecarRunner::create_venv(app, &venv_dir, PYTHON_VERSION, &envs).await?;

        // 2. Install dependencies
        println!("Installing dependencies from lockfile: {:?}", lock_file);
        UvSidecarRunner::pip_install(app, &python_bin, &lock_file, &envs).await?;

        Ok(BootstrapResponse::Ready {
            details: "Bootstrap complete".into(),
        })
    }
}
