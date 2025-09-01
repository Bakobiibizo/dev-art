use comfyui_api_proxy::{Config, ComfyUIClient};
use serde_json::{Value, json};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load env (if .env present)
    Config::dotenv_load();
    let cfg = Config::new().expect("Failed to load config");

    // Read prompts/sdxl.json as raw JSON
    let data = tokio::fs::read_to_string("prompts/sdxl.json").await?;
    let prompt_raw: Value = serde_json::from_str(&data)?;

    // Send to ComfyUI
    let client = ComfyUIClient::new(cfg.comfyui_url.clone());
    println!("Queueing prompt to {}", cfg.comfyui_url);
    let body = if prompt_raw.get("prompt").is_some() {
        prompt_raw
    } else {
        json!({"prompt": prompt_raw})
    };
    let res = client.queue_prompt(body).await?;
    println!("Response: {}", serde_json::to_string_pretty(&res)?);
    Ok(())
}
