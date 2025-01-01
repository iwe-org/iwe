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

### Helix Editor

Make sure you have the `iwes` binary in your path, then add to your `languages.toml` (usually in `~/.config/helix`):

``` toml
[language-server.iwe]
command = "iwes"

[[language]]
name = "markdown"
language-servers = [ "iwe" ] # you might want more LSs in here
auto-format = true # optional, enable format-on-save
```

Then run:

``` bash
hx --health markdown
```

To make sure everything is how you would expect.

### Visual Studio Code

Contributors are welcome.
