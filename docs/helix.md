# Helix

## Installation & Setup

First, the `iwes` binary needs to be available on your system `$PATH`. Please see the [installation instructions](./how-to-install.md) and pick your preferred method of installation. I recommend the AUR for Arch users.

Next, you'll need to add `iwe` as an LSP and enable it for files.

### Setup Snippet

``` toml
# `$HOME/.config/helix/languages.toml`

[language-server.iwe]
command = "iwes"

[[language]]
name = "markdown"
language-servers = ["iwe"]
# You can add other LSPs here, too:
# language-servers = ["iwe", "marksman"]

# NOTE: You may consider disabling 
# autoformat if you're having issues 
# with tables!
auto-format = true
```

### Setup IWE Only For Your Notes

You probably don't want `iwe` enabled for **every Markdown file you ever open**. For example, you may not want its features when you're working on README files for different projects. In that case, I recommend Helix's project-specific configuration feature. In the root of your notes directory, you can create a folder called `.helix`, add a file called `languages.toml` and put the [setup snippet](#setup-snippet) in there.

## Usage

Please refer to the [usage guide](./how-to-use.md) for a quick reference.

### Hover Preview

To preview a wiki/markdown-linked note without navigating away, place your cursor on the link and trigger LSP hover (Helix default: `space` + `k`).

More details: [Hover preview](feature-hover-preview.md)

### Common Keybindings

| Action | Keybinding |
|--------|------------|
| Go to definition (follow link) | `gd` |
| Find references (backlinks) | `gr` |
| Hover preview | `space` + `k` |
| Code actions | `space` + `a` |
| Document symbols (outline) | `space` + `s` |
| Workspace symbols (search) | `space` + `S` |
| Rename file | `space` + `r` |
| Format document | `:format` |

### Code Actions

To use IWE code actions (extract, inline, attach, etc.):

1. Place cursor on the target content
2. Press `space` + `a` to open code actions menu
3. Select the desired action

### Helix-Specific Notes

- **Buffer management after inline**: When you inline a section (merge an extracted note back), the buffer for the deleted file remains open. Use `:buffer-close` or `:bc` to close it manually.

- **Auto-format with tables**: If you work with markdown tables, you may want to disable auto-format to prevent unwanted table reformatting:

```toml
[[language]]
name = "markdown"
language-servers = ["iwe"]
auto-format = false
```

- **Multiple language servers**: You can use IWE alongside other markdown language servers:

```toml
[[language]]
name = "markdown"
language-servers = ["iwe", "marksman"]
```
