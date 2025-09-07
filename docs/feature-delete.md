# Delete Action

The delete action allows you to cleanly remove a referenced section and automatically update all files that reference it.

## How It Works

When you place your cursor on a block reference (like `[Important Topic](file)`) and trigger the delete action, IWE will:

1. **Delete the target file** - The referenced section/file is completely removed
2. **Clean up block references** - All block references to the deleted section are removed from other files
3. **Convert inline links** - Inline links to the deleted section are converted to plain text, preserving readability

## Usage

1. Position your cursor on any block reference in your markdown file
2. Open the code actions menu (typically `Ctrl+.` or `Cmd+.`)
3. Select "Delete" from the refactor actions

## Example

**Before deletion:**
```markdown
# My Notes

Some text with an inline link to [Important Topic](file).

[Important Topic](file)
```

**After deleting the reference on line with `[Important Topic](5)`:**
```markdown
# My Notes

Some text with an inline link to Important Topic.
```

The referenced file `Important Topic` is completely deleted, the block reference is removed, and the inline link becomes plain text.

## When Delete Action Is Available

- The delete action only appears when your cursor is on a **block reference**
- It will not appear on regular text, headers, or other content types
- The action ensures safe deletion by updating all referencing files automatically
