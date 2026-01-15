# Hover Preview

IWE supports the standard LSP `textDocument/hover` request to preview the contents of a linked note without navigating away from the current document.

## Supported links

- Wiki links: `[[note]]`
- Markdown links: `[title](note)`

External links (e.g. `https://...`, `mailto:...`) are ignored.

## Preview content

- Returns the target note as Markdown (`MarkupContent` with kind `markdown`).
- Strips frontmatter at the top of the note (delimited by `---` and terminated by `---` or `...`).
- Returns the rest of the document without truncation so your editor can decide how to render/clip it.

## Editor usage

- Helix: place the cursor on a link, then `space` + `k`
- Neovim: place the cursor on a link, then `K` (or run `:lua vim.lsp.buf.hover()`)
- VS Code: hover the link (or run “Show Hover”)
