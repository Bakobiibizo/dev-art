use serde_json::{json, Value};
use tokio::fs;

use crate::utils::prompt_ops::{apply_params_map, apply_set_path, ensure_filename_prefix, parse_set_pairs};

pub async fn resolve_prompt_root_from_payload(payload: &Value, prompts_dir: &str) -> Result<Value, String> {
    if let Some(prompt) = payload.get("prompt").cloned() {
        return Ok(json!({"prompt": prompt}));
    }
    let workflow_name = payload.get("workflow")
        .and_then(|v| v.as_str())
        .ok_or("Either 'prompt' or 'workflow' must be provided")?;
    let workflow_path = format!("{}/{}.json", prompts_dir.trim_end_matches('/'), workflow_name);
    let workflow_content = fs::read_to_string(&workflow_path)
        .await
        .map_err(|e| format!("Failed to read workflow file: {}", e))?;
    let wf: Value = serde_json::from_str(&workflow_content)
        .map_err(|e| format!("Failed to parse workflow JSON: {}", e))?;
    Ok(if wf.get("prompt").is_some() { wf } else { json!({"prompt": wf}) })
}

pub fn apply_overrides_from_payload(root: &mut Value, payload: &Value) -> Result<(), String> {
    // Merge params from `params` and convenient top-level keys
    let mut params_obj = serde_json::Map::new();
    if let Some(params) = payload.get("params").and_then(|v| v.as_object()) {
        for (k, v) in params.iter() { params_obj.insert(k.clone(), v.clone()); }
    }
    let top_keys = [
        "seed","steps","cfg","sampler_name","scheduler","denoise",
        "width","height","batch_size","ckpt_name","text","text_positive","text_negative"
    ];
    for k in top_keys.iter() {
        if let Some(v) = payload.get(*k) { params_obj.insert((*k).to_string(), v.clone()); }
    }
    if !params_obj.is_empty() {
        if let Some(graph) = root.get_mut("prompt") { apply_params_map(graph, &Value::Object(params_obj)); }
    }

    if let Some(sets) = payload.get("sets").and_then(|v| v.as_array()) {
        let items: Vec<String> = sets.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
        if !items.is_empty() {
            let pairs = parse_set_pairs(&items).map_err(|e| e.to_string())?;
            for (path, new_val) in pairs {
                let applied_to_graph = {
                    let graph = root.get_mut("prompt").ok_or("Missing 'prompt' in body")?;
                    apply_set_path(graph, &path, new_val.clone())
                };
                if !applied_to_graph { let _ = apply_set_path(root, &path, new_val); }
            }
        }
    }
    Ok(())
}

pub fn ensure_defaults_on_root(root: &mut Value, filename_prefix: Option<&str>) {
    if let Some(graph) = root.get_mut("prompt") {
        let default_prefix = filename_prefix.unwrap_or("Derivata");
        ensure_filename_prefix(graph, default_prefix);
    }
}

pub fn maybe_log_verbose(root: &Value, verbose: bool) {
    if verbose {
        if let Ok(s) = serde_json::to_string(root) {
            tracing::info!(target = "queue_prompt", body = %s, "Constructed request body");
        }
    }
}

pub fn is_probably_graph(graph: &Value) -> bool {
    if let Some(obj) = graph.as_object() {
        for (_k, v) in obj.iter() {
            if let Some(node) = v.as_object() {
                if node.get("class_type").and_then(|ct| ct.as_str()).is_some() { return true; }
            }
        }
    }
    false
}

