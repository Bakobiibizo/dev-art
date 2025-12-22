//! Env-driven configuration for the service and library.
//!
//! Values are read from the process environment; `dotenv` is loaded on demand
//! by the binary. Defaults are provided for convenience during development.
use std::env;
use dotenv;


pub struct Config {
    pub comfyui_url: String,
    pub static_drive_path: String,
    pub prompts_dir: String,
    pub api_host: String,
    pub api_port: String,
}

impl Config {
    pub fn dotenv_load() {
        dotenv::dotenv().ok();
    }
    pub fn new() -> Result<Self, env::VarError> {
        Ok(Config {
            comfyui_url: env::var("COMFYUI_URL").unwrap_or_else(|_| "http://localhost:8188".to_string()),
            static_drive_path: env::var("STATIC_DRIVE_PATH").unwrap_or_else(|_| "./static".to_string()),
            prompts_dir: env::var("PROMPTS_DIR").unwrap_or_else(|_| "./prompts".to_string()),
            api_host: env::var("API_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            api_port: env::var("API_PORT").unwrap_or_else(|_| "8189".to_string()),
            
        })
    }
    pub fn print_env_vars() {
        println!("COMFYUI_URL: {}", env::var("COMFYUI_URL").unwrap_or_else(|_| "<unset>".to_string()));
        println!("STATIC_DRIVE_PATH: {}", env::var("STATIC_DRIVE_PATH").unwrap_or_else(|_| "<unset>".to_string()));
        println!("PROMPTS_DIR: {}", env::var("PROMPTS_DIR").unwrap_or_else(|_| "<unset>".to_string()));
        println!("API_HOST: {}", env::var("API_HOST").unwrap_or_else(|_| "<unset>".to_string()));
        println!("API_PORT: {}", env::var("API_PORT").unwrap_or_else(|_| "<unset>".to_string()));
    }
}
