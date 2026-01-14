# Text structure normalization / formatting

IWE provides auto-formatting through the LSP formatting feature. This typically triggers when you save your document (if your editor is configured for format-on-save) or manually via a formatting command.

## What Gets Normalized

### 1. Link Title Updates

Link titles are automatically updated to match the header of the linked document.

**Before:**
```markdown
See [old title](my-note) for details.
```

**After** (if `my-note.md` has header "# My Updated Note"):
```markdown
See [My Updated Note](my-note) for details.
```

### 2. Header Level Adjustment

Header levels are adjusted to maintain a proper tree structure. This ensures that nested sections have correct relative header levels.

**Before:**
```markdown
# Main Document

#### Incorrectly Deep Section

Some content.
```

**After:**
```markdown
# Main Document

## Incorrectly Deep Section

Some content.
```

### 3. Ordered List Numbering

Ordered lists are renumbered sequentially, regardless of the original numbers.

**Before:**
```markdown
1. First item
5. Second item
3. Third item
```

**After:**
```markdown
1. First item
2. Second item
3. Third item
```

### 4. List Indentation and Spacing

List structure is normalized with consistent indentation and proper newlines.

**Before:**
```markdown
- Item one
    - Badly indented
  - Inconsistent
-No space after marker
```

**After:**
```markdown
- Item one
  - Badly indented
  - Inconsistent
- No space after marker
```

### 5. Whitespace Cleanup

Excess blank lines and trailing whitespace are cleaned up.

## Usage

### Format on Save

Most editors can be configured to format on save:

- **VS Code**: Enable "Format On Save" in settings
- **Neovim**: Configure autocommand or use plugin like `conform.nvim`
- **Helix**: Set `auto-format = true` in `languages.toml`

### Manual Formatting

Trigger formatting manually:

- **VS Code**: `Shift+Alt+F` / `Shift+Option+F`
- **Neovim**: `:lua vim.lsp.buf.format()`
- **Helix**: `:format`

## Configuration

Formatting behavior can be configured in `.iwe/config.toml`:

```toml
[markdown]
normalize_headers = true  # Adjust header levels
normalize_lists = true    # Fix list formatting
```

## Tips

- **Enable format-on-save** for consistent formatting across your library
- **Use with version control** to easily review formatting changes
- If you notice unexpected formatting, check [Header levels normalization](feature-normalization.md) for detailed header adjustment rules
