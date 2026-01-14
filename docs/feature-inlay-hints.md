# Inlay hints

Inlay hints are visual annotations that appear inline within your document, providing contextual information without requiring navigation. IWE provides two types of inlay hints.

## Note Header Hints

At the top of each document, IWE displays information about the note's position in your knowledge graph.

### What's Shown

- **Parent document title**: Shows which document links to this note
- **Links counter**: Number of incoming references (backlinks)

### Example

When viewing `project-ideas.md`:

```
                                    [linked from: Index] [3 links]
# Project Ideas

Your content here...
```

This tells you that "Index" links to this document and there are 3 total backlinks.

## Block Reference Hints

When viewing a block reference link, inlay hints show which notes directly reference the linked content.

### What's Shown

- **Parent notes list**: The direct parent notes (one level up) that link to the referenced content

### Example

```markdown
# Daily Notes

[Meeting Notes](meeting-2024)           [Index, Journal]
```

The hint `[Index, Journal]` shows that the linked note is referenced from both "Index" and "Journal" documents.

## Enabling Inlay Hints

### VS Code

Inlay hints should work automatically. If not visible, check:

1. Open Settings (`Ctrl+,` / `Cmd+,`)
2. Search for "inlay hints"
3. Enable "Editor: Inlay Hints"

### Neovim

Enable inlay hints in your LSP configuration:

```lua
vim.lsp.inlay_hint.enable(true)
```

Or toggle with a keybinding:

```lua
vim.keymap.set('n', '<leader>ih', function()
  vim.lsp.inlay_hint.enable(not vim.lsp.inlay_hint.is_enabled())
end)
```

### Helix

Inlay hints are enabled by default. Configure in `config.toml`:

```toml
[editor.lsp]
display-inlay-hints = true
```

## Benefits

- **Context at a glance**: Understand note relationships without navigating away
- **Track connections**: See how many documents link to the current note
- **Navigate hierarchy**: Understand where content fits in your knowledge structure
