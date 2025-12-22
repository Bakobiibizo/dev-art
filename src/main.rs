
use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tokio::sync::RwLock;

use comfyui_api_proxy::{
    comfyui, 
    api,
    config,
    utils,
    prompt,
    workflow,
};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load configuration
    config::Config::dotenv_load();
    let config = config::Config::new().expect("Failed to load configuration");
    config::Config::print_env_vars();
    // Create ComfyUI client
    let comfyui_client = comfyui::client::ComfyUIClient::new(config.comfyui_url.clone());
    let static_drive_poller = utils::static_drive_poller::StaticDrivePoller::new(config.static_drive_path.clone());

    tokio::spawn(async move {
        static_drive_poller.start_polling().await;
    });
    let state = Arc::new(api::routes::AppState {
        prompt_constructor: RwLock::new(prompt::constructor::PromptConstructor::new()),
        comfyui_client,
        workflow_manager: RwLock::new(workflow::manager::WorkflowManager::new()),
        static_drive_poller: Arc::new(utils::static_drive_poller::StaticDrivePoller::new(config.static_drive_path.clone())),
        prompts_dir: config.prompts_dir.clone(),
    });

    // Build our application with a route
    let app = Router::new()
        .route("/", get(|| async { "ComfyUI API Proxy" }))
        .route("/queue_prompt", post(api::handlers::queue_prompt))
        .route("/get_image", get(api::handlers::get_image))
        .route("/get_history", get(api::handlers::get_history))
        .route("/history", get(api::handlers::history_friendly))
        .route("/add_workflow", post(api::handlers::add_workflow))
        .route("/get_node_info", get(api::handlers::get_node_info))
        .route("/construct_prompt", post(api::handlers::construct_prompt))
        .route("/models", get(api::handlers::models_categories))
        .route("/models/checkpoints", get(api::handlers::models_checkpoints))
        .route("/models/:category", get(api::handlers::models_in_category))
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Run our application with safe parsing
    let host_str = config.api_host.clone();
    let port_str = config.api_port.clone();
    let ip: std::net::IpAddr = host_str.parse().unwrap_or_else(|_| {
        tracing::warn!("Invalid API_HOST '{}', falling back to 127.0.0.1", host_str);
        std::net::IpAddr::from([127, 0, 0, 1])
    });
    let port: u16 = port_str.parse().unwrap_or_else(|_| {
        tracing::warn!("Invalid API_PORT '{}', falling back to 8189", port_str);
        8189
    });
    let socket_address = SocketAddr::new(ip, port);
    tracing::info!("listening on {}", socket_address);
    axum::Server::bind(&socket_address)

        .serve(app.into_make_service())
        .await
        .unwrap();
}
