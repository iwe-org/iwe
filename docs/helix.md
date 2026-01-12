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

> TODO: add specific examples and keybindings in Helix
>
> TODO: document Helix-specific quirks (e.g. need to manually delete buffer after inlining a section)
