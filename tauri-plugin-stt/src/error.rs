use serde::{ser::SerializeStruct, Serialize, Serializer};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[cfg(mobile)]
    #[error(transparent)]
    PluginInvoke(#[from] tauri::plugin::mobile::PluginInvokeError),

    #[error("Bootstrap failed: {message}")]
    BootstrapFailed {
        message: String,
        details: Option<String>,
    },
    #[error("Dependency install failed: {message}")]
    DependencyInstallFailed {
        message: String,
        details: Option<String>,
    },
    #[error("Model download failed: {message}")]
    ModelDownloadFailed {
        message: String,
        details: Option<String>,
    },
    #[error("Generation failed: {message}")]
    GenerationFailed {
        message: String,
        details: Option<String>,
    },
    #[error("Invalid input: {message}")]
    InvalidInput {
        message: String,
        details: Option<String>,
    },
    #[error("Not ready: {message}")]
    NotReady {
        message: String,
        details: Option<String>,
    },
    #[error("Feature disabled: {message}")]
    FeatureDisabled {
        message: String,
        details: Option<String>,
    },
}

impl Error {
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::InvalidInput {
            message: msg.into(),
            details: None,
        }
    }
    pub fn not_ready(msg: impl Into<String>) -> Self {
        Self::NotReady {
            message: msg.into(),
            details: None,
        }
    }
    pub fn bootstrap_failed(msg: impl Into<String>) -> Self {
        Self::BootstrapFailed {
            message: msg.into(),
            details: None,
        }
    }
    pub fn dependency_install_failed(msg: impl Into<String>) -> Self {
        Self::DependencyInstallFailed {
            message: msg.into(),
            details: None,
        }
    }
    pub fn model_download_failed(msg: impl Into<String>) -> Self {
        Self::ModelDownloadFailed {
            message: msg.into(),
            details: None,
        }
    }
    pub fn generation_failed(msg: impl Into<String>) -> Self {
        Self::GenerationFailed {
            message: msg.into(),
            details: None,
        }
    }
    pub fn feature_disabled(msg: impl Into<String>) -> Self {
        Self::FeatureDisabled {
            message: msg.into(),
            details: None,
        }
    }
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Error", 3)?;
        match self {
            Error::Io(e) => {
                state.serialize_field("code", "internal_error")?;
                state.serialize_field("message", &e.to_string())?;
                state.serialize_field("details", &None::<String>)?;
            }
            #[cfg(mobile)]
            Error::PluginInvoke(e) => {
                state.serialize_field("code", "internal_error")?;
                state.serialize_field("message", &e.to_string())?;
                state.serialize_field("details", &None::<String>)?;
            }
            Error::BootstrapFailed { message, details } => {
                state.serialize_field("code", "bootstrap_failed")?;
                state.serialize_field("message", message)?;
                state.serialize_field("details", details)?;
            }
            Error::DependencyInstallFailed { message, details } => {
                state.serialize_field("code", "dependency_install_failed")?;
                state.serialize_field("message", message)?;
                state.serialize_field("details", details)?;
            }
            Error::ModelDownloadFailed { message, details } => {
                state.serialize_field("code", "model_download_failed")?;
                state.serialize_field("message", message)?;
                state.serialize_field("details", details)?;
            }
            Error::GenerationFailed { message, details } => {
                state.serialize_field("code", "generation_failed")?;
                state.serialize_field("message", message)?;
                state.serialize_field("details", details)?;
            }
            Error::InvalidInput { message, details } => {
                state.serialize_field("code", "invalid_input")?;
                state.serialize_field("message", message)?;
                state.serialize_field("details", details)?;
            }
            Error::NotReady { message, details } => {
                state.serialize_field("code", "not_ready")?;
                state.serialize_field("message", message)?;
                state.serialize_field("details", details)?;
            }
            Error::FeatureDisabled { message, details } => {
                state.serialize_field("code", "feature_disabled")?;
                state.serialize_field("message", message)?;
                state.serialize_field("details", details)?;
            }
        }
        state.end()
    }
}
