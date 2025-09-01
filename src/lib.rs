//! ComfyUI API Proxy library
//!
//! Modules:
//! - `api`: Axum HTTP handlers and router setup used by the binary.
//! - `comfyui`: Thin client for ComfyUI REST endpoints.
//! - `prompt`: Prompt construction helpers with `{{placeholder}}` replacement.
//! - `workflow`: Loading/saving named workflows in `prompts/`.
//! - `utils`: Background helpers like the static drive poller.
//! - `config`: Env-driven configuration loader.
//! - `error`: Common error type and alias.
//!
//! Re-exports are provided for common types: `Config`, `ComfyUIClient`,
//! `PromptConstructor`, and `WorkflowManager`.
pub mod api;
pub mod comfyui;
pub mod prompt;
pub mod workflow;
pub mod utils;
pub mod config;
pub mod error;

pub use config::Config;
pub use comfyui::client::ComfyUIClient;
pub use prompt::constructor::PromptConstructor;
pub use workflow::manager::WorkflowManager;
