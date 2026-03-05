use serde::{ser::Serializer, Serialize};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error(transparent)]
  Io(#[from] std::io::Error),
  #[cfg(mobile)]
  #[error(transparent)]
  PluginInvoke(#[from] tauri::plugin::mobile::PluginInvokeError),
  
  #[error("Bootstrap failed: {0}")]
  BootstrapFailed(String),
  #[error("Dependency install failed: {0}")]
  DependencyInstallFailed(String),
  #[error("Model download failed: {0}")]
  ModelDownloadFailed(String),
  #[error("Generation failed: {0}")]
  GenerationFailed(String),
  #[error("Invalid input: {0}")]
  InvalidInput(String),
  #[error("Not ready: {0}")]
  NotReady(String),
}

impl Serialize for Error {
  fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.serialize_str(self.to_string().as_ref())
  }
}
