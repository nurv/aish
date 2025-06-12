pub mod isolate;
pub mod module_loader;
pub mod ops;

pub use isolate::TypeScriptIsolate;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeScriptConfig {
    pub ai: Option<TypeScriptAiConfig>,
    pub shell: Option<TypeScriptShellConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeScriptAiConfig {
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeScriptShellConfig {
    pub prompt: Option<String>,
    pub history_size: Option<usize>,
    pub multiline_continuation: Option<String>,
    pub mode_toggle_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTool {
    pub name: String,
    pub description: String,
    pub parameters: Value, // JSON Schema for parameters
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRegistry {
    pub tools: HashMap<String, AgentTool>,
}

impl Default for TypeScriptConfig {
    fn default() -> Self {
        Self {
            ai: Some(TypeScriptAiConfig {
                model: Some("gpt-3.5-turbo".to_string()),
                api_key: None,
                base_url: None,
                temperature: Some(0.7),
                max_tokens: Some(1000),
            }),
            shell: Some(TypeScriptShellConfig {
                prompt: Some("aish> ".to_string()),
                history_size: Some(1000),
                multiline_continuation: Some("... ".to_string()),
                mode_toggle_key: Some("esc-x".to_string()),
            }),
        }
    }
}

pub struct TypeScriptConfigLoader {
    script_path: PathBuf,
}

impl TypeScriptConfigLoader {
    pub fn new() -> Result<Self> {
        let config_paths = [
            dirs::home_dir().map(|mut p| { p.push(".aish.ts"); p }),
            dirs::home_dir().map(|mut p| { p.push("aish.ts"); p }),
            Some(PathBuf::from("aish.ts")),
        ];

        for path_option in &config_paths {
            if let Some(path) = path_option {
                if path.exists() {
                    println!("Found TypeScript configuration at: {}", path.display());
                    return Ok(Self {
                        script_path: path.clone(),
                    });
                }
            }
        }

        // If no TS config found, create a default one
        let default_path = dirs::home_dir()
            .map(|mut p| { p.push(".aish.ts"); p })
            .unwrap_or_else(|| PathBuf::from(".aish.ts"));

        Self::create_default_config(&default_path)?;
        
        Ok(Self {
            script_path: default_path,
        })
    }

    fn create_default_config(path: &Path) -> Result<()> {
        let default_config = r#"// aish JavaScript Configuration
// This file is executed by aish to load configuration and custom functions
// Simple JavaScript for compatibility with the basic runtime

// Default configuration - export this as the main config
const config = {
  ai: {
    model: "gpt-4",
    temperature: 0.7,
    max_tokens: 1000,
    // api_key: "your-api-key-here", // Uncomment and set your API key
  },
  shell: {
    prompt: "aish> ",
    history_size: 1000,
    multiline_continuation: "... ",
  }
};

// Example custom prompt function
function customPrompt() {
  try {
    const shellInfo = Deno.core.ops.op_get_shell_info();
    const time = new Date().toLocaleTimeString();
    return `[${time}] ${shellInfo.user}@${shellInfo.hostname}:${shellInfo.current_dir} [${shellInfo.mode}]$ `;
  } catch (error) {
    return "aish> ";
  }
}

// Example utility function for future AI agent integration
function getProjectInfo() {
  return {
    type: "rust",
    name: "aish",
    version: "0.1.0"
  };
}

// Define agent tool functions
function listFiles(params) {
  const targetPath = params.path || Deno.core.ops.op_get_shell_info().current_dir;
  const pattern = params.pattern || "*";
  
  try {
    const result = Deno.core.ops.op_execute_command(`find ${targetPath} -name "${pattern}" -type f | head -20`);
    return {
      success: true,
      files: result.split('\n').filter(f => f.trim().length > 0),
      path: targetPath,
      pattern: pattern
    };
  } catch (error) {
    return {
      success: false,
      error: error.message,
      path: targetPath,
      pattern: pattern
    };
  }
}

function readFile(params) {
  try {
    const command = params.lines 
      ? `head -n ${params.lines} "${params.path}"`
      : `cat "${params.path}"`;
    const content = Deno.core.ops.op_execute_command(command);
    return {
      success: true,
      content: content,
      path: params.path,
      lines: params.lines
    };
  } catch (error) {
    return {
      success: false,
      error: error.message,
      path: params.path
    };
  }
}

function gitStatus(params) {
  try {
    const status = Deno.core.ops.op_execute_command("git status --porcelain");
    const branch = Deno.core.ops.op_execute_command("git branch --show-current").trim();
    return {
      success: true,
      status: status,
      branch: branch,
      files: status.split('\n').filter(f => f.trim().length > 0)
    };
  } catch (error) {
    return {
      success: false,
      error: error.message
    };
  }
}

// Agent tools schema
const agentTools = {
  tools: {
    "list_files": {
      name: "list_files",
      description: "List files in a directory with optional pattern matching",
      parameters: {
        type: "object",
        properties: {
          path: {
            type: "string",
            description: "Directory path to list files from (defaults to current directory)"
          },
          pattern: {
            type: "string",
            description: "Glob pattern to match files (defaults to '*')"
          }
        },
        required: []
      }
    },
    "read_file": {
      name: "read_file",
      description: "Read the contents of a file",
      parameters: {
        type: "object",
        properties: {
          path: {
            type: "string",
            description: "Path to the file to read"
          },
          lines: {
            type: "number",
            description: "Number of lines to read from the beginning (optional, reads entire file if not specified)"
          }
        },
        required: ["path"]
      }
    },
    "git_status": {
      name: "git_status",
      description: "Get git repository status and current branch information",
      parameters: {
        type: "object",
        properties: {},
        required: []
      }
    }
  }
};

// Export functions to global scope for Rust access
globalThis.customPrompt = customPrompt;
globalThis.getProjectInfo = getProjectInfo;
globalThis.config = config;
globalThis.agentTools = agentTools;

// Export tool functions
globalThis.list_files = listFiles;
globalThis.read_file = readFile;
globalThis.git_status = gitStatus;
"#;

        std::fs::write(path, default_config)?;
        println!("Created default TypeScript configuration at: {}", path.display());
        Ok(())
    }

    pub async fn load_config(&self) -> Result<TypeScriptConfig> {
        let mut isolate = TypeScriptIsolate::new(&self.script_path).await?;
        isolate.execute(&self.script_path).await?;

        // Try to get the config from global scope
        match isolate.get_export("config").await {
            Ok(config_value) => {
                let config: TypeScriptConfig = serde_json::from_value(config_value)?;
                Ok(config)
            }
            Err(_) => {
                println!("No config found in TypeScript config, using defaults");
                Ok(TypeScriptConfig::default())
            }
        }
    }

    pub async fn call_prompt_function(&self, function_name: &str) -> Result<Option<String>> {
        let mut isolate = TypeScriptIsolate::new(&self.script_path).await?;
        isolate.execute(&self.script_path).await?;

        match isolate.call_function(function_name, &[]).await {
            Ok(result) => {
                if let Value::String(prompt) = result {
                    Ok(Some(prompt))
                } else {
                    Ok(None)
                }
            }
            Err(_) => Ok(None),
        }
    }

    pub async fn load_agent_tools(&self) -> Result<ToolRegistry> {
        let mut isolate = TypeScriptIsolate::new(&self.script_path).await?;
        isolate.execute(&self.script_path).await?;

        // Try to get the tools registry from global scope
        match isolate.get_export("agentTools").await {
            Ok(tools_value) => {
                let tools: ToolRegistry = serde_json::from_value(tools_value)?;
                Ok(tools)
            }
            Err(_) => {
                // Return empty registry if no tools defined
                Ok(ToolRegistry {
                    tools: HashMap::new(),
                })
            }
        }
    }

    pub async fn call_agent_tool(&self, tool_name: &str, parameters: &Value) -> Result<Value> {
        let mut isolate = TypeScriptIsolate::new(&self.script_path).await?;
        isolate.execute(&self.script_path).await?;

        // Call the tool function with parameters
        let args = vec![parameters.clone()];
        isolate.call_function(tool_name, &args).await
    }
}