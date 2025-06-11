# aish - AI Shell

A modern shell that combines traditional Unix command-line capabilities with AI-powered natural language processing.

## Features

- **Dual Mode System**: Toggle between AGENT and COMMAND modes with `ESC-x`
- **Agent Mode**: AI prompts without prefix, Unix commands with `$` prefix
- **Command Mode**: All input executed as Unix shell commands
- **AI-Powered Agent**: GPT-4 integration with tool calling for intelligent command execution
- **Working Directory Management**: Persistent `cd` command support across sessions
- **PS1-style Prompts**: Environment variables and escape sequences including mode display
- **Full Unix Shell Compatibility**: Execute any Unix command with complete stdio support
- **YAML Configuration**: Customizable settings via `~/.aish.yaml` or `~/.aish.yml`
- **Multiline Commands**: Use backslash (`\`) continuation for complex commands
- **Interactive History**: Full readline support with command history and editing
- **Clean Interface**: Modern, user-friendly command prompt
- **Single Command Mode**: Execute commands directly with `-c` flag

## Installation

### From Source

```bash
# Clone the repository
git clone <repository-url>
cd aish

# Build the project
cargo build --release

# Install (optional)
sudo cp target/release/aish /usr/local/bin/
```

## Usage

### Interactive Mode

Start the shell:
```bash
./target/release/aish
```

### Command Modes

#### Unix Commands (with $ prefix)
```bash
aish> $ ls -la
aish> $ echo "Hello World"
aish> $ grep -r "pattern" .
```

#### AI Prompts (no prefix)
```bash
aish> list all files in the current directory
**** Running command
   $ ls -la
[Command output and AI interpretation]

aish> show me the git status
**** Running command
   $ git status
[Git status output with AI explanation]

aish> what files have been modified recently?
**** Running command
   $ find . -type f -mtime -1
[Recent files with AI analysis]
```

### Single Command Mode

Execute commands directly:
```bash
./target/release/aish -c "$ ls -la"           # Unix command
./target/release/aish -c "list all files"     # AI prompt
```

### Multiline Commands

Use backslash continuation for multiline commands:
```bash
aish> $ echo "This is a long command that" \
... "spans multiple lines"

aish> explain this complex algorithm that \
... processes data in multiple stages
```

## Built-in Commands

- `help` - Show help information
- `exit` - Exit the shell
- `quit` - Exit the shell
- `ESC then x` (or `Alt+x`) - Toggle between AGENT and COMMAND modes

## Shell Features

- **Command History**: Navigate through previous commands with arrow keys
- **Line Editing**: Full readline editing capabilities (Ctrl+A, Ctrl+E, etc.)
- **Interruption Handling**: Proper handling of Ctrl+C and Ctrl+D
- **Error Reporting**: Clear error messages for failed commands

## Configuration

Create a configuration file at `~/.aish.yaml` or `~/.aish.yml`:

```yaml
# AI Settings (for future AI integration)
ai:
  model: "gpt-4"
  temperature: 0.8
  max_tokens: 2000
  # api_key: "your-api-key-here"
  # base_url: "https://api.openai.com/v1"

# Shell Behavior  
shell:
  # Supports PS1-style variables and escape sequences
  prompt: "🤖 aish> "               # or try: "\u@\h:\W$ "
  history_size: 2000
  multiline_continuation: "... "    # also supports PS1-style variables
```

If no configuration file exists, sensible defaults are used automatically.

### Advanced Prompt Configuration

The prompt supports PS1-style escape sequences and environment variables:

```yaml
shell:
  # Username and hostname
  prompt: "\u@\h:\W$ "
  
  # Colorized prompt
  prompt: "\[\033[01;32m\]\u@\h\[\033[00m\]:\[\033[01;34m\]\w\[\033[00m\]$ "
  
  # Environment variables
  prompt: "$USER in \w > "
  
  # Multi-line
  prompt: "\u@\h:\w\n🤖 "
```

**Supported escape sequences:**
- `\u` - Username
- `\h` - Hostname  
- `\w` - Full working directory (~ for home)
- `\W` - Current directory name only
- `\m` - Current mode (agent/command)
- `\M` - Current mode uppercase (AGENT/COMMAND)
- `\$` - `#` if root, `$` otherwise
- `\n` - Newline
- `\[\033[...m\]` - ANSI color codes
- `$VAR` or `${VAR}` - Environment variables

**Important**: To use AI features, you must set your OpenAI API key in the configuration file:

```yaml
ai:
  api_key: "your-openai-api-key-here"
```

Without an API key, only Unix commands (with `$` prefix) will work. AI prompts will show an error message.

## Mode System

Aish operates in two modes that you can switch between:

### Agent Mode (Default)
- Natural language inputs are processed by AI
- Unix commands require `$` prefix: `$ ls -la`
- AI can execute commands and provide explanations
- Best for interactive assistance

### Command Mode  
- All inputs executed as Unix shell commands
- No `$` prefix needed: `ls -la`
- Functions like traditional shell
- Best for scripting and command-line work

### Switching Modes
Press `ESC` then `x` (or `Alt+x`) to toggle between modes. The current mode is shown in your prompt if configured with `\m` or `\M` escape sequences.

```bash
# Agent mode examples
aish> list all files
**** Running command
   $ ls -la
[AI explanation of files]

aish> $ pwd
/current/directory

aish> # Press ESC then x to switch to command mode
Mode switched to: COMMAND

# Command mode examples  
[command] user@host:~$ ls -la
[direct command output]

[command] user@host:~$ # Press ESC then x to switch back
Mode switched to: AGENT
```

## Examples

```bash
# Unix commands (with $ prefix)
aish> $ ls -la
aish> $ cd /tmp
Changed directory to: /tmp
aish> $ pwd
/tmp

# AI prompts (no prefix)
aish> list all rust files
**** Running command
   $ find . -name "*.rs"
[Files listed with AI explanation]

aish> show me recent git commits
**** Running command
   $ git log --oneline -10
[Commit history with AI summary]

# Multiline commands
aish> $ find . -name "*.rs" \
... -exec grep -l "main" {} \;

aish> analyze the performance of \
... this code and suggest improvements
**** Running command
   $ [AI determines appropriate profiling commands]

# Built-in help
aish> help
```

## Development

### Prerequisites

- Rust 1.70 or later
- Cargo

### Building

```bash
cargo build          # Debug build
cargo build --release # Release build
```

### Running

```bash
cargo run            # Run in development mode
cargo run -- -c "ls" # Run single command
```

### Testing

```bash
cargo test
```

## Roadmap

- [ ] AI-powered natural language command interpretation
- [ ] Advanced shell features (pipes, redirections, job control)
- [ ] Configuration system
- [ ] Plugin architecture
- [ ] Enhanced tab completion
- [ ] Persistent history with search

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.

## License

[License information to be added]

## Project Status

This is an early-stage project. The basic Unix shell functionality is complete and ready for use, with AI features planned for future releases.