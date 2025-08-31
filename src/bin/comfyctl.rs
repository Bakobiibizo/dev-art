use clap::{Parser, Subcommand};
use comfyui_api_proxy::{Config, ComfyUIClient};
use serde_json::{json, Value};

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
            PromptCmd::Queue { workflow, file } => {
                let path = match (workflow, file) {
                    (Some(name), None) => format!("prompts/{}.json", name),
                    (None, Some(p)) => p,
                    _ => {
                        eprintln!("Must provide either --workflow <name> or --file <path>");
                        std::process::exit(2);
                    }
                };
                let data = tokio::fs::read_to_string(&path).await?;
                let raw: Value = serde_json::from_str(&data)?;
                let body = if raw.get("prompt").is_some() { raw } else { json!({"prompt": raw}) };

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
    }
}

