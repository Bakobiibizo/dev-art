use clap::{Parser, Subcommand};
use comfyui_api_proxy::{Config, ComfyUIClient};
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "comfyctl", about = "CLI for ComfyUI API Proxy", version)]
struct Cli {
    /// Override COMFYUI_URL
    #[arg(global = true, long)]
    comfyui_url: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Prompt-related commands
    Prompt {
        #[command(subcommand)]
        cmd: PromptCmd,
    },
    /// Fetch ComfyUI execution history
    History {
        /// Filter by prompt ID to list output filenames
        #[arg(long)]
        prompt_id: Option<String>,
        /// Pretty-print full JSON history
        #[arg(long)]
        pretty: bool,
    },
    /// Image operations
    Image {
        #[command(subcommand)]
        cmd: ImageCmd,
    },
}

#[derive(Subcommand, Debug)]
enum PromptCmd {
    /// Queue a workflow prompt to ComfyUI
    Queue {
        /// Workflow name under prompts/<name>.json
        #[arg(long, conflicts_with = "file")]
        workflow: Option<String>,
        /// Explicit file path to a workflow JSON
        #[arg(long, value_name = "PATH")]
        file: Option<String>,
        /// Dynamic overrides as key=value (repeatable). Key is a path like
        /// `2.inputs.seed`, `4.inputs.ckpt_name`, or `prompt.2.inputs.seed`.
        #[arg(long = "set", value_name = "KEY=VALUE")]
        sets: Vec<String>,
        /// Default filename prefix to apply if present and not overridden
        #[arg(long, default_value = "Derivata")]
        filename_prefix: String,
    },
}

