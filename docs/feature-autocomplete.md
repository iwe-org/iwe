# Auto-Complete

IWE can suggest links as you type using the standard LSP code completion feature.

## Trigger Characters

IWE asks the editor to open the completion popup as soon as the user types one of the configured trigger characters. The default is `[`, so typing `[` (or `[[`) immediately offers the list of documents.

``` toml
[completion]
trigger_characters = ["["]
```

The value is a list, so you can add any number of characters:

``` toml
[completion]
trigger_characters = ["[", "+", "@"]
```

Most editors also open the popup automatically as you type word characters (letters and digits), independently of this list — the list is only needed for non-word symbols. To restore the pre-0.1.7 behavior, set the list to `["+"]`.

> **Note (Neovim):** to make completions appear *only* after `[` / `[[` and never on word characters, use the built-in `vim.lsp.completion.enable(..., { autotrigger = true })` (Neovim 0.11+) — its autotrigger only fires on the server's trigger characters. With `nvim-cmp`, set the LSP source's `keyword_length = 999`; with `blink.cmp`, set the LSP provider's `min_keyword_length = 999`. Trigger characters still pass through in both cases.
>
> **Note (VS Code):** turn off word-character auto-suggestions for markdown while leaving trigger characters on:
>
> ``` json
> "[markdown]": {
>   "editor.quickSuggestions": { "other": "off", "comments": "off", "strings": "off" },
>   "editor.suggestOnTriggerCharacters": true
> }
> ```
>
> Typing `[` still opens the popup; typing letters no longer does. `Ctrl+Space` always invokes completions manually.

When the cursor is preceded by `[` or `[[`, the inserted completion replaces those brackets and the link shape is chosen by the typed prefix:

- `[` → markdown link `[title](key)`
- `[[` → wiki link `[[key]]`

This happens regardless of `link_format` — the typed brackets always win. When no bracket precedes the cursor, `link_format` decides the shape (see below).

If your editor auto-pairs `[` to `[]` (or `[[` to `[[]]`), the completion still works cleanly: the inserted range extends through any `]` / `]]` immediately after the cursor, so the result is a single well-formed link with no stray closing bracket.

## Link Format

By default, completions insert Markdown-style links `[title](key)`. You can configure IWE to use WikiLinks `[[key]]` instead:

``` toml
[completion]
link_format = "wiki"
```

Available options:

- `"markdown"` (default): Creates `[title](key)` style links
- `"wiki"`: Creates `[[key]]` style WikiLinks

This setting only applies when the cursor is not preceded by `[` or `[[`; an explicit bracket prefix always overrides it.

## Minimum prefix length

By default, IWE shows completions as soon as the editor requests them (no minimum length). Raise the threshold to suppress the popup until the user has typed a few characters:

``` toml
[completion]
min_prefix_length = 3
```

The length is measured against the search query after any leading `[` or `[[` is stripped — `[ab` is treated as 2 characters, not 3.

## Document Titles

By default, IWE uses the first header of a document as its title in completion suggestions. You can configure IWE to use a YAML frontmatter field instead:

``` toml
[library]
frontmatter_document_title = "title"
```

With this configuration, documents with frontmatter like:

``` markdown
---
title: Custom Document Title
---

# Header
```

Will appear in completions as "Custom Document Title" and insert `[Custom Document Title](key)` when selected. If the frontmatter field is missing, IWE falls back to using the first header.
