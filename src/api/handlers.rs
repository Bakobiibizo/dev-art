//! Axum request handlers for the HTTP API.
use axum::{extract::{Query, State}, Json};
use serde_json::{Value, json, from_str};
use std::sync::Arc;
use tokio::fs;

use crate::api::routes::AppState;

pub async fn root() -> &'static str {
    "ComfyUI API Proxy"
}

pub async fn queue_prompt(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, String> {
    let workflow_name = payload.get("workflow")
        .and_then(|v| v.as_str())
        .ok_or("Workflow name is required")?;

    // Load the workflow from file
    let workflow_path = format!("prompts/{}.json", workflow_name);
    let workflow_content = fs::read_to_string(&workflow_path)
        .await
        .map_err(|e| format!("Failed to read workflow file: {}", e))?;

    let workflow: Value = from_str(&workflow_content)
        .map_err(|e| format!("Failed to parse workflow JSON: {}", e))?;

    // Ensure payload matches ComfyUI's expected shape {"prompt": {...}}
    let body = if workflow.get("prompt").is_some() {
        workflow
    } else {
        json!({"prompt": workflow})
    };

    // Use the loaded workflow as the body of the request
    state.comfyui_client.queue_prompt(body)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to queue prompt: {:?}", e);
            e.to_string()
        })
}

pub async fn get_name(Query(params): Query<std::collections::HashMap<String, String>>) -> String {
    let default = String::from("sdxl");
    let name = params.get("name").ok_or(&default).unwrap_or(&default);
    name.to_string()
}

pub async fn get_image(
    State(state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Vec<u8>, String> {
    let filename = params.get("filename").ok_or("Filename is required")?;
    state.comfyui_client.get_image(filename)
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_history(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, String> {
    state.comfyui_client.get_history()
        .await
        .map(Json)
        .map_err(|e| e.to_string())
}

pub async fn add_workflow(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, String> {
    let workflow_name = payload.get("name").and_then(|v| v.as_str()).map(String::from);
    let workflow = payload.get("workflow").cloned();

    if workflow_name.is_none() && workflow.is_none() {
        return Err("Either 'name' or 'workflow' must be provided".to_string());
    }

    let mut workflow_manager = state.workflow_manager.write().await;
    workflow_manager
        .add_workflow(workflow_name, workflow)
        .await
        .map(|_| Json(json!({"status": "success"})))
        .map_err(|e| e.to_string())
}

pub async fn get_node_info(
    State(state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Value>, String> {
    let node_type = params.get("node_type").ok_or("Node type is required")?;
    state.workflow_manager.read().await.get_node_info(node_type)
        .map(Json)
        .ok_or_else(|| "Node type not found".to_string())
}

pub async fn construct_prompt(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, String> {
    let template = payload.get("template").ok_or("Template is required")?;
    let inputs = payload.get("inputs").ok_or("Inputs are required")?;
    println!("Constructing prompt with template: {}", template);
    println!("Inputs: {}", inputs);
    state.prompt_constructor.read().await
        .construct_prompt(template, inputs)
        .map(Json)
        .map_err(|e| e.to_string())
}
