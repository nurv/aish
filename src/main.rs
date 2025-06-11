use anyhow::Result;
use clap::Parser;
use reqwest::Client;
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, KeyEvent, EventHandler, ConditionalEventHandler, Event, RepeatCount, EventContext, Cmd};
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};

mod ts_runtime;

#[derive(Debug, Clone, PartialEq)]
pub enum ShellMode {
    Agent,
    Command,
}

impl ShellMode {
    fn as_str(&self) -> &'static str {
        match self {
            ShellMode::Agent => "agent",
            ShellMode::Command => "command",
        }
    }
    
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "command" => ShellMode::Command,
            _ => ShellMode::Agent, // default to agent
        }
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    command: Option<String>,
}

// Config is now handled by TypeScript runtime
pub type Config = ts_runtime::TypeScriptConfig;
pub type AiConfig = ts_runtime::TypeScriptAiConfig;
pub type ShellConfig = ts_runtime::TypeScriptShellConfig;

impl Config {
    pub async fn load() -> Result<Self> {
        let loader = ts_runtime::TypeScriptConfigLoader::new()?;
        loader.load_config().await
    }

    pub fn get_prompt(&self, current_dir: &PathBuf, mode: &ShellMode) -> String {
        let prompt_template = self.shell
            .as_ref()
            .and_then(|s| s.prompt.as_ref())
            .cloned()
            .unwrap_or_else(|| "aish> ".to_string());
        
        self.expand_prompt(&prompt_template, current_dir, mode)
    }

    pub fn get_continuation_prompt(&self, current_dir: &PathBuf, mode: &ShellMode) -> String {
        let prompt_template = self.shell
            .as_ref()
            .and_then(|s| s.multiline_continuation.as_ref())
            .cloned()
            .unwrap_or_else(|| "... ".to_string());
        
        self.expand_prompt(&prompt_template, current_dir, mode)
    }

    fn expand_prompt(&self, template: &str, current_dir: &PathBuf, mode: &ShellMode) -> String {
        let mut result = template.to_string();
        
        // Expand environment variables using $VAR or ${VAR} syntax
        while let Some(start) = result.find('$') {
            if start + 1 >= result.len() {
                break;
            }
            
            let remaining = &result[start + 1..];
            let (var_name, end_pos) = if remaining.starts_with('{') {
                // ${VAR} syntax
                if let Some(close) = remaining.find('}') {
                    (&remaining[1..close], close + 2)
                } else {
                    break;
                }
            } else {
                // $VAR syntax - find end of variable name
                let end = remaining.find(|c: char| !c.is_alphanumeric() && c != '_')
                    .unwrap_or(remaining.len());
                if end == 0 {
                    // Just a $ with no variable name
                    break;
                }
                (&remaining[..end], end + 1)
            };
            
            let env_value = env::var(var_name).unwrap_or_default();
            result.replace_range(start..start + end_pos, &env_value);
        }
        
        // PS1-style escape sequences
        result = result.replace("\\u", &env::var("USER").unwrap_or_else(|_| "user".to_string()));
        result = result.replace("\\h", &gethostname());
        result = result.replace("\\H", &gethostname());
        
        // Working directory expansions
        let home_dir = dirs::home_dir();
        let current_dir_str = current_dir.display().to_string();
        
        if let Some(home) = &home_dir {
            let home_str = home.display().to_string();
            if current_dir_str.starts_with(&home_str) {
                let relative = current_dir_str.strip_prefix(&home_str)
                    .unwrap_or(&current_dir_str);
                let tilde_path = if relative.is_empty() {
                    "~".to_string()
                } else {
                    format!("~{}", relative)
                };
                result = result.replace("\\w", &tilde_path);
                result = result.replace("\\W", 
                    &current_dir.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("~")
                );
            } else {
                result = result.replace("\\w", &current_dir_str);
                result = result.replace("\\W", 
                    &current_dir.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("/")
                );
            }
        } else {
            result = result.replace("\\w", &current_dir_str);
            result = result.replace("\\W", 
                &current_dir.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("/")
            );
        }
        
        // Mode-specific escape sequences
        result = result.replace("\\m", mode.as_str());
        result = result.replace("\\M", &mode.as_str().to_uppercase());
        
        // Other common escape sequences
        result = result.replace("\\$", if env::var("USER").unwrap_or_default() == "root" { "#" } else { "$" });
        result = result.replace("\\n", "\n");
        result = result.replace("\\t", "\t");
        result = result.replace("\\[", "\x1b["); // ANSI escape start
        result = result.replace("\\]", ""); // ANSI escape end (invisible)
        
        result
    }
}

