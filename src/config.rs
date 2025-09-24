use std::env;
use dotenv;


pub struct Config {
    pub comfyui_url: String,
    pub static_drive_path: String,
}

impl Config {
    pub fn dotenv_load() {
        dotenv::dotenv().ok();
    }
    pub fn new() -> Result<Self, env::VarError> {
        Ok(Config {
            comfyui_url: env::var("COMFYUI_URL").unwrap_or_else(|_| "https://art-comai.ngrok.dev".to_string()),
            static_drive_path: env::var("STATIC_DRIVE_PATH").unwrap_or_else(|_| "./static".to_string()),
        })
    }
    pub fn print_env_vars() {
        println!("COMFYUI_URL: {}", env::var("COMFYUI_URL").unwrap());
        println!("STATIC_DRIVE_PATH: {}", env::var("STATIC_DRIVE_PATH").unwrap());
    }
}