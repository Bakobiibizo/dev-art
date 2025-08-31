
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
    let config = config::Config::new().expect("Failed to load configuration");
    config::Config::dotenv_load();
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
    });

    // Build our application with a route
    let app = Router::new()
        .route("/", get(|| async { "ComfyUI API Proxy" }))
        .route("/queue_prompt", post(api::handlers::queue_prompt))
        .route("/get_image", get(api::handlers::get_image))
        .route("/get_history", get(api::handlers::get_history))
        .route("/add_workflow", post(api::handlers::add_workflow))
        .route("/get_node_info", get(api::handlers::get_node_info))
        .route("/construct_prompt", post(api::handlers::construct_prompt))
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Run our application
    let ip = config.api_host;
    let port = config.api_port;
    let socket_address = SocketAddr::new(ip.parse().unwrap(), port.parse().unwrap());
    tracing::info!("listening on {}", socket_address);
    axum::Server::bind(&socket_address)
        .serve(app.into_make_service())
        .await
        .unwrap();
}