fn gethostname() -> String {
    // Try to get hostname from environment first
    if let Ok(hostname) = env::var("HOSTNAME") {
        return hostname;
    }
    
    // Fallback to calling hostname command
    if let Ok(output) = Command::new("hostname").output() {
        if output.status.success() {
            return String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
    }
    
    // Final fallback
    "localhost".to_string()
}

// Custom event handler for mode toggle (ESC-x)
#[derive(Clone)]
struct ModeToggleHandler {
    toggle_flag: Arc<Mutex<bool>>,
}

impl ModeToggleHandler {
    fn new() -> Self {
        Self {
            toggle_flag: Arc::new(Mutex::new(false)),
        }
    }
    
    fn check_toggle(&self) -> bool {
        if let Ok(mut flag) = self.toggle_flag.lock() {
            if *flag {
                *flag = false;
                return true;
            }
        }
        false
    }
    
    fn set_toggle(&self) {
        if let Ok(mut flag) = self.toggle_flag.lock() {
            *flag = true;
        }
    }
}

impl ConditionalEventHandler for ModeToggleHandler {
    fn handle(&self, evt: &Event, _: RepeatCount, _: bool, _ctx: &EventContext) -> Option<Cmd> {
        if let Some(k) = evt.get(0) {
            // ESC followed by 'x' is typically represented as Alt+x in many terminals
            if *k == KeyEvent::alt('x') {
                self.set_toggle();
                // Return Interrupt to break out of readline loop
                return Some(Cmd::Interrupt);
            }
        }
        None // default behavior
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolCall {
    id: String,
    r#type: String,
    function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: Option<String>,
    tool_calls: Option<Vec<ToolCall>>,
    tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Choice {
    message: ChatMessage,
    finish_reason: Option<String>,
}

struct AiAgent {
    client: Client,
    config: Config,
}

impl AiAgent {
    fn new(config: Config) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    async fn process_prompt(&self, prompt: &str, current_dir: &PathBuf, ts_config_loader: &ts_runtime::TypeScriptConfigLoader) -> Result<()> {
        let api_key = self.config.ai.as_ref()
            .and_then(|ai| ai.api_key.as_ref())
            .ok_or_else(|| anyhow::anyhow!(
                "OpenAI API key not found. Please set it in ~/.aish.ts:\n\n\
                ai: {{ api_key: \"your-api-key-here\" }}"
            ))?;

        let model = self.config.ai.as_ref()
            .and_then(|ai| ai.model.as_ref())
            .cloned()
            .unwrap_or_else(|| "gpt-4".to_string());

        let base_url = self.config.ai.as_ref()
            .and_then(|ai| ai.base_url.as_ref())
            .cloned()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

        let temperature = self.config.ai.as_ref()
            .and_then(|ai| ai.temperature)
            .unwrap_or(0.7);

        let max_tokens = self.config.ai.as_ref()
            .and_then(|ai| ai.max_tokens)
            .unwrap_or(1000);

        // Load available tools from TypeScript configuration
        let tool_registry = ts_config_loader.load_agent_tools().await?;

        let mut messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: Some(
                    "You are an AI assistant integrated into a Unix shell called 'aish'. \
                    Your role is to help users accomplish tasks by analyzing their requests and \
                    executing appropriate commands when needed.\n\n\
                    You have access to a 'run_command' tool that can execute shell commands. \
                    Use this tool when the user's request requires running commands.\n\n\
                    When you use run_command, always prefix your explanation with:\n\
                    '**** Running command'\n\
                    Then show the command being executed with a '$ ' prefix.\n\n\
                    After executing commands and getting the results, provide a helpful \
                    response to the user. If the command output answers their question, \
                    you can simply acknowledge the result. If additional explanation is needed, \
                    provide it.\n\n\
                    Always be concise and helpful.".to_string()
                ),
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: Some(prompt.to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        loop {
            let response = self.make_openai_request(&messages, &model, temperature, max_tokens, &base_url, api_key, &tool_registry).await?;
            
            if let Some(choice) = response.choices.first() {
                let message = &choice.message;
                messages.push(message.clone());

                // Check if the assistant wants to use tools
                if let Some(tool_calls) = &message.tool_calls {
                    for tool_call in tool_calls {
                        let function_name = &tool_call.function.name;
                        let args: Value = serde_json::from_str(&tool_call.function.arguments)?;
                        
                        let output = if function_name == "run_command" {
                            // Legacy built-in command execution
                            let command = args["command"].as_str()
                                .ok_or_else(|| anyhow::anyhow!("Invalid command argument"))?;

                            println!("**** Running command");
                            println!("   $ {}", command);
                            
                            self.execute_command(command, current_dir)?
                        } else if tool_registry.tools.contains_key(function_name) {
                            // TypeScript-defined tool
                            println!("**** Calling tool: {}", function_name);
                            match ts_config_loader.call_agent_tool(function_name, &args).await {
                                Ok(result) => {
                                    serde_json::to_string_pretty(&result)?
                                }
                                Err(e) => {
                                    format!("Tool error: {}", e)
                                }
                            }
                        } else {
                            format!("Unknown tool: {}", function_name)
                        };
                        
                        // Add tool response to conversation
                        messages.push(ChatMessage {
                            role: "tool".to_string(),
                            content: Some(output),
                            tool_calls: None,
                            tool_call_id: Some(tool_call.id.clone()),
                        });
                    }
                } else {
                    // No tools used, this is the final response
                    if let Some(content) = &message.content {
                        if !content.trim().is_empty() {
                            println!("{}", content);
                        }
                    }
                    break;
                }
            } else {
                return Err(anyhow::anyhow!("No response from OpenAI"));
            }
        }

        Ok(())
    }

    async fn make_openai_request(
        &self,
        messages: &[ChatMessage],
        model: &str,
        temperature: f32,
        max_tokens: u32,
        base_url: &str,
        api_key: &str,
        tool_registry: &ts_runtime::ToolRegistry,
    ) -> Result<OpenAIResponse> {
        // Start with built-in run_command tool
        let mut tools = vec![json!({
            "type": "function",
            "function": {
                "name": "run_command",
                "description": "Execute a shell command and return the output",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The shell command to execute"
                        }
                    },
                    "required": ["command"]
                }
            }
        })];
        
        // Add TypeScript-defined tools
        for (_, tool) in &tool_registry.tools {
            tools.push(json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.parameters
                }
            }));
        }
        
        let tools = json!(tools);

        let request_body = json!({
            "model": model,
            "messages": messages,
            "tools": tools,
            "tool_choice": "auto",
            "temperature": temperature,
            "max_tokens": max_tokens
        });

        let response = self.client
            .post(&format!("{}/chat/completions", base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("OpenAI API error: {}", error_text));
        }

        let openai_response: OpenAIResponse = response.json().await?;
        Ok(openai_response)
    }

    fn execute_command(&self, command: &str, current_dir: &PathBuf) -> Result<String> {
        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(current_dir)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let mut result = String::new();
        if !stdout.is_empty() {
            result.push_str(&stdout);
        }
        if !stderr.is_empty() {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str("STDERR: ");
            result.push_str(&stderr);
        }

        // Also show the command exit status if it failed
        if !output.status.success() {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(&format!("Exit code: {}", 
                output.status.code().unwrap_or(-1)));
        }

        Ok(result)
    }
}


