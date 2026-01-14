# Zed

IWE integrates with Zed editor through an official extension that provides LSP support for markdown files.

## Installation

### From Zed Extensions

1. Open Zed
2. Open the Extensions panel (`Cmd+Shift+X` on macOS)
3. Search for "IWE"
4. Click Install

The extension will automatically download the `iwes` language server binary when first activated.

### Manual Installation

If you prefer to manage the binary yourself:

1. Install `iwes` using one of the methods from the [installation guide](how-to-install.md)
2. Ensure `iwes` is available in your `$PATH`
3. Install the IWE extension from Zed Extensions

The extension will use the system `iwes` binary if available, otherwise it downloads from GitHub releases.

## Setup

### Enable for Specific Projects

To enable IWE only for your notes directory (not all markdown files), create a `.zed/settings.json` in your notes root:

```json
{
  "lsp": {
    "iwe": {
      "binary": {
        "path": "iwes"
      }
    }
  }
}
```

### Initialize IWE

Create an IWE project in your notes directory:

```bash
cd ~/notes
iwes init
```

This creates the `.iwe/config.toml` configuration file.

## Usage

Please refer to the [usage guide](how-to-use.md) for a quick reference.

### Common Keybindings

| Action | Keybinding |
|--------|------------|
| Go to definition (follow link) | `F12` or `Cmd+Click` |
| Find references (backlinks) | `Shift+F12` |
| Code actions | `Cmd+.` |
| Format document | `Cmd+Shift+I` |
| Document symbols (outline) | `Cmd+Shift+O` |
| Workspace symbols (search) | `Cmd+T` |
| Rename | `F2` |

### Code Actions

To use IWE code actions (extract, inline, attach, etc.):

1. Place cursor on the target content
2. Press `Cmd+.` to open code actions menu
3. Select the desired action

## Troubleshooting

### Extension Not Working

1. Check that the extension is installed and enabled
2. Verify that your notes directory has `.iwe/config.toml`
3. Check Zed's LSP logs for errors

### Binary Not Found

If the extension fails to download the binary:

1. Install `iwes` manually from [installation guide](how-to-install.md)
2. Ensure it's in your `$PATH`
3. Restart Zed

## Platform Support

The extension supports:
- macOS (Apple Silicon and Intel)
- Linux (x86_64 and aarch64)

Windows support is planned for a future release.
