# Neovim

## Installation & Setup

### Install the IWE Plugin

The IWE Neovim plugin is available at: [iwe.nvim](https://github.com/iwe-org/iwe.nvim)

**Option 1: Using lazy.nvim (recommended)**

``` lua
{
  "iwe-org/iwe.nvim",
  dependencies = {
    "nvim-lua/plenary.nvim", "nvim-telescope/telescope.nvim",
  },
  config = function()
    require("iwe").setup()
  end,
}
```

**Option 2: Using packer.nvim**

``` lua
use {
  "iwe-org/iwe.nvim",
  requires = {
    "nvim-lua/plenary.nvim",
    "nvim-telescope/telescope.nvim",
  },
  config = function()
    require("iwe").setup()
  end,
}
```

**Option 3: Using vim-plug**

``` vim
Plug 'nvim-lua/plenary.nvim'
Plug 'nvim-telescope/telescope.nvim'
Plug 'iwe-org/iwe.nvim'

" Add to your init.vim after plug#end()
lua require("iwe").setup()
```

**Option 4: Manual Installation**

``` bash
git clone https://github.com/iwe-org/iwe.nvim.git ~/.local/share/nvim/site/pack/plugins/start/iwe.nvim
```

### Prerequisites

The IWE plugin requires the `iwes` LSP server binary to be installed on your system:

1.  **Install via Cargo** (recommended):
    ``` bash
    cargo install iwe
    ```
2.  **Download from GitHub Releases**:
    - Visit [IWE releases](https://github.com/iwe-org/iwe/releases)
    - Download the appropriate binary for your system
    - Ensure `iwes` is in your system PATH
3.  **Build from Source**:
    ``` bash
    git clone https://github.com/iwe-org/iwe.git
    cd iwe
    cargo build --release --bin iwes
    # Copy target/release/iwes to your PATH
    ```

### Verify Installation

1.  Open Neovim in a directory with markdown files
2.  Open a `.md` file
3.  Run `:checkhealth iwe` to verify the plugin is working
4.  Check `:LspInfo` to see if the IWE LSP server is attached

## Neovim Shortcuts for IWE Actions

### Default Keybindings

The plugin provides these default keybindings (can be customized):

|IWE Feature|Neovim Shortcut|Mode|Description|
|-----------|---------------|----|-----------|
|**Code Actions**||||

``` <leader>ca

```

|Normal|Extract/Inline/AI/Transform actions||**Go to Definition**|

``` gd

```

|Normal|Follow markdown links||**Find References**|

``` gr

```

|Normal|Find backlinks to current document||**Document Symbols**|

``` <leader>ds

```

|Normal|Navigate document outline||**Workspace Search**|

``` <leader>ws

```

|Normal|Global search with Telescope||**Format Document**|

``` <leader>f

```

|Normal/Visual|Auto-format document||**Rename Symbol**|

``` <leader>rn

```

|Normal|Rename file and update references|

### LSP Keybindings

Standard LSP keybindings that work with IWE:

|Feature|Shortcut|Description|
|-------|--------|-----------|
|**Hover Info**|||

``` K

```

|Show information about current element||**Signature Help**|

``` <C-k>

```

|Show function signature (in insert mode)||**Code Action**|

``` <leader>ca

```

|Show available code actions||**Diagnostic Next**|

``` ]d

```

|Go to next diagnostic||**Diagnostic Previous**|

``` [d

```

|Go to previous diagnostic|

### Telescope Integration

IWE integrates with Telescope for enhanced search capabilities:

|Command|Shortcut|Description|
|-------|--------|-----------|

|

``` :Telescope iwe search

```

|

``` <leader>ws

```

|Search through all notes||

``` :Telescope iwe backlinks

```

|

``` <leader>wb

```

|Find backlinks to current document||

``` :Telescope iwe links

```

|

``` <leader>wl

```

|Browse all links in current document|

## Configuration

### Basic Setup

``` lua
require("iwe").setup({
  -- LSP server configuration
  lsp = {
    -- Path to iwes binary (auto-detected if in PATH)
    cmd = { "iwes" },
    
    -- LSP server settings
    settings = {
      iwe = {
        debug = false,
      },
    },
  },
  
  -- Keybindings (set to false to disable default bindings)
  keybindings = {
    enable = true,
    
    -- Custom keybindings
    code_action = "<leader>ca",
    goto_definition = "gd",
    find_references = "gr",
    document_symbols = "<leader>ds",
    workspace_search = "<leader>ws",
    format_document = "<leader>f",
    rename_symbol = "<leader>rn",
  },
  
  -- Telescope integration
  telescope = {
    enable = true,
    
    -- Telescope-specific settings
    search = {
      layout_strategy = "horizontal",
      layout_config = {
        preview_width = 0.6,
      },
    },
  },
})
```

### Advanced Configuration

``` lua
require("iwe").setup({
  -- LSP configuration
  lsp = {
    cmd = { "iwes" },
    filetypes = { "markdown" },
    root_dir = function(fname)
      return require("lspconfig.util").root_pattern(".iwe")(fname)
        or require("lspconfig.util").find_git_ancestor(fname)
        or vim.loop.os_homedir()
    end,
    
    -- Custom capabilities
    capabilities = require("cmp_nvim_lsp").default_capabilities(),
    
    -- LSP server settings
    settings = {
      iwe = {
        debug = vim.env.IWE_DEBUG == "true",
        trace = "off", -- or "messages", "verbose"
      },
    },
    
    -- Custom handlers
    handlers = {
      ["textDocument/hover"] = vim.lsp.with(vim.lsp.handlers.hover, {
        border = "rounded",
      }),
    },
  },
  
  -- Disable default keybindings and set custom ones
  keybindings = {
    enable = false, -- Disable defaults
  },
  
  -- Telescope customization
  telescope = {
    enable = true,
    extensions = {
      iwe = {
        search = {
          prompt_title = "IWE Search",
          results_title = "Documents",
        },
        backlinks = {
          prompt_title = "Backlinks",
          results_title = "References",
        },
      },
    },
  },
  
  -- Health check configuration
  health = {
    check_iwes_binary = true,
    check_iwe_config = true,
  },
})

-- Custom keybindings
local map = vim.keymap.set
map("n", "<leader>ia", "<cmd>lua vim.lsp.buf.code_action()<cr>", { desc = "IWE Code Actions" })
map("n", "<leader>ig", "<cmd>lua vim.lsp.buf.definition()<cr>", { desc = "IWE Go to Definition" })
map("n", "<leader>ir", "<cmd>lua vim.lsp.buf.references()<cr>", { desc = "IWE Find References" })
map("n", "<leader>is", "<cmd>Telescope iwe search<cr>", { desc = "IWE Search" })
map("n", "<leader>ib", "<cmd>Telescope iwe backlinks<cr>", { desc = "IWE Backlinks" })
map("n", "<leader>if", "<cmd>lua vim.lsp.buf.format()<cr>", { desc = "IWE Format" })
map("n", "<leader>in", "<cmd>lua vim.lsp.buf.rename()<cr>", { desc = "IWE Rename" })
```

### Which-Key Integration

If you use which-key.nvim, add descriptions for IWE commands:

``` lua
require("which-key").register({
  ["<leader>i"] = {
    name = "IWE",
    a = "Code Actions",
    g = "Go to Definition",
    r = "Find References", 
    s = "Search",
    b = "Backlinks",
    f = "Format Document",
    n = "Rename",
  },
})
```

## Usage Examples

### Extracting a Section

1.  Place cursor on a header line (e.g., `## Section Title`)
2.  Press `<leader>ca` to open code actions
3.  Select "Extract section" from the list
4.  Neovim will create a new buffer with the extracted content

### Following Links

1.  Place cursor on any markdown link
2.  Press `gd` to follow the link
3.  Use `<C-o>` to return to the previous location

### Finding Backlinks

1.  In any document, press `gr` or `<leader>wb`
2.  Telescope will show all documents linking to the current one
3.  Use arrow keys to navigate, `Enter` to open

### Global Search with Telescope

1.  Press `<leader>ws` to open IWE search
2.  Start typing to search across all documents
3.  Results show document paths and matching content
4.  Use `<C-p>` preview to see content without opening

### AI-Powered Actions (if configured)

1.  Select text in visual mode
2.  Press `<leader>ca` to show code actions
3.  Choose from available AI actions
4.  The text will be processed and replaced

## Telescope Commands

### Available Commands

``` vim
" Search through all notes
:Telescope iwe search

" Find backlinks to current document  
:Telescope iwe backlinks

" Browse links in current document
:Telescope iwe links

" Show document symbols/outline
:Telescope lsp_document_symbols

" Search workspace symbols
:Telescope lsp_workspace_symbols
```

### Telescope Keybindings (within picker)

|Key|Action|
|---|------|

|

``` <CR>

```

|Open selected item||

``` <C-x>

```

|Open in horizontal split||

``` <C-v>

```

|Open in vertical split||

``` <C-t>

```

|Open in new tab||

``` <C-u>

```

|Scroll preview up||

``` <C-d>

```

|Scroll preview down||

``` <C-q>

```

|Send to quickfix list|

## Health Check

Run health checks to verify your setup:

``` vim
:checkhealth iwe
```

This will check:

- IWE plugin installation
- `iwes` binary availability
- LSP server configuration
- Telescope integration
- IWE project configuration

## Troubleshooting

### Common Issues

1.  **LSP Server Not Starting**
    ``` bash
    # Check if iwes is in PATH
    which iwes

    # Check LSP server status
    :LspInfo

    # View LSP logs
    :LspLog
    ```
2.  **Telescope Not Working**
    ``` lua
    -- Ensure telescope is loaded
    require("telescope").load_extension("iwe")
    ```
3.  **Keybindings Not Working**
    - Check if default keybindings are enabled in config
    - Verify no conflicts with other plugins
    - Use `:verbose map <key>` to check key mappings
4.  **Performance Issues**
    - Check `:IweStatus` for server information
    - Consider workspace size and complexity
    - Enable debug mode temporarily: `IWE_DEBUG=true nvim`

### Debug Mode

Enable debug logging:

``` bash
# Start Neovim with debug mode
IWE_DEBUG=true nvim

# Or set in Neovim
:lua vim.env.IWE_DEBUG = "true"
:LspRestart
```

Debug logs will be written to `iwe.log` in your working directory.

### Getting Help

- **Plugin Repository**: [iwe.nvim Issues](https://github.com/iwe-org/iwe.nvim/issues)
- **Main Project**: [IWE Issues](https://github.com/iwe-org/iwe/issues)
- **Discussions**: [Community Support](https://github.com/iwe-org/iwe/discussions)
- **Documentation**: [Full Wiki](https://github.com/iwe-org/iwe/wiki)

## Integration with Other Plugins

### nvim-cmp (Autocompletion)

``` lua
require("cmp").setup({
  sources = {
    { name = "nvim_lsp" }, -- Includes IWE completions
    { name = "buffer" },
    { name = "path" },
  },
})
```

### nvim-treesitter

``` lua
require("nvim-treesitter.configs").setup({
  ensure_installed = { "markdown", "markdown_inline" },
  highlight = { enable = true },
})
```

### gitsigns.nvim

IWE works well with git integration for version control of your knowledge base.

## Best Practices for Neovim

1.  **Use Workspace Sessions**: Save and restore IWE workspace sessions
2.  **Configure LSP Properly**: Ensure proper root directory detection
3.  **Leverage Telescope**: Use fuzzy finding for efficient navigation
4.  **Set Up Health Checks**: Regular `:checkhealth iwe` for maintenance
5.  **Customize Keybindings**: Adapt shortcuts to your workflow
6.  **Use Splits and Tabs**: Work with multiple documents simultaneously
7.  **Enable Auto-Save**: Use `:set autowrite` to prevent data loss
8.  **Integrate with Git**: Version control your knowledge base
9.  **Configure Completion**: Set up nvim-cmp for link auto-completion
10. **Use Which-Key**: Document your IWE keybindings for easy reference