struct AishShell {
    editor: DefaultEditor,
    config: Config,
    ai_agent: AiAgent,
    current_dir: PathBuf,
    mode: ShellMode,
    mode_toggle_handler: ModeToggleHandler,
    ts_config_loader: ts_runtime::TypeScriptConfigLoader,
}

impl AishShell {
    async fn new() -> Result<Self> {
        let mut editor = DefaultEditor::new()
            .map_err(|e| anyhow::anyhow!("Failed to create editor: {}", e))?;
        
        // Create mode toggle handler
        let mode_toggle_handler = ModeToggleHandler::new();
        
        // Bind ESC-x (Alt+x) to mode toggle
        editor.bind_sequence(
            KeyEvent::alt('x'),
            EventHandler::Conditional(Box::new(mode_toggle_handler.clone())),
        );
        
        let ts_config_loader = ts_runtime::TypeScriptConfigLoader::new()?;
        let config = ts_config_loader.load_config().await?;
        let ai_agent = AiAgent::new(config.clone());
        let current_dir = env::current_dir()?;
        
        // Initialize mode from environment or default to Agent
        let mode = env::var("AISH_MODE")
            .map(|m| ShellMode::from_str(&m))
            .unwrap_or(ShellMode::Agent);
        
        // Set the environment variable to match our mode
        unsafe {
            env::set_var("AISH_MODE", mode.as_str());
        }
        
        Ok(Self {
            editor,
            config,
            ai_agent,
            current_dir,
            mode,
            mode_toggle_handler,
            ts_config_loader,
        })
    }
    
    fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            ShellMode::Agent => ShellMode::Command,
            ShellMode::Command => ShellMode::Agent,
        };
        
        // Update environment variable
        unsafe {
            env::set_var("AISH_MODE", self.mode.as_str());
        }
        
        // Print mode change notification
        println!("\nMode switched to: {}", self.mode.as_str().to_uppercase());
    }

    async fn run(&mut self) -> Result<()> {
        println!("Welcome to aish (AI Shell) v0.1.0");
        println!("Current mode: {}", self.mode.as_str().to_uppercase());
        println!("Type 'exit' to quit, 'help' for help, press ESC then x to toggle mode");
        if self.mode == ShellMode::Agent {
            println!("Prefix commands with '$' for Unix shell execution");
        } else {
            println!("All commands are executed as Unix shell commands");
        }
        println!("Use '\\' at the end of a line for multiline commands");
        println!();

        loop {
            let command = self.read_command().await?;
            
            if command.is_empty() {
                continue;
            }

            if let Some(should_exit) = self.handle_input(&command).await {
                if should_exit {
                    break;
                }
            }
        }
        Ok(())
    }

    async fn read_command(&mut self) -> Result<String> {
        let mut command = String::new();
        let mut continuation = false;
        
        // Try to get custom prompt from TypeScript function first
        let prompt = if let Ok(Some(custom_prompt)) = self.ts_config_loader.call_prompt_function("customPrompt").await {
            custom_prompt
        } else {
            self.config.get_prompt(&self.current_dir, &self.mode)
        };
        
        let continuation_prompt = self.config.get_continuation_prompt(&self.current_dir, &self.mode);

        loop {
            let current_prompt = if continuation { &continuation_prompt } else { &prompt };
            
            // Check if mode toggle was triggered by ESC-x
            if self.mode_toggle_handler.check_toggle() {
                self.toggle_mode();
                if continuation {
                    command.clear();
                    continuation = false;
                }
                continue; // Re-prompt with new mode
            }
            
            match self.editor.readline(current_prompt) {
                Ok(line) => {
                    let trimmed = line.trim();
                    
                    if trimmed.is_empty() && !continuation {
                        return Ok(String::new());
                    }
                    

                    if trimmed.ends_with('\\') && !trimmed.ends_with("\\\\") {
                        let line_without_backslash = &trimmed[..trimmed.len() - 1];
                        if !command.is_empty() {
                            command.push(' ');
                        }
                        command.push_str(line_without_backslash);
                        continuation = true;
                    } else {
                        if !command.is_empty() {
                            command.push(' ');
                        }
                        command.push_str(trimmed);
                        
                        if !command.trim().is_empty() {
                            self.editor.add_history_entry(&command)?;
                        }
                        break;
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    // Check if this was a mode toggle
                    if self.mode_toggle_handler.check_toggle() {
                        self.toggle_mode();
                        if continuation {
                            command.clear();
                            continuation = false;
                        }
                        continue; // Re-prompt with new mode
                    }
                    
                    // Regular Ctrl+C handling
                    if continuation {
                        println!("^C");
                        command.clear();
                        continuation = false;
                        continue;
                    } else {
                        println!("^C");
                        return Ok(String::new());
                    }
                }
                Err(ReadlineError::Eof) => {
                    if continuation {
                        println!("^D");
                        return Ok(command);
                    } else {
                        println!("^D");
                        std::process::exit(0);
                    }
                }
                Err(err) => {
                    return Err(anyhow::anyhow!("Readline error: {:?}", err));
                }
            }
        }

        Ok(command)
    }

    async fn handle_input(&mut self, input: &str) -> Option<bool> {
        let trimmed = input.trim();
        
        match trimmed {
            "exit" | "quit" => {
                println!("Goodbye!");
                return Some(true);
            }
            "help" => {
                self.show_help();
                return Some(false);
            }
            _ => {}
        }
        
        match self.mode {
            ShellMode::Agent => {
                // Agent mode: $ prefix for Unix commands, everything else for AI
                if trimmed.starts_with('$') {
                    let command = trimmed[1..].trim();
                    if !command.is_empty() {
                        if let Err(e) = self.execute_unix_command(command) {
                            eprintln!("Error: {}", e);
                        }
                    }
                } else {
                    if let Err(e) = self.handle_ai_prompt(trimmed).await {
                        eprintln!("Error: {}", e);
                    }
                }
            }
            ShellMode::Command => {
                // Command mode: everything is a Unix command
                if let Err(e) = self.execute_unix_command(trimmed) {
                    eprintln!("Error: {}", e);
                }
            }
        }
        
        Some(false)
    }

    async fn handle_ai_prompt(&mut self, prompt: &str) -> Result<()> {
        if prompt.is_empty() {
            return Ok(());
        }
        
        match self.ai_agent.process_prompt(prompt, &self.current_dir, &self.ts_config_loader).await {
            Ok(()) => Ok(()),
            Err(e) => {
                eprintln!("AI Error: {}", e);
                Ok(())
            }
        }
    }

    fn show_help(&self) {
        println!("aish (AI Shell) - A shell that handles both natural language and Unix commands");
        println!();
        println!("Current mode: {}", self.mode.as_str().to_uppercase());
        println!();
        println!("Built-in commands:");
        println!("  help     - Show this help message");
        println!("  exit     - Exit the shell");
        println!("  quit     - Exit the shell");
        println!("  ESC then x - Toggle between AGENT and COMMAND modes (Alt+x)");
        println!();
        
        match self.mode {
            ShellMode::Agent => {
                println!("AGENT MODE - Command routing:");
                println!("  $ <command>  - Execute Unix shell command (e.g., '$ ls -la')");
                println!("  <text>       - AI prompt for natural language processing");
                println!();
                println!("Examples:");
                println!("  $ echo 'Hello World'     - Execute echo command");
                println!("  list all files           - AI prompt to list files");
                println!("  what is the weather?      - AI prompt for weather");
            }
            ShellMode::Command => {
                println!("COMMAND MODE - All input is executed as Unix commands:");
                println!("  <command>    - Execute Unix shell command directly");
                println!();
                println!("Examples:");
                println!("  ls -la                    - Execute ls command");
                println!("  echo 'Hello World'        - Execute echo command");
                println!("  cd /tmp                   - Change directory");
            }
        }
    }

    fn execute_unix_command(&mut self, input: &str) -> Result<()> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }

        let command = parts[0];
        let args = &parts[1..];

        // Handle cd command specially
        if command == "cd" {
            let target_dir = if args.is_empty() {
                dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
            } else {
                let path = PathBuf::from(args[0]);
                if path.is_absolute() {
                    path
                } else {
                    self.current_dir.join(path)
                }
            };

            match env::set_current_dir(&target_dir) {
                Ok(()) => {
                    self.current_dir = target_dir;
                    println!("Changed directory to: {}", self.current_dir.display());
                }
                Err(e) => {
                    eprintln!("cd: {}: {}", target_dir.display(), e);
                }
            }
            return Ok(());
        }

        let mut cmd = Command::new(command);
        cmd.args(args);
        cmd.current_dir(&self.current_dir);
        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        match cmd.status() {
            Ok(status) => {
                if !status.success() {
                    if let Some(code) = status.code() {
                        eprintln!("Command exited with code: {}", code);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to execute command '{}': {}", command, e);
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if let Some(command) = args.command {
        let mut shell = AishShell::new().await?;
        shell.handle_input(&command).await;
    } else {
        let mut shell = AishShell::new().await?;
        shell.run().await?;
    }

    Ok(())
}
