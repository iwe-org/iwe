# Files Renaming

IWE provides file renaming through the LSP `rename` refactoring feature. When you rename a note file, IWE automatically updates all references throughout your entire library.

## How It Works

When you trigger a rename operation on a markdown file:

1. **Rename the file** - The file is renamed to your specified name
2. **Update all references** - Every link pointing to the old filename is updated to use the new name
3. **Preserve link titles** - Link display text remains unchanged

This ensures your knowledge graph stays consistent without manual search-and-replace operations.

## Usage

### In Your Editor

1. Open a markdown file you want to rename
2. Trigger the LSP rename command:
   - **VS Code**: `F2` or right-click and select "Rename Symbol"
   - **Neovim**: `:lua vim.lsp.buf.rename()` or your configured keybinding
   - **Helix**: `space` + `r`
3. Enter the new filename
4. Confirm the rename

### Example

**Before renaming:**

`old-topic.md`:
```markdown
# Old Topic

Some content here.
```

`index.md`:
```markdown
# Index

See [Old Topic](old-topic) for details.
Check also [Old Topic](old-topic) in another context.
```

**After renaming to `new-topic.md`:**

`new-topic.md`:
```markdown
# Old Topic

Some content here.
```

`index.md`:
```markdown
# Index

See [Old Topic](new-topic) for details.
Check also [Old Topic](new-topic) in another context.
```

Note that the link text ("Old Topic") is preserved while the link target is updated.

## Benefits

- **Safe refactoring** - No broken links after renaming
- **Bulk updates** - All references updated in a single operation
- **Undo support** - Most editors support undoing the rename operation
