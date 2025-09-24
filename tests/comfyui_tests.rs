use serde_json::json;

use comfyui_api_proxy::comfyui::client::ComfyUIClient;


#[tokio::test]
async fn test_queue_prompt() {
    let client = ComfyUIClient::new("http://art-comai.ngrok.dev".to_string());
    let test_prompt = json!({
        "3": {
            "inputs": {
                "seed": 5,
                "steps": 20,
                "cfg": 8,
                "sampler_name": "euler",
                "scheduler": "normal",
                "denoise": 1,
                "model": ["4", 0],
                "positive": ["6", 0],
                "negative": ["7", 0],
                "latent_image": ["5", 0]
            },
            "class_type": "KSampler"
        },
        "4": {
            "inputs": {
                "ckpt_name": "v1-5-pruned-emaonly.ckpt"
            },
            "class_type": "CheckpointLoaderSimple"
        },
        "5": {
            "inputs": {
                "width": 512,
                "height": 512,
                "batch_size": 1
            },
            "class_type": "EmptyLatentImage"
        },
        "6": {
            "inputs": {
                "text": "a beautiful landscape",
                "clip": ["4", 1]
            },
            "class_type": "CLIPTextEncode"
        },
        "7": {
            "inputs": {
                "text": "ugly, blurry",
                "clip": ["4", 1]
            },
            "class_type": "CLIPTextEncode"
        }
    });

    let result = client.queue_prompt(test_prompt).await;
    assert!(result.is_ok());
    // Add more specific assertions based on the expected response
}

#[tokio::test]
async fn test_get_image() {
    let client = ComfyUIClient::new("https://comfy-agentartificial.ngrok.dev".to_string());
    let result = client.get_image("test_image.png").await;
    assert!(result.is_ok());
    // Add more specific assertions based on the expected response
}

#[tokio::test]
async fn test_get_history() {
    let client = ComfyUIClient::new("https://comfy-agentartificial.ngrok.dev".to_string());
    let result = client.get_history().await;
    assert!(result.is_ok());
    // Add more specific assertions based on the expected response
}
