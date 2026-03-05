use tauri::AppHandle;
use tauri_plugin_shell::{process::CommandEvent, ShellExt};
use tauri::Runtime;
use std::path::PathBuf;

use crate::{Error, Result};

pub struct UvSidecarRunner;

impl UvSidecarRunner {
    pub async fn create_venv<R: Runtime>(app: &AppHandle<R>, venv_path: &PathBuf, python_version: &str) -> Result<()> {
        let venv_path_str = venv_path.to_string_lossy().to_string();
        
        let (mut rx, _child) = app
            .shell()
            .sidecar("uv")
            .map_err(|e| Error::BootstrapFailed(format!("Failed to spawn uv sidecar: {}", e)))?
            .args(["venv", &venv_path_str, "--python", python_version])
            .spawn()
            .map_err(|e| Error::BootstrapFailed(format!("Failed to execute uv venv: {}", e)))?;

        let mut success = false;
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(_) => {}
                CommandEvent::Stderr(_) => {}
                CommandEvent::Error(err) => {
                    return Err(Error::BootstrapFailed(format!("uv venv error: {}", err)));
                }
                CommandEvent::Terminated(payload) => {
                    if payload.code.unwrap_or(-1) == 0 {
                        success = true;
                    } else {
                        return Err(Error::BootstrapFailed(format!("uv venv exited with code: {:?}", payload.code)));
                    }
                }
                _ => {}
            }
        }
        
        if !success {
            return Err(Error::BootstrapFailed("uv venv did not complete successfully".into()));
        }

        Ok(())
    }

    pub async fn pip_install<R: Runtime>(app: &AppHandle<R>, python_path: &PathBuf, lock_file: &PathBuf) -> Result<()> {
        let python_path_str = python_path.to_string_lossy().to_string();
        let lock_file_str = lock_file.to_string_lossy().to_string();

        let (mut rx, _child) = app
            .shell()
            .sidecar("uv")
            .map_err(|e| Error::DependencyInstallFailed(format!("Failed to spawn uv sidecar: {}", e)))?
            .args(["pip", "install", "--python", &python_path_str, "-r", &lock_file_str])
            .spawn()
            .map_err(|e| Error::DependencyInstallFailed(format!("Failed to execute uv pip install: {}", e)))?;

        let mut success = false;
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(_) => {}
                CommandEvent::Stderr(_) => {}
                CommandEvent::Error(err) => {
                    return Err(Error::DependencyInstallFailed(format!("uv pip install error: {}", err)));
                }
                CommandEvent::Terminated(payload) => {
                    if payload.code.unwrap_or(-1) == 0 {
                        success = true;
                    } else {
                        return Err(Error::DependencyInstallFailed(format!("uv pip install exited with code: {:?}", payload.code)));
                    }
                }
                _ => {}
            }
        }
        
        if !success {
            return Err(Error::DependencyInstallFailed("uv pip install did not complete successfully".into()));
        }

        Ok(())
    }
}
