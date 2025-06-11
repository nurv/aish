# aish (AI Shell) - Claude Development Guide

## Project Overview
aish is an AI-powered shell that combines traditional Unix shell capabilities with natural language processing. It's built in Rust and designed to be a primary shell replacement that can handle both standard command-line operations and natural language inputs.

## Project Structure
```
aish/
├── src/
│   ├── main.rs          # Main shell implementation
│   └── ts_runtime/      # TypeScript runtime module
│       ├── mod.rs       # TypeScript configuration loader
│       ├── isolate.rs   # Deno isolate management
│       └── ops.rs       # Rust ops exposed to TypeScript
├── Cargo.toml           # Rust dependencies and project configuration
├── CLAUDE.md           # This development guide
└── README.md           # User-facing documentation
```

## Key Dependencies
- `rustyline`: For readline functionality and command history
- `clap`: For command-line argument parsing
- `anyhow`: For error handling
- `tokio`: For async runtime and AI API calls
- `deno_core` + `deno_runtime`: For TypeScript/JavaScript execution via Deno
- `basic_deno_ts_module_loader`: For TypeScript transpilation and HTTP imports
- `deno_error_mapping`: For proper error handling in Deno runtime
- `serde_v8`: For serialization between Rust and V8 JavaScript engine
- `dirs`: For cross-platform home directory detection
- `reqwest`: For HTTP client to OpenAI API
- `serde_json`: For JSON serialization/deserialization

