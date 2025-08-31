//! Central error definitions used across the crate.
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    #[error("JSON serialization error: {0}")]
    JsonSerialization(#[from] serde_json::Error),

    #[error("Prompt construction error: {0}")]
    PromptConstruction(String),

    #[error("ComfyUI error: {0}")]
    ComfyUI(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Workflow management error: {0}")]
    WorkflowManagement(String),

    #[error("Static drive polling error: {0}")]
    StaticDrivePolling(String),
}

pub type AppResult<T> = Result<T, AppError>;
