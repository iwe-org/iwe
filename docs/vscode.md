# VS Code

## Installation & Setup

### Install the IWE Extension

The IWE extension is available on the Visual Studio Code Marketplace:

**Option 1: Via VS Code Marketplace**
1. Open VS Code
2. Go to Extensions view (`Ctrl+Shift+X` / `Cmd+Shift+X`)
3. Search for "IWE"
4. Click "Install" on the IWE extension

**Option 2: Via Command Line**
```bash
code --install-extension IWE.iwe
```

**Option 3: Direct Link**
Visit the [IWE extension on VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=IWE.iwe)

### Prerequisites

The IWE extension requires the `iwes` LSP server binary to be installed on your system:

1. **Install via Cargo** (recommended):
   ```bash
   cargo install iwe
   ```

2. **Download from GitHub Releases**:
   - Visit [IWE releases](https://github.com/iwe-org/iwe/releases)
   - Download the appropriate binary for your system
   - Ensure `iwes` is in your system PATH

3. **Build from Source**:
   ```bash
   git clone https://github.com/iwe-org/iwe.git
   cd iwe
   cargo build --release --bin iwes
   # Copy target/release/iwes to your PATH
   ```

### Verify Installation

1. Open VS Code in a directory with markdown files
2. Open a `.md` file
3. Check the bottom status bar - you should see "IWE" indicating the language server is active
4. Try using IWE features (see shortcuts below)

## VS Code Shortcuts for IWE Actions

### Core Actions

| IWE Feature | VS Code Shortcut | Alternative Access |
|-------------|------------------|-------------------|
| **Code Actions** (Extract/Inline/AI/Transform) | `Ctrl+.` / `Cmd+.` | Right-click → "Quick Fix..." |
| **Go to Definition** (Follow Links) | `F12` | Right-click → "Go to Definition" |
| **Find All References** (Backlinks) | `Shift+F12` | Right-click → "Go to References" |
| **Document Symbols** (Table of Contents) | `Ctrl+Shift+O` / `Cmd+Shift+O` | Command Palette → "Go to Symbol" |
| **Workspace Search** (Global Search) | `Ctrl+T` / `Cmd+T` | Command Palette → "Go to Symbol in Workspace" |
| **Format Document** (Auto-Format) | `Shift+Alt+F` / `Shift+Option+F` | Right-click → "Format Document" |
| **Rename Symbol** (Rename File) | `F2` | Right-click → "Rename Symbol" |

### Additional VS Code Features

| Feature | Shortcut | Description |
|---------|----------|-------------|
| **Command Palette** | `Ctrl+Shift+P` / `Cmd+Shift+P` | Access all IWE commands |
| **Auto-Complete** | `Ctrl+Space` / `Cmd+Space` | Trigger link completion while typing |
| **Peek Definition** | `Alt+F12` / `Option+F12` | Preview linked document without opening |
| **Peek References** | `Shift+Alt+F12` / `Shift+Option+F12` | Preview backlinks without opening |

### Command Palette Access

All IWE features are also available via the Command Palette (`Ctrl+Shift+P` / `Cmd+Shift+P`):

- Type "IWE" to see all available commands
- Type "Go to" for navigation commands
- Type "Format" for formatting commands
- Type "Rename" for refactoring commands

## Usage Examples

### Extracting a Section

1. Place cursor on a header line (e.g., `## Section Title`)
2. Press `Ctrl+.` / `Cmd+.` to open Quick Actions
3. Select "Extract section"
4. VS Code will create a new file and replace the section with a link

### Following Links

1. Click on any markdown link or place cursor within brackets
2. Press `F12` or `Ctrl+Click` / `Cmd+Click`
3. VS Code will navigate to the target document

### Finding Backlinks

1. Place cursor on a header or anywhere in a document
2. Press `Shift+F12`
3. VS Code will show all documents that link to the current location
4. Click any result to navigate

### AI-Powered Actions

1. Select text you want to modify
2. Press `Ctrl+.` / `Cmd+.`
3. Choose from available AI actions (if configured)
4. The selected text will be processed and replaced

### Global Search

1. Press `Ctrl+T` / `Cmd+T`
2. Type search terms
3. VS Code will show matching documents and sections
4. Use arrow keys to navigate results, Enter to open

## Configuration

### Workspace Settings

Create or edit `.vscode/settings.json` in your workspace:

```json
{
  "iwe.enable": true,
  "iwe.trace.server": "off",
  "files.associations": {
    "*.md": "markdown"
  },
  "markdown.validate.enabled": true
}
```

### User Settings

For global IWE configuration, edit your VS Code user settings:

1. Open Settings (`Ctrl+,` / `Cmd+,`)
2. Search for "IWE"
3. Configure available options

### IWE Project Configuration

Create `.iwe/config.toml` in your project root:

```toml
[library]
path = ""  # Subdirectory for markdown files (empty = root)

[markdown]
normalize_headers = true
normalize_lists = true

# AI Configuration (optional)
[models.default]
api_key_env = "OPENAI_API_KEY"
base_url = "https://api.openai.com"
name = "gpt-4o"

[actions.rewrite]
title = "Improve Text"
model = "default"
context = "Document"
prompt_template = "Improve this text: {{context}}"
```

## Features in VS Code

### IntelliSense and Auto-Complete

- **Link Completion**: Type `[` and get suggestions for existing documents
- **Smart Suggestions**: Context-aware completions based on document structure
- **Snippet Support**: Quick insertion of common markdown patterns

### Visual Enhancements

- **Inlay Hints**: See parent document references and link counts
- **Syntax Highlighting**: Enhanced markdown highlighting with IWE-specific elements
- **Error Detection**: Real-time validation of links and structure

### File Management

- **Auto-Save**: New files created by extraction are automatically saved
- **File Watching**: Changes are tracked and processed in real-time
- **Project Integration**: Works with VS Code's built-in file explorer

## Troubleshooting

### Common Issues

1. **LSP Server Not Starting**
   - Check that `iwes` is installed and in PATH
   - Restart VS Code
   - Check Output panel → "IWE Language Server" for errors

2. **Features Not Working**
   - Ensure you're in a directory with `.iwe/config.toml`
   - Verify the file is saved as `.md`
   - Check VS Code status bar for IWE indicator

3. **Performance Issues**
   - Large workspaces may be slow; consider using library path configuration
   - Disable unnecessary VS Code extensions
   - Check system resources

### Debug Mode

Enable debug logging:

1. Set environment variable: `IWE_DEBUG=true`
2. Restart VS Code
3. Check the IWE log file in your workspace directory
4. Include logs when reporting issues

### Getting Help

- **GitHub Issues**: [Report bugs or request features](https://github.com/iwe-org/iwe/issues)
- **Discussions**: [Community support and questions](https://github.com/iwe-org/iwe/discussions)
- **Documentation**: [Full documentation wiki](https://github.com/iwe-org/iwe/wiki)

## Best Practices for VS Code

1. **Use Workspace Folders**: Open your entire knowledge base as a workspace folder
2. **Configure File Associations**: Ensure all markdown files are properly associated
3. **Enable Auto-Save**: Prevent data loss with VS Code's auto-save feature
4. **Use Split Views**: Work with multiple documents simultaneously
5. **Organize with Explorer**: Use VS Code's file explorer alongside IWE's navigation
6. **Keyboard Shortcuts**: Learn the shortcuts for faster workflow
7. **Extensions Integration**: IWE works well with other markdown extensions
