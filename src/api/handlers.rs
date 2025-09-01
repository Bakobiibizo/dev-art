//! Axum request handlers for the HTTP API.
use axum::{extract::{Query, State}, Json};
use axum::extract::Path;
use axum::response::IntoResponse;
use serde_json::{Value, json};
use std::sync::Arc;
// use tokio::fs; // not needed in this module after refactor

use crate::api::routes::AppState;
use crate::utils::prompt_build::{resolve_prompt_root_from_payload, apply_overrides_from_payload, ensure_defaults_on_root, maybe_log_verbose};

pub async fn root() -> &'static str {
    "ComfyUI API Proxy"
}

pub async fn queue_prompt(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, String> {
    // Resolve base {"prompt": {...}}
    let mut root = resolve_prompt_root_from_payload(&payload, &state.prompts_dir).await?;
    apply_overrides_from_payload(&mut root, &payload)?;
    ensure_defaults_on_root(&mut root, payload.get("filename_prefix").and_then(|v| v.as_str()));
    maybe_log_verbose(&root, payload.get("verbose").and_then(|v| v.as_bool()).unwrap_or(false));

    // Use the constructed body for the request
    state.comfyui_client.queue_prompt(root)
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

// Friendly history endpoint: defaults to human-readable lines; add ?json=true for raw JSON
pub async fn history_friendly(
    State(state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, String> {
    let json_flag = params.get("json").map(|v| v == "true" || v == "1").unwrap_or(false);
    let hist = state.comfyui_client.get_history().await.map_err(|e| e.to_string())?;
    if json_flag {
        return Ok(Json(hist).into_response());
    }
    if let Some(pid) = params.get("prompt_id") {
        let mut files = Vec::new();
        collect_filenames_for_id(&hist, pid, &mut files);
        let body = if files.is_empty() { String::new() } else { files.join("\n") };
        Ok(body.into_response())
    } else {
        let mut ids = Vec::new();
        collect_prompt_ids(&hist, &mut ids);
        let body = if ids.is_empty() { String::new() } else { ids.join("\n") };
        Ok(body.into_response())
    }
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

// Models: list categories
pub async fn models_categories(
    State(state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, String> {
    let json_flag = params.get("json").map(|v| v == "true" || v == "1").unwrap_or(false);
    let v = state.comfyui_client.get_model_categories().await.map_err(|e| e.to_string())?;
    if json_flag {
        Ok(Json(v).into_response())
    } else if let Some(arr) = v.as_array() {
        let mut lines = String::new();
        for it in arr {
            match it {
                Value::String(s) => { lines.push_str(s); lines.push('\n'); }
                _ => { lines.push_str(&it.to_string()); lines.push('\n'); }
            }
        }
        Ok(lines.into_response())
    } else {
        Ok(serde_json::to_string_pretty(&v).unwrap_or_default().into_response())
    }
}

// Models: list for a category (e.g., checkpoints)
pub async fn models_in_category(
    State(state): State<Arc<AppState>>,
    Path(category): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, String> {
    let json_flag = params.get("json").map(|v| v == "true" || v == "1").unwrap_or(false);
    let v = state.comfyui_client.get_models_in_category(&category).await.map_err(|e| e.to_string())?;
    if json_flag {
        Ok(Json(v).into_response())
    } else if let Some(arr) = v.as_array() {
        let mut lines = String::new();
        for item in arr {
            match item {
                Value::String(s) => {
                    lines.push_str(s);
                    lines.push('\n');
                }
                Value::Object(o) => {
                    if let Some(name) = o.get("name").and_then(|x| x.as_str()) {
                        lines.push_str(name);
                        lines.push('\n');
                    } else {
                        lines.push_str(&serde_json::to_string_pretty(item).unwrap_or_default());
                        lines.push('\n');
                    }
                }
                _ => {
                    lines.push_str(&item.to_string());
                    lines.push('\n');
                }
            }
        }
        Ok(lines.into_response())
    } else {
        Ok(serde_json::to_string_pretty(&v).unwrap_or_default().into_response())
    }
}

// Models: checkpoints convenience
pub async fn models_checkpoints(
    State(state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, String> {
    let json_flag = params.get("json").map(|v| v == "true" || v == "1").unwrap_or(false);
    let v = state.comfyui_client.get_checkpoints().await.map_err(|e| e.to_string())?;
    if json_flag {
        Ok(Json(v).into_response())
    } else if let Some(arr) = v.as_array() {
        let mut lines = String::new();
        for it in arr {
            match it {
                Value::String(s) => { lines.push_str(s); lines.push('\n'); }
                _ => { lines.push_str(&it.to_string()); lines.push('\n'); }
            }
        }
        Ok(lines.into_response())
    } else {
        Ok(serde_json::to_string_pretty(&v).unwrap_or_default().into_response())
    }
}

// Helpers (duplicated from CLI to avoid coupling)
fn collect_filenames_for_id(v: &Value, prompt_id: &str, out: &mut Vec<String>) {
    match v {
        Value::Object(map) => {
            if let Some(entry) = map.get(prompt_id) { collect_any_filenames(entry, out); }
            if let Some(hist) = map.get("history") { collect_filenames_for_id(hist, prompt_id, out); }
            for (_k, vv) in map.iter() { collect_filenames_for_id(vv, prompt_id, out); }
        }
        Value::Array(arr) => { for vv in arr { collect_filenames_for_id(vv, prompt_id, out); } }
        _ => {}
    }
}

fn collect_any_filenames(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(map) => {
            for (k, vv) in map.iter() {
                if k == "filename" { if let Value::String(s) = vv { out.push(s.clone()); } }
                collect_any_filenames(vv, out);
            }
        }
        Value::Array(arr) => { for vv in arr { collect_any_filenames(vv, out); } }
        _ => {}
    }
}

fn collect_prompt_ids(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(map) => {
            for (k, vv) in map.iter() {
                if k.len() >= 8 && vv.is_object() { out.push(k.clone()); }
                if k == "history" { collect_prompt_ids(vv, out); }
            }
        }
        Value::Array(arr) => { for vv in arr { collect_prompt_ids(vv, out); } }
        _ => {}
    }
}