## Current Features
1. **Interactive Shell**: REPL with history and line editing
2. **Dual Mode System**: Switch between AGENT and COMMAND modes with `ESC-x`
3. **Unix Command Execution**: Execute any Unix command with `$` prefix in AGENT mode
4. **AI Agent System**: GPT-4 powered agent with tool calling for command execution
5. **Working Directory Management**: Persistent `cd` command support across sessions
6. **Multiline Commands**: Support for backslash continuation (`\`)
7. **Built-in Commands**: `help`, `exit`, `quit`, `ESC-x` (mode toggle)
8. **Command-line Mode**: Execute single commands with `-c` flag
9. **TypeScript Configuration**: Load settings from `~/.aish.ts` with full TypeScript support
10. **PS1-style Prompts**: Environment variables and escape sequences in prompts
11. **Custom Prompt Functions**: TypeScript functions for dynamic prompt generation
12. **Extensible Agent Tools**: Plugin system for AI agent tool registration via TypeScript

## Architecture
- `AishShell` struct manages the shell state, configuration, readline editor, and working directory
- `AiAgent` struct handles OpenAI API communication with tool calling support
- `TypeScriptConfigLoader` handles TypeScript configuration loading and function execution
- `TypeScriptIsolate` manages Deno runtime for executing TypeScript code
- `read_command()` handles multiline input with backslash continuation
- `handle_input()` routes commands to Unix execution or AI prompt processing
- `execute_unix_command()` spawns Unix processes for `$`-prefixed commands with directory context
- Working directory persistence across commands and AI agent tool calls
- Error handling uses the `anyhow` crate for clean error propagation

## AI Agent System
The shell includes a sophisticated AI agent powered by OpenAI's GPT-4 that can:

### Tool System
- **`run_command`**: Execute shell commands and return output
- **Directory Awareness**: All commands run in the current working directory
- **Multi-turn Conversations**: Agent maintains context across tool calls
- **Error Handling**: Graceful handling of command failures and API errors

### Conversation Flow
1. User provides natural language prompt (no `$` prefix)
2. AI agent analyzes the request and determines if commands are needed
3. Agent uses `run_command` tool to execute necessary commands
4. Agent processes command output and provides helpful response
5. Conversation ends when agent provides final text response

### Example Interaction
```
aish> show me the git status
**** Running command
   $ git status
[Command output displayed]
[AI provides interpretation of the git status]
```

## Configuration System
The shell loads configuration from `~/.aish.yaml` or `~/.aish.yml` (in that order of priority). If no config file exists, sensible defaults are used.

### Configuration Structure
```yaml
# AI Settings (for future use)
ai:
  model: "gpt-4"                    # AI model to use
  api_key: "your-key-here"          # API key (optional)
  base_url: "https://api.custom"    # Custom API endpoint (optional)
  temperature: 0.7                  # Response creativity (0.0-1.0)
  max_tokens: 1000                  # Maximum response length

# Shell Behavior
shell:
  prompt: "aish> "                  # Primary prompt (supports PS1-style variables)
  history_size: 1000                # Command history size
  multiline_continuation: "... "    # Continuation prompt (supports PS1-style variables)
  mode_toggle_key: "esc-x"          # Key combination to toggle between modes
```

### Default Values
- **Prompt**: `"aish> "` (supports PS1-style variables and escape sequences)
- **Continuation**: `"... "` (supports PS1-style variables and escape sequences)
- **History Size**: 1000 entries
- **AI Model**: `"gpt-3.5-turbo"`
- **Temperature**: 0.7
- **Max Tokens**: 1000

### Prompt Configuration (PS1-style)
The prompt configuration supports PS1-style escape sequences and environment variables:

#### Environment Variables
- `$VAR` or `${VAR}` - Expand environment variable
- Example: `prompt: "$USER@$HOSTNAME:aish> "`

#### PS1 Escape Sequences
- `\u` - Current username
- `\h` - Hostname (short form)
- `\H` - Hostname (full form)
- `\w` - Current working directory with ~ for home
- `\W` - Basename of current working directory
- `\m` - Current shell mode (agent/command)
- `\M` - Current shell mode in uppercase (AGENT/COMMAND)
- `\$` - `#` if root, `$` otherwise
- `\n` - Newline
- `\t` - Tab
- `\[` - Start of ANSI escape sequence (for colors)
- `\]` - End of ANSI escape sequence

#### Example Configurations
```yaml
shell:
  # Simple username and directory
  prompt: "\u@\h:\W$ "
  
  # Show current mode in prompt
  prompt: "[\m] \u@\h:\W$ "
  
  # Colorized prompt with mode and full path
  prompt: "\[\033[01;33m\][\M]\[\033[00m\] \[\033[01;32m\]\u@\h\[\033[00m\]:\[\033[01;34m\]\w\[\033[00m\]$ "
  
  # Environment variable expansion
  prompt: "$USER in \w > "
  
  # Multi-line prompt with mode
  prompt: "\u@\h:\w [\m]\n$ "
```

## Development Commands
```bash
# Build the project
cargo build

# Run the shell
cargo run

# Run with a single command
cargo run -- -c "ls -la"

# Run tests (when available)
cargo test

# Format code
cargo fmt

# Check code
cargo check
```

## Mode System
Aish operates in two distinct modes that can be toggled using the `ESC-x` command:

### Agent Mode (Default)
- Natural language inputs are processed by the AI agent
- Unix commands require `$` prefix (e.g., `$ ls -la`)
- AI agent can execute commands using the `run_command` tool
- Best for interactive AI assistance and command explanation

### Command Mode
- All inputs are executed as Unix shell commands directly
- No `$` prefix needed
- Functions like a traditional Unix shell
- Best for traditional shell scripting and command-line work

### Mode Switching
- Press `ESC` then `x` (or `Alt+x`) to toggle between modes
- Key binding implemented using rustyline's custom key binding system
- Current mode is stored in `AISH_MODE` environment variable
- Mode can be displayed in prompt using `\m` (lowercase) or `\M` (uppercase)
- Mode change is immediate and displays confirmation message

### Examples
```bash
# In AGENT mode
aish> list all files
**** Running command
   $ ls -la
[AI provides file listing with explanation]

aish> $ pwd
/Users/username/project

aish> # Press ESC then x to switch to COMMAND mode
Mode switched to: COMMAND

# In COMMAND mode
[command] aish> ls -la
[Direct command execution]

[command] aish> # Press ESC then x to switch back to AGENT mode
Mode switched to: AGENT
```

## Working Directory Management
The shell maintains persistent working directory state across all operations:

### Built-in `cd` Command
- `$ cd <path>`: Change to specified directory (absolute or relative)
- `$ cd`: Change to user's home directory
- Directory changes persist for all subsequent commands
- Both Unix commands and AI agent tools respect current directory

### Implementation Details
- `AishShell.current_dir` field tracks working directory
- `execute_unix_command()` sets `.current_dir()` on all `Command` instances
- AI agent's `run_command` tool executes in current directory context
- `env::set_current_dir()` updates the process working directory

## Future Development Areas
1. **Enhanced AI Capabilities**: More sophisticated tool system and context awareness
2. **Plugin System**: Extensible architecture for additional functionality
3. **Advanced Shell Features**: Pipes, redirections, job control
4. **History Management**: Persistent history with search capabilities
5. **Completion System**: Tab completion for commands and files
6. **Multiple AI Model Support**: Support for different AI providers and models

## Code Style
- Follow standard Rust conventions
- Use `anyhow::Result` for error handling
- Prefer explicit error handling over panicking
- Use descriptive variable names
- Keep functions focused and single-purpose

## Testing Strategy
- Unit tests for individual functions
- Integration tests for shell behavior
- Manual testing for interactive features
- Performance testing for command execution

## Deployment
The shell can be installed as a system shell once mature:
1. Build release binary: `cargo build --release`
2. Install to system: `sudo cp target/release/aish /usr/local/bin/`
3. Add to `/etc/shells` for system recognition
4. Set as default shell: `chsh -s /usr/local/bin/aish`

## Known Limitations
- No pipe or redirection support yet
- No job control (background processes)
- No advanced tab completion
- No configuration file support
- No AI/NL processing capability yet

## Development Notes
- The shell uses `rustyline` for advanced readline features
- Command execution inherits stdio for proper terminal interaction
- Multiline support uses backslash continuation similar to bash
- Error messages are user-friendly and informative