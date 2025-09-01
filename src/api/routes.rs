//! HTTP router setup for the Axum server.
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::comfyui::client::ComfyUIClient;
use crate::prompt::constructor::PromptConstructor;
use crate::workflow::manager::WorkflowManager;
use crate::api::handlers;  // Import the handlers
use crate::utils::static_drive_poller::StaticDrivePoller;
use crate::config::Config;


pub struct AppState {
    pub prompt_constructor: RwLock<PromptConstructor>,
    pub comfyui_client: ComfyUIClient,
    pub workflow_manager: RwLock<WorkflowManager>,
    pub static_drive_poller: Arc<StaticDrivePoller>,
    pub prompts_dir: String,
}

pub fn setup_routes(comfyui_client: ComfyUIClient) -> Router {
    let config = Config::new().expect("Failed to load configuration");
    let state = Arc::new(AppState {
        comfyui_client,
        prompt_constructor: RwLock::new(PromptConstructor::new()),
        workflow_manager: RwLock::new(WorkflowManager::new()),
        static_drive_poller: Arc::new(StaticDrivePoller::new(config.static_drive_path.clone())),
        prompts_dir: config.prompts_dir.clone(),
    });

    Router::new()
        .route("/", get(handlers::root))
        .route("/queue_prompt", post(handlers::queue_prompt))
        .route("/get_image", get(handlers::get_image))
        .route("/history", get(handlers::get_history))
        .route("/add_workflow", post(handlers::add_workflow))
        .route("/get_node_info", get(handlers::get_node_info))
        .route("/construct_prompt", post(handlers::construct_prompt))
        .with_state(state)
}
