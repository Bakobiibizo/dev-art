//! Thin HTTP client for ComfyUI endpoints.
//!
//! - `queue_prompt` posts a prompt JSON to `/prompt`.
//! - `get_image` proxies to `/view?filename=...` and returns raw bytes.
//! - `get_history` fetches `/history` as JSON.
use reqwest::Client;
use serde_json::Value;
use crate::error::{AppResult, AppError};

#[derive(Clone)]
pub struct ComfyUIClient {
    client: Client,
    base_url: String,
}

impl ComfyUIClient {
    pub fn new(base_url: String) -> Self {
        let base = base_url.trim_end_matches('/').to_string();
        ComfyUIClient { client: Client::new(), base_url: base }
    }

    /// Queue a prompt with ComfyUI.
    ///
    /// Expects a JSON document compatible with ComfyUI's `/prompt` endpoint.
    /// Returns the JSON response from ComfyUI on success.
    pub async fn queue_prompt(&self, prompt: Value) -> AppResult<Value> {
        let url = format!("{}/prompt", self.base_url);
        tracing::info!("Sending prompt to ComfyUI at URL: {}", url);
        tracing::debug!("Prompt payload: {:?}", prompt);
    
        let response = self.client.post(&url)
            .json(&prompt)
            .send()
            .await
            .map_err(AppError::HttpClient)?;
    
        if response.status().is_success() {
            let json = response.json().await.map_err(AppError::HttpClient)?;
            tracing::info!("Successfully queued prompt. Response: {:?}", json);
            Ok(json)
        } else {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_else(|_| "Unable to read error body".to_string());
            let error_message = format!("Failed to queue prompt. Status: {}, Body: {}", status, error_body);
            tracing::error!("{}", error_message);
            Err(AppError::ComfyUI(error_message))
        }
    }

    /// Fetch image bytes by filename via ComfyUI's `/view` endpoint.
    pub async fn get_image(&self, filename: &str) -> AppResult<Vec<u8>> {
        let url = format!("{}/view", self.base_url);
        let response = self.client.get(&url)
            .query(&[("filename", filename)])
            .send()
            .await
            .map_err(AppError::HttpClient)?;

        if response.status().is_success() {
            response.bytes().await.map(|b| b.to_vec()).map_err(AppError::HttpClient)
        } else {
            Err(AppError::ComfyUI(format!("Failed to get image: {:?}", response.status())))
        }
    }

    /// Retrieve ComfyUI execution history as JSON.
    pub async fn get_history(&self) -> AppResult<Value> {
        let url = format!("{}/history", self.base_url);
        let response = self.client.get(&url)
            .send()
            .await
            .map_err(AppError::HttpClient)?;

        if response.status().is_success() {
            response.json().await.map_err(AppError::HttpClient)
        } else {
            Err(AppError::ComfyUI(format!("Failed to get history: {:?}", response.status())))
        }
    }

    // Add more methods for other ComfyUI API endpoints here
}
