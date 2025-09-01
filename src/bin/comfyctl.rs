use clap::{Parser, Subcommand};
use comfyui_api_proxy::{Config, ComfyUIClient};
use serde_json::{json, Value};
use std::path::PathBuf;
use comfyui_api_proxy::utils::prompt_ops::{apply_set_path, ensure_filename_prefix, parse_set_pairs, apply_params_map};

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
    /// Model listing utilities
    Models {
        #[command(subcommand)]
        cmd: ModelsCmd,
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
        /// Positive prompt text; auto-routed via KSampler links when possible
        #[arg(long, value_name = "TEXT")]
        text_positive: Option<String>,
        /// Negative prompt text; auto-routed via KSampler links when possible
        #[arg(long, value_name = "TEXT")]
        text_negative: Option<String>,
        /// Seed
        #[arg(long)]
        seed: Option<i64>,
        /// Steps
        #[arg(long)]
        steps: Option<i64>,
        /// CFG scale
        #[arg(long)]
        cfg: Option<f64>,
        /// Sampler name
        #[arg(long)]
        sampler_name: Option<String>,
        /// Scheduler
        #[arg(long)]
        scheduler: Option<String>,
        /// Denoise strength
        #[arg(long)]
        denoise: Option<f64>,
        /// Width
        #[arg(long)]
        width: Option<i64>,
        /// Height
        #[arg(long)]
        height: Option<i64>,
        /// Batch size
        #[arg(long, alias = "batchsize")]
        batch_size: Option<i64>,
        /// Checkpoint name
        #[arg(long)]
        ckpt_name: Option<String>,
        /// Verbose: print constructed prompt body before sending
        #[arg(short, long)]
        verbose: bool,
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

#[derive(Subcommand, Debug)]
enum ModelsCmd {
    /// Show available model categories from /models
    Categories {
        /// Output raw JSON instead of pretty lines
        #[arg(long)]
        json: bool,
    },
    /// List models in a category, e.g. checkpoints, vae, clip
    List {
        /// Category name under /models/<category>
        #[arg(long)]
        category: String,
        /// Output raw JSON instead of pretty lines
        #[arg(long)]
        json: bool,
    },
    /// Convenience: list checkpoints (values for ckpt_name)
    Checkpoints {
        /// Output raw JSON instead of pretty lines
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load env and parse CLI
    Config::dotenv_load();
    let cli = Cli::parse();

    let mut conf = Config::new().expect("Failed to load config");
    if let Some(url) = cli.comfyui_url {
        conf.comfyui_url = url;
    }

    match cli.command {
        Commands::Prompt { cmd } => match cmd {
            PromptCmd::Queue {
                workflow, file, sets, filename_prefix,
                text_positive, text_negative,
                seed, steps, cfg, sampler_name, scheduler, denoise,
                width, height, batch_size, ckpt_name,
                verbose,
            } => {
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

                // Build params map from flags
                let mut params = serde_json::Map::new();
                if let Some(t) = text_positive { params.insert("text_positive".into(), Value::String(t)); }
                if let Some(t) = text_negative { params.insert("text_negative".into(), Value::String(t)); }
                if let Some(v) = seed { params.insert("seed".into(), Value::from(v)); }
                if let Some(v) = steps { params.insert("steps".into(), Value::from(v)); }
                if let Some(v) = cfg { params.insert("cfg".into(), json!(v)); }
                if let Some(v) = sampler_name { params.insert("sampler_name".into(), Value::String(v)); }
                if let Some(v) = scheduler { params.insert("scheduler".into(), Value::String(v)); }
                if let Some(v) = denoise { params.insert("denoise".into(), json!(v)); }
                if let Some(v) = width { params.insert("width".into(), Value::from(v)); }
                if let Some(v) = height { params.insert("height".into(), Value::from(v)); }
                if let Some(v) = batch_size { params.insert("batch_size".into(), Value::from(v)); }
                if let Some(v) = ckpt_name { params.insert("ckpt_name".into(), Value::String(v)); }
                if !params.is_empty() {
                    apply_params_map(&mut graph, &Value::Object(params));
                }

                // Apply dynamic overrides
                if !sets.is_empty() {
                    let pairs = parse_set_pairs(&sets).map_err(|e| {
                        let boxed: Box<dyn std::error::Error> = e.into();
                        boxed
                    })?;
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

                // Construct request body
                let body = json!({"prompt": graph});

                if verbose {
                    eprintln!("[verbose] Request body to ComfyUI:\n{}", serde_json::to_string_pretty(&body)?);
                }

                let client = ComfyUIClient::new(conf.comfyui_url.clone());
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
            let client = ComfyUIClient::new(conf.comfyui_url.clone());
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
                let client = ComfyUIClient::new(conf.comfyui_url.clone());
                let bytes = client.get_image(&filename).await.map_err(|e| {
                    eprintln!("Error: {}", e);
                    e
                })?;
                // Default to <STATIC_DRIVE_PATH>/images/<filename>
                let default_dir = PathBuf::from(conf.static_drive_path).join("images");
                tokio::fs::create_dir_all(&default_dir).await?;
                let path = out.unwrap_or_else(|| default_dir.join(&filename));
                tokio::fs::write(&path, &bytes).await?;
                println!("Saved {} ({} bytes)", path.display(), bytes.len());
                Ok(())
            }
        },
        Commands::Models { cmd } => match cmd {
            ModelsCmd::Categories { json } => {
                let client = ComfyUIClient::new(conf.comfyui_url.clone());
                let v = client.get_model_categories().await?;
                if json {
                    println!("{}", serde_json::to_string(&v)?);
                } else {
                    if let Some(arr) = v.as_array() {
                        for item in arr {
                            if let Some(s) = item.as_str() { println!("{}", s); } else { println!("{}", item); }
                        }
                    } else {
                        println!("{}", serde_json::to_string_pretty(&v)?);
                    }
                }
                Ok(())
            }
            ModelsCmd::List { category, json } => {
                let client = ComfyUIClient::new(conf.comfyui_url.clone());
                let v = client.get_models_in_category(&category).await?;
                if json {
                    println!("{}", serde_json::to_string(&v)?);
                } else if let Some(arr) = v.as_array() {
                    for item in arr {
                        match item {
                            serde_json::Value::String(s) => println!("{}", s),
                            serde_json::Value::Object(o) => {
                                if let Some(name) = o.get("name").and_then(|x| x.as_str()) {
                                    println!("{}", name);
                                } else {
                                    println!("{}", serde_json::to_string_pretty(item).unwrap_or_default());
                                }
                            }
                            _ => println!("{}", item),
                        }
                    }
                } else {
                    println!("{}", serde_json::to_string_pretty(&v)?);
                }
                Ok(())
            }
            ModelsCmd::Checkpoints { json } => {
                let client = ComfyUIClient::new(conf.comfyui_url.clone());
                let v = client.get_checkpoints().await?;
                if json {
                    println!("{}", serde_json::to_string(&v)?);
                } else if let Some(arr) = v.as_array() {
                    for item in arr {
                        if let Some(s) = item.as_str() { println!("{}", s); } else { println!("{}", item); }
                    }
                } else {
                    println!("{}", serde_json::to_string_pretty(&v)?);
                }
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

// helper functions moved to utils::prompt_ops
