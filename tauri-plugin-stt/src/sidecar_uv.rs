use tauri::AppHandle;
use tauri_plugin_shell::{process::CommandEvent, ShellExt};
use tauri::Runtime;
use std::path::PathBuf;
use std::collections::HashMap;

use crate::{Error, Result};

pub struct UvSidecarRunner;

impl UvSidecarRunner {
    pub async fn create_venv<R: Runtime>(
        app: &AppHandle<R>, 
        venv_path: &PathBuf, 
        python_version: &str,
        envs: &HashMap<String, String>,
    ) -> Result<()> {
        let venv_path_str = venv_path.to_string_lossy().to_string();
        
        let (mut rx, _child) = app
            .shell()
            .sidecar("uv")
            .map_err(|e| Error::bootstrap_failed(format!("Failed to spawn uv sidecar: {}", e)))?
            .args(["venv", &venv_path_str, "--python", python_version, "--clear"])
            .envs(envs.clone())
            .spawn()
            .map_err(|e| Error::bootstrap_failed(format!("Failed to execute uv venv: {}", e)))?;

        let mut stderr_lines = Vec::new();
        let mut success = false;
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    println!("[uv venv] {}", String::from_utf8_lossy(&line));
                }
                CommandEvent::Stderr(line) => {
                    let s = String::from_utf8_lossy(&line).into_owned();
                    eprintln!("[uv venv error] {}", s);
                    stderr_lines.push(s);
                }
                CommandEvent::Error(err) => {
                    return Err(Error::bootstrap_failed(format!("uv venv error: {}\nStderr: {}", err, stderr_lines.join("\n"))));
                }
                CommandEvent::Terminated(payload) => {
                    if payload.code.unwrap_or(-1) == 0 {
                        success = true;
                    } else {
                        return Err(Error::bootstrap_failed(format!("uv venv exited with code: {:?}\nStderr: {}", payload.code, stderr_lines.join("\n"))));
                    }
                }
                _ => {}
            }
        }
        
        if !success {
            return Err(Error::bootstrap_failed(format!("uv venv did not complete successfully. Stderr: {}", stderr_lines.join("\n"))));
        }

        Ok(())
    }

    pub async fn pip_install<R: Runtime>(
        app: &AppHandle<R>, 
        python_path: &PathBuf, 
        lock_file: &PathBuf,
        envs: &HashMap<String, String>,
    ) -> Result<()> {
        let python_path_str = python_path.to_string_lossy().to_string();
        let lock_file_str = lock_file.to_string_lossy().to_string();

        let (mut rx, _child) = app
            .shell()
            .sidecar("uv")
            .map_err(|e| Error::dependency_install_failed(format!("Failed to spawn uv sidecar: {}", e)))?
            .args(["pip", "install", "--python", &python_path_str, "-r", &lock_file_str])
            .envs(envs.clone())
            .spawn()
            .map_err(|e| Error::dependency_install_failed(format!("Failed to execute uv pip install: {}", e)))?;

        let mut stderr_lines = Vec::new();
        let mut success = false;
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    println!("[uv pip] {}", String::from_utf8_lossy(&line));
                }
                CommandEvent::Stderr(line) => {
                    let s = String::from_utf8_lossy(&line).into_owned();
                    eprintln!("[uv pip error] {}", s);
                    stderr_lines.push(s);
                }
                CommandEvent::Error(err) => {
                    return Err(Error::dependency_install_failed(format!("uv pip install error: {}\nStderr: {}", err, stderr_lines.join("\n"))));
                }
                CommandEvent::Terminated(payload) => {
                    if payload.code.unwrap_or(-1) == 0 {
                        success = true;
                    } else {
                        return Err(Error::dependency_install_failed(format!("uv pip install exited with code: {:?}\nStderr: {}", payload.code, stderr_lines.join("\n"))));
                    }
                }
                _ => {}
            }
        }
        
        if !success {
            return Err(Error::dependency_install_failed(format!("uv pip install did not complete successfully\nStderr: {}", stderr_lines.join("\n"))));
        }

        Ok(())
    }
}
