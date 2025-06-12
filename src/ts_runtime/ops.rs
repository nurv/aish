use deno_core::op2;
use deno_error::{JsErrorClass, AdditionalProperties};
use std::env;
use std::borrow::Cow;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use serde_json::Value;

// Custom error type for operations
#[derive(Debug, thiserror::Error)]
pub enum AishError {
    #[error("Command execution failed: {0}")]
    CommandFailed(String),
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
}

impl JsErrorClass for AishError {
    fn get_class(&self) -> Cow<'static, str> {
        match self {
            AishError::CommandFailed(_) => Cow::Borrowed("Error"),
            AishError::ToolNotFound(_) => Cow::Borrowed("Error"),
        }
    }

    fn get_message(&self) -> Cow<'static, str> {
        Cow::Owned(self.to_string())
    }

    fn get_additional_properties(&self) -> AdditionalProperties {
        Box::new(std::iter::empty())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Serialize, Deserialize)]
pub struct ShellInfo {
    pub current_dir: String,
    pub mode: String,
    pub user: String,
    pub hostname: String,
    pub home_dir: Option<String>,
}

/// Get current shell information
#[op2]
#[serde]
pub fn op_get_shell_info() -> ShellInfo {
    let current_dir = env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "/".to_string());
    
    let mode = env::var("AISH_MODE").unwrap_or_else(|_| "agent".to_string());
    let user = env::var("USER").unwrap_or_else(|_| "user".to_string());
    let hostname = env::var("HOSTNAME").unwrap_or_else(|_| "localhost".to_string());
    let home_dir = dirs::home_dir().map(|p| p.display().to_string());
    
    ShellInfo {
        current_dir,
        mode,
        user,
        hostname,
        home_dir,
    }
}

/// Get environment variable
#[op2]
#[string]
pub fn op_get_env(#[string] key: String) -> Option<String> {
    env::var(key).ok()
}

/// Set environment variable (for configuration)
#[op2(fast)]
pub fn op_set_env(#[string] key: String, #[string] value: String) {
    env::set_var(key, value);
}

/// Log message from TypeScript
#[op2(fast)]
pub fn op_log(#[string] message: String) {
    println!("[TS] {}", message);
}

/// Console.log implementation
#[op2(fast)]
pub fn op_console_log(#[string] message: String) {
    println!("{}", message);
}

/// Execute shell command from TypeScript
#[op2(async)]
#[string]
pub async fn op_execute_command(#[string] command: String) -> Result<String, AishError> {
    use std::process::Command;
    
    let output = Command::new("sh")
        .arg("-c")
        .arg(&command)
        .output()
        .map_err(|e| AishError::CommandFailed(format!("Failed to execute command: {}", e)))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    if output.status.success() {
        Ok(stdout.to_string())
    } else {
        Err(AishError::CommandFailed(format!("Command failed: {}\nSTDOUT: {}\nSTDERR: {}", 
               command, stdout, stderr)))
    }
}

// Global tool registry for storing registered tools
lazy_static::lazy_static! {
    static ref TOOL_REGISTRY: Arc<Mutex<HashMap<String, (String, Value)>>> = 
        Arc::new(Mutex::new(HashMap::new()));
}

/// Register a tool for AI agent use with JSON schema
#[op2(fast)]
pub fn op_register_agent_tool(#[string] name: String, #[string] description: String, #[string] parameters: String) -> bool {
    if let Ok(mut registry) = TOOL_REGISTRY.lock() {
        // Parse the JSON parameters string
        if let Ok(params_json) = serde_json::from_str::<Value>(&parameters) {
            registry.insert(name.clone(), (description, params_json));
            println!("[TS] Registered agent tool: {}", name);
            true
        } else {
            false
        }
    } else {
        false
    }
}

/// Get available agent tools with their schemas
#[op2]
#[string]
pub fn op_get_agent_tools() -> String {
    if let Ok(registry) = TOOL_REGISTRY.lock() {
        let tools: Vec<Value> = registry.iter().map(|(name, (description, parameters))| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": name,
                    "description": description,
                    "parameters": parameters
                }
            })
        }).collect();
        
        serde_json::to_string(&serde_json::json!(tools)).unwrap_or_else(|_| "[]".to_string())
    } else {
        "[]".to_string()
    }
}

/// Call an agent tool with parameters
#[op2(async)]
#[string]
pub async fn op_call_agent_tool(#[string] tool_name: String, #[string] parameters: String) -> Result<String, AishError> {
    // Check if tool exists in registry
    {
        let registry = TOOL_REGISTRY.lock().map_err(|_| AishError::ToolNotFound("Registry lock failed".to_string()))?;
        if !registry.contains_key(&tool_name) {
            return Err(AishError::ToolNotFound(tool_name));
        }
    }

    // For now, return a placeholder indicating the tool call would be dispatched
    // In a full implementation, this would call the actual TypeScript function
    Ok(serde_json::to_string(&serde_json::json!({
        "tool": tool_name,
        "parameters": parameters,
        "note": "Tool call would be dispatched to TypeScript runtime"
    })).unwrap_or_else(|_| "{}".to_string()))
}