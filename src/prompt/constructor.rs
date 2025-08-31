//! Prompt templating with simple `{{placeholder}}` substitution.
//!
//! Given a JSON `template` and an `inputs` object, recursively walks the
//! template and replaces any string values of the form `{{ key }}` with
//! `inputs[key]`, returning a constructed JSON value.
use serde_json::Value;
use crate::error::{AppResult, AppError};

pub struct PromptConstructor;

impl PromptConstructor {
    pub fn new() -> Self {
        PromptConstructor
    }

    /// Construct a prompt by substituting placeholders inside `template`
    /// with corresponding values from `inputs`.
    pub fn construct_prompt(&self, template: &Value, inputs: &Value) -> AppResult<Value> {
        self.validate_template(template)?;
        self.validate_inputs(inputs)?;

        let mut constructed = template.clone();
        self.replace_placeholders(&mut constructed, inputs)?;

        Ok(constructed)
    }

    /// TODO: Placeholder for template validation (shape, required fields, etc.).
    fn validate_template(&self, template: &Value) -> AppResult<()> {
        // Add template validation logic here
        Ok(())
    }

    /// TODO: Placeholder for input validation.
    fn validate_inputs(&self, inputs: &Value) -> AppResult<()> {
        // Add input validation logic here
        Ok(())
    }

    /// Recursively replace `{{key}}` strings with `inputs[key]`.
    fn replace_placeholders(&self, value: &mut Value, inputs: &Value) -> AppResult<()> {
        match value {
            Value::Object(map) => {
                for (_, v) in map.iter_mut() {
                    self.replace_placeholders(v, inputs)?;
                }
            }
            Value::Array(arr) => {
                for v in arr.iter_mut() {
                    self.replace_placeholders(v, inputs)?;
                }
            }
            Value::String(s) => {
                if s.starts_with("{{") && s.ends_with("}}") {
                    let key = s.trim_start_matches("{{").trim_end_matches("}}").trim();
                    if let Some(replacement) = inputs.get(key) {
                        *value = replacement.clone();
                    } else {
                        return Err(AppError::PromptConstruction(format!("Missing input for placeholder: {}", key)));
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}
