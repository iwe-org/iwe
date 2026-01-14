# Navigation

IWE provides several ways to navigate your documents using standard LSP features. This allows you to move through your knowledge graph efficiently using familiar editor commands.

## Links Navigation (Go To Definition)

Jump directly to linked documents using the LSP "Go To Definition" command.

### Usage

1. Place your cursor on a markdown link like `[Topic](topic-file)`
2. Trigger Go To Definition:
   - **VS Code**: `F12` or `Ctrl+Click` / `Cmd+Click`
   - **Neovim**: `gd` or `:lua vim.lsp.buf.definition()`
   - **Helix**: `gd`
3. The linked document opens automatically

### Example

```markdown
# My Notes

Check out [Project Ideas](project-ideas) for brainstorming.
```

With your cursor on "Project Ideas", triggering Go To Definition opens `project-ideas.md`.

## Table of Contents (Document Symbols)

View the structure of your current document using the LSP "Document Symbols" command.

### Usage

1. Open any markdown file
2. Trigger Document Symbols:
   - **VS Code**: `Ctrl+Shift+O` / `Cmd+Shift+O`
   - **Neovim**: `:lua vim.lsp.buf.document_symbol()` or use Telescope
   - **Helix**: `space` + `s`
3. Navigate through headers and sections

This provides an outline view of your document, showing all headers in a hierarchical tree. You can quickly jump to any section.

## Backlinks (Find References)

Find all documents that link to the current document using the LSP "Find References" command.

### Usage

1. Open a document you want to find references to
2. Trigger Find References:
   - **VS Code**: `Shift+F12` or right-click and select "Find All References"
   - **Neovim**: `:lua vim.lsp.buf.references()` or `gr`
   - **Helix**: `gr`
3. View all documents containing links to this file

### Example

If you're viewing `project-ideas.md` and trigger Find References, you'll see a list of all documents that contain links like `[Project Ideas](project-ideas)`.

This is essential for understanding how your notes are connected and for discovering relationships in your knowledge graph.

## Navigation Tips

- **Preview before jumping**: Use [Hover Preview](feature-hover-preview.md) to see linked content without leaving your current document
- **Use search for discovery**: Combine navigation with [Notes Search](feature-search.md) to find documents by content path
- **Track your trail**: Some editors maintain a navigation history, allowing you to go back to previous locations
