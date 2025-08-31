//! Minimal in-memory and file-backed workflow manager.
//!
//! Responsibilities:
//! - Load/save workflows from `prompts/<name>.json`.
//! - Keep track of the last selected workflow.
//! - Store arbitrary node metadata (if provided programmatically).
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone)]
pub struct WorkflowManager {
    workflow: Value,
    workflows: HashMap<String, Value>,
    nodes: HashMap<String, Value>,
}

impl WorkflowManager {
    pub fn new() -> Self {
        WorkflowManager {
            workflow: Value::Null,
            workflows: HashMap::new(),
            nodes: HashMap::new(),
        }
    }

    pub async fn add_workflow(&mut self, name: Option<String>, workflow: Option<Value>) -> Result<(), String> {
        match (name, workflow) {
            (Some(name), Some(workflow)) => {
                // Save the provided workflow under the given name
                let file_path = format!("prompts/{}.json", name);
                let workflow_content = serde_json::to_string_pretty(&workflow)
                    .map_err(|e| format!("Failed to serialize workflow: {}", e))?;
                
                tokio::fs::write(&file_path, workflow_content)
                    .await
                    .map_err(|e| format!("Failed to write workflow to {}: {}", file_path, e))?;
                
                self.workflows.insert(name, workflow.clone());
                self.workflow = workflow;
                Ok(())
            },
            (Some(name), None) => {
                // Load the workflow with the given name
                let loaded_workflow = self.load_workflow(&name).await?;
                self.workflows.insert(name, loaded_workflow.clone());
                self.workflow = loaded_workflow;
                Ok(())
            },
            (None, Some(_)) => Err("A name must be provided when adding a new workflow".to_string()),
            (None, None) => Err("Either a name or a workflow must be provided".to_string()),
        }
    }

    pub async fn add_node(&mut self, node_type: String, node_info: Value) {
        self.nodes.insert(node_type, node_info);
    }

    pub fn get_node_info(&self, node_type: &str) -> Option<Value> {
        self.nodes.get(node_type).cloned()
    }
    pub async fn load_workflow(&self, name: &str) -> Result<Value, String> {
        let file_path = format!("prompts/{}.json", name);
        let workflow_content = tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| format!("Failed to read file {}: {}", file_path, e))?;

        let workflow: Value = serde_json::from_str(&workflow_content)
            .map_err(|e| format!("Failed to parse JSON from {}: {}", file_path, e))?;

        tokio::fs::write("workflow.json", &workflow_content)
            .await
            .map_err(|e| format!("Failed to write to workflow.json: {}", e))?;

        Ok(workflow)
    }
}
