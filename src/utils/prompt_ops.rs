use serde_json::{json, Map, Value};

pub fn parse_set_pairs(items: &[String]) -> Result<Vec<(Vec<String>, Value)>, String> {
    let mut out = Vec::new();
    for s in items {
        let Some((k, val)) = s.split_once('=') else {
            return Err(format!("Invalid --set '{}', expected KEY=VALUE", s));
        };
        let key_path: Vec<String> = k.split('.').map(|p| p.to_string()).collect();
        let parsed_val = parse_value(val);
        out.push((key_path, parsed_val));
    }
    Ok(out)
}

pub fn parse_value(src: &str) -> Value {
    if let Ok(v) = serde_json::from_str::<Value>(src) { return v; }
    if src.eq_ignore_ascii_case("null") { return Value::Null; }
    if src.eq_ignore_ascii_case("true") { return Value::Bool(true); }
    if src.eq_ignore_ascii_case("false") { return Value::Bool(false); }
    if let Ok(i) = src.parse::<i64>() { return Value::from(i); }
    if let Ok(f) = src.parse::<f64>() { return json!(f); }
    Value::String(src.to_string())
}

pub fn apply_set_path(root: &mut Value, path: &[String], new_val: Value) -> bool {
    if path.is_empty() { return false; }
    let mut cur = root;
    for (i, key) in path.iter().enumerate() {
        let is_last = i == path.len() - 1;
        if is_last {
            if let Value::Object(map) = cur {
                map.insert(key.clone(), new_val);
                return true;
            } else {
                return false;
            }
        } else {
            match cur {
                Value::Object(map) => {
                    cur = map.entry(key.clone()).or_insert(Value::Object(Map::new()));
                }
                _ => return false,
            }
        }
    }
    false
}

pub fn ensure_filename_prefix(graph: &mut Value, default_prefix: &str) {
    if let Some(obj) = graph.as_object_mut() {
        if let Some(node8) = obj.get_mut("8") {
            if let Some(inputs) = node8.get_mut("inputs").and_then(|v| v.as_object_mut()) {
                if !inputs.contains_key("filename_prefix") {
                    inputs.insert("filename_prefix".to_string(), Value::String(default_prefix.to_string()));
                }
            }
        }
        for (_k, node) in obj.iter_mut() {
            if node.get("class_type").and_then(|v| v.as_str()) == Some("SaveImage") {
                if let Some(inputs) = node.get_mut("inputs").and_then(|v| v.as_object_mut()) {
                    if !inputs.contains_key("filename_prefix") {
                        inputs.insert("filename_prefix".to_string(), Value::String(default_prefix.to_string()));
                    }
                }
            }
        }
    }
}

// Known parameter keys we support mapping into node inputs dynamically.
const KNOWN_PARAM_KEYS: &[&str] = &[
    "seed",
    "steps",
    "cfg",
    "sampler_name",
    "scheduler",
    "denoise",
    "width",
    "height",
    "batch_size",
    "ckpt_name",
    "text",
];

/// Apply a params object to the prompt graph by matching keys to node input names.
///
/// - For each key in KNOWN_PARAM_KEYS present in `params`, finds all nodes that
///   have `inputs` containing that key and sets it to the provided value.
/// - Special case for `text`: applies to all nodes with `inputs.text` (common for
///   CLIPTextEncode). If the caller wants different values per text node, they can
///   still use explicit `sets` paths.
pub fn apply_params_map(graph: &mut Value, params: &Value) {
    let obj = match params.as_object() { Some(o) => o, None => return };

    // Extract only known keys with values
    let mut kvs: Vec<(&str, &Value)> = Vec::new();
    for &k in KNOWN_PARAM_KEYS {
        if let Some(v) = obj.get(k) {
            kvs.push((k, v));
        }
    }
    if kvs.is_empty() { return; }

    if let Some(nodes) = graph.as_object_mut() {
        for (_id, node) in nodes.iter_mut() {
            if let Some(inputs) = node.get_mut("inputs").and_then(|v| v.as_object_mut()) {
                for (k, v) in kvs.iter() {
                    if inputs.contains_key(*k) {
                        inputs.insert((*k).to_string(), (*v).clone());
                    }
                }
            }
        }
    }
}
