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
                    if let Some(next) = map.get_mut(key) {
                        cur = next;
                    } else {
                        return false;
                    }
                }
                _ => return false,
            }
        }
    }
    false
}

pub fn ensure_filename_prefix(graph: &mut Value, default_prefix: &str) {
    if let Some(obj) = graph.as_object_mut() {
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

    // Handle specialized text mapping first (positive/negative)
    let text_pos = obj.get("text_positive").cloned();
    let text_neg = obj.get("text_negative").cloned();
    if text_pos.is_some() || text_neg.is_some() {
        apply_text_pos_neg(graph, text_pos.as_ref(), text_neg.as_ref());
    }

    // Extract only known keys with values (excluding specialized keys above)
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

fn apply_text_pos_neg(graph: &mut Value, text_pos: Option<&Value>, text_neg: Option<&Value>) {
    // Strategy:
    // 1) Prefer to locate a KSampler node and follow its `positive`/`negative` inputs to CLIPTextEncode nodes.
    // 2) If not resolvable, apply to all CLIPTextEncode nodes, in order: first gets positive, second gets negative (if provided).
    // 3) If only one text provided, apply that one where possible and leave others untouched.

    let ksampler_id = find_first_node_id_by_class(graph, "KSampler");

    let mut applied_pos = false;
    let mut applied_neg = false;

    if let Some(ref ks_id) = ksampler_id {
        if let Some(v) = text_pos {
            if let Some(src) = source_node_id_from_ksampler_input(graph, ks_id, "positive") {
                applied_pos = set_node_text(graph, &src, v);
            }
        }
        if let Some(v) = text_neg {
            if let Some(src) = source_node_id_from_ksampler_input(graph, ks_id, "negative") {
                applied_neg = set_node_text(graph, &src, v);
            }
        }
    }

    // Fallback to applying on CLIPTextEncode nodes in encounter order
    if (!applied_pos && text_pos.is_some()) || (!applied_neg && text_neg.is_some()) {
        let mut clip_nodes = collect_clip_textencode_ids(graph);

        if let Some(v) = text_pos {
            if !applied_pos {
                if let Some(first) = clip_nodes.get(0) {
                    let _ = set_node_text(graph, first, v);
                }
            }
        }
        if let Some(v) = text_neg {
            if !applied_neg {
                if let Some(second) = clip_nodes.get(1) {
                    let _ = set_node_text(graph, second, v);
                }
            }
        }
    }
}

fn find_first_node_id_by_class(graph: &Value, class_type: &str) -> Option<String> {
    graph.as_object()?.iter().find_map(|(id, node)| {
        node.get("class_type")
            .and_then(|ct| ct.as_str())
            .filter(|ct| *ct == class_type)
            .map(|_| id.clone())
    })
}

fn source_node_id_from_ksampler_input(graph: &Value, ksampler_id: &str, input_name: &str) -> Option<String> {
    let node = graph.get(ksampler_id)?;
    let inputs = node.get("inputs")?.as_object()?;
    let source = inputs.get(input_name)?;
    if let Some(arr) = source.as_array() {
        if let Some(idv) = arr.get(0) {
            if let Some(s) = idv.as_str() { return Some(s.to_string()); }
            if let Some(n) = idv.as_i64() { return Some(n.to_string()); }
        }
    }
    None
}

fn set_node_text(graph: &mut Value, node_id: &str, v: &Value) -> bool {
    if let Some(node) = graph.get_mut(node_id) {
        if let Some(inputs) = node.get_mut("inputs").and_then(|i| i.as_object_mut()) {
            inputs.insert("text".to_string(), v.clone());
            return true;
        }
    }
    false
}

fn collect_clip_textencode_ids(graph: &Value) -> Vec<String> {
    let mut ids: Vec<String> = graph.as_object()
        .into_iter()
        .flat_map(|o| o.iter())
        .filter_map(|(id, node)| {
            node.get("class_type")
                .and_then(|ct| ct.as_str())
                .filter(|ct| *ct == "CLIPTextEncode")
                .map(|_| id.clone())
        })
        .collect();
    ids.sort();
    ids
}