#[derive(Subcommand, Debug)]
enum ImageCmd {
    /// Download an image by filename
    Get {
        /// Filename reported by ComfyUI (e.g. in history)
        filename: String,
        /// Output path (defaults to ./<filename>)
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load env and parse CLI
    Config::dotenv_load();
    let cli = Cli::parse();

    let mut cfg = Config::new().expect("Failed to load config");
    if let Some(url) = cli.comfyui_url {
        cfg.comfyui_url = url;
    }

    match cli.command {
        Commands::Prompt { cmd } => match cmd {
            PromptCmd::Queue { workflow, file, sets, filename_prefix } => {
                let path = match (workflow, file) {
                    (Some(name), None) => format!("prompts/{}.json", name),
                    (None, Some(p)) => p,
                    _ => {
                        eprintln!("Must provide either --workflow <name> or --file <path>");
                        std::process::exit(2);
                    }
                };
                let data = tokio::fs::read_to_string(&path).await?;
                let mut raw: Value = serde_json::from_str(&data)?;

                // Extract graph whether already wrapped or not
                let mut graph = if let Some(p) = raw.get("prompt").cloned() { p } else { raw.clone() };

                // Apply dynamic overrides
                if !sets.is_empty() {
                    let pairs = parse_set_pairs(&sets)?;
                    for (path, new_val) in pairs {
                        if !apply_set_path(&mut graph, &path, new_val.clone()) {
                            // If graph was originally wrapped, user may have provided a full path starting with `prompt.`
                            if !apply_set_path(&mut raw, &path, new_val.clone()) {
                                eprintln!("Warning: could not apply --set to path: {}", path.join("."));
                            }
                        }
                    }
                }

                // If not overridden, set filename_prefix defaults
                ensure_filename_prefix(&mut graph, &filename_prefix);

                // Rewrap if needed
                let body = if raw.get("prompt").is_some() { json!({"prompt": graph}) } else { json!({"prompt": graph}) };

                let client = ComfyUIClient::new(cfg.comfyui_url.clone());
                let res = client.queue_prompt(body).await;
                match res {
                    Ok(v) => {
                        println!("{}", serde_json::to_string_pretty(&v)?);
                        Ok(())
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        },
        Commands::History { prompt_id, pretty } => {
            let client = ComfyUIClient::new(cfg.comfyui_url.clone());
            let hist = client.get_history().await.map_err(|e| {
                eprintln!("Error: {}", e);
                e
            })?;

            if let Some(id) = prompt_id {
                let mut files: Vec<String> = Vec::new();
                collect_filenames_for_id(&hist, &id, &mut files);
                if files.is_empty() {
                    eprintln!("No filenames found for prompt_id={}", id);
                } else {
                    for f in files { println!("{}", f); }
                }
                Ok(())
            } else if pretty {
                println!("{}", serde_json::to_string_pretty(&hist)?);
                Ok(())
            } else {
                println!("{}", serde_json::to_string(&hist)?);
                Ok(())
            }
        }
        Commands::Image { cmd } => match cmd {
            ImageCmd::Get { filename, out } => {
                let client = ComfyUIClient::new(cfg.comfyui_url.clone());
                let bytes = client.get_image(&filename).await.map_err(|e| {
                    eprintln!("Error: {}", e);
                    e
                })?;
                // Default to <STATIC_DRIVE_PATH>/images/<filename>
                let default_dir = PathBuf::from(cfg.static_drive_path).join("images");
                tokio::fs::create_dir_all(&default_dir).await?;
                let path = out.unwrap_or_else(|| default_dir.join(&filename));
                tokio::fs::write(&path, &bytes).await?;
                println!("Saved {} ({} bytes)", path.display(), bytes.len());
                Ok(())
            }
        },
    }
}

fn collect_filenames_for_id(v: &Value, prompt_id: &str, out: &mut Vec<String>) {
    // Expected shapes vary by ComfyUI version. Try common cases.
    match v {
        Value::Object(map) => {
            // Direct match on key == prompt_id
            if let Some(entry) = map.get(prompt_id) {
                collect_any_filenames(entry, out);
            }
            // Some servers wrap under "history"
            if let Some(hist) = map.get("history") {
                collect_filenames_for_id(hist, prompt_id, out);
            }
            // Recurse into all values
            for (_k, vv) in map.iter() {
                collect_filenames_for_id(vv, prompt_id, out);
            }
        }
        Value::Array(arr) => {
            for vv in arr { collect_filenames_for_id(vv, prompt_id, out); }
        }
        _ => {}
    }
}

fn collect_any_filenames(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(map) => {
            for (k, vv) in map.iter() {
                if k == "filename" {
                    if let Value::String(s) = vv { out.push(s.clone()); }
                }
                collect_any_filenames(vv, out);
            }
        }
        Value::Array(arr) => {
            for vv in arr { collect_any_filenames(vv, out); }
        }
        _ => {}
    }
}

fn parse_set_pairs(items: &[String]) -> Result<Vec<(Vec<String>, Value)>, Box<dyn std::error::Error>> {
    let mut out = Vec::new();
    for s in items {
        let Some((k, val)) = s.split_once('=') else {
            return Err(format!("Invalid --set '{}', expected KEY=VALUE", s).into());
        };
        let key_path: Vec<String> = k.split('.').map(|p| p.to_string()).collect();
        let parsed_val = parse_value(val);
        out.push((key_path, parsed_val));
    }
    Ok(out)
}

fn parse_value(src: &str) -> Value {
    // Try JSON first (so strings can be quoted, objects/arrays supported)
    if let Ok(v) = serde_json::from_str::<Value>(src) { return v; }
    // Fall back to primitive inference
    if src.eq_ignore_ascii_case("null") { return Value::Null; }
    if src.eq_ignore_ascii_case("true") { return Value::Bool(true); }
    if src.eq_ignore_ascii_case("false") { return Value::Bool(false); }
    if let Ok(i) = src.parse::<i64>() { return Value::from(i); }
    if let Ok(f) = src.parse::<f64>() { return json!(f); }
    Value::String(src.to_string())
}

fn apply_set_path(root: &mut Value, path: &[String], new_val: Value) -> bool {
    if path.is_empty() { return false; }
    // Navigate creating objects as needed; arrays are not auto-created
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
                    cur = map.entry(key.clone()).or_insert(Value::Object(serde_json::Map::new()));
                }
                _ => return false,
            }
        }
    }
    false
}

fn ensure_filename_prefix(graph: &mut Value, default_prefix: &str) {
    // If node 8 exists and has inputs.filename_prefix, set default (unless already set)
    if let Some(obj) = graph.as_object_mut() {
        if let Some(node8) = obj.get_mut("8") {
            if let Some(inputs) = node8.get_mut("inputs").and_then(|v| v.as_object_mut()) {
                if !inputs.contains_key("filename_prefix") {
                    inputs.insert("filename_prefix".to_string(), Value::String(default_prefix.to_string()));
                }
            }
        }
        // Also set on any SaveImage nodes if not set
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
