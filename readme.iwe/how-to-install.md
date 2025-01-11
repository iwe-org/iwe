# How to install

## Prerequisites

- Rust and Cargo installed on your system. You can get them from [rustup.rs](https://rustup.rs).

## Installation

Clone the repository, navigate into the project directory, and build the project:

``` sh
git clone git@github.com:iwe-org/iwe.git
cd iwe
cargo build --release
```

This will create an executable located in the `target/release` directory.

## Editors

IWE can be used with any text editor with LSP support. IWE contains a special LSP binary called `iwes`.

### VIM integration

To enable IWE LSP for markdown files in VIM you need to make sure that `iwes` binary is in your path and add this to your config:

``` lua
vim.api.nvim_create_autocmd('FileType', {
  pattern = 'markdown',
  callback = function(args)
    vim.lsp.start({
      name = 'iwes',
      cmd = {'iwes'},
      root_dir = vim.fs.root(args.buf, {'.iwe' }),
      flags = {
        debounce_text_changes = 500
      }
    })
  end,
})

-- optional, enabled inlay hints
vim.lsp.inlay_hint.enable(not vim.lsp.inlay_hint.is_enabled())
```

And create `.iwe` directory as a marker in you notes root directory.

It works best with [render-markdown.nvim](https://github.com/MeanderingProgrammer/render-markdown.nvim/tree/main)

### Zed integration

IWE Zed [extension](https://github.com/iwe-org/zed-iwe) can be installed from the editor extensions menu.

The extension automatically fetches a pre-compiled binary of the LSP from a GitHub repository. If there is an LSP binary already installed on the system and it is accessible from the system's PATH, the extension will use that local binary instead of downloading a new one.

### Helix integration

Make sure you have the `iwes` binary in your path, then add to your `languages.toml` (usually in `~/.config/helix`, create file if needed):

``` toml
[language-server.iwe]
command = "iwes"

[[language]]
name = "markdown"
language-servers = [ "iwe" ] # you might want more LSP's in here
auto-format = true # optional, enable format-on-save
```

Then run:

``` sh
hx --health markdown
```

To see configured language servers.

### Visual Studio Code

Contributors are welcome.
