# Text manipulation

IWE offers actions for context-aware transformations on your notes. These actions are available through your editor's code actions menu (usually `Ctrl+.` or `Cmd+.`).

## List to Sections

Convert a list into a series of headers/sections. Each list item becomes a section header with its nested content preserved.

### Example

**Before:**
```markdown
# Topics

- Project A
  - Details about project A
  - More info
- Project B
  - Details about project B
```

**After:**
```markdown
# Topics

## Project A

Details about project A

More info

## Project B

Details about project B
```

### When to Use

- Converting brainstorm lists into structured documents
- Expanding outline notes into full sections
- Promoting list items to top-level content

## Sections to List

Convert a series of headers back into a list. This is the reverse of "List to Sections".

### Example

**Before:**
```markdown
# Topics

## Project A

Details about project A

## Project B

Details about project B
```

**After:**
```markdown
# Topics

- Project A
  - Details about project A
- Project B
  - Details about project B
```

### When to Use

- Condensing detailed sections into an overview
- Creating summary lists from expanded content
- Reorganizing document structure

## Change List Type

Toggle between bullet lists and ordered (numbered) lists.

### Example

**Before (bullet list):**
```markdown
- First item
- Second item
- Third item
```

**After (ordered list):**
```markdown
1. First item
2. Second item
3. Third item
```

### When to Use

- Converting unordered lists to numbered steps
- Changing numbered lists back to bullet points
- Adjusting list style based on content type

## Sort List Items

Sort list items alphabetically or by other criteria.

### Example

**Before:**
```markdown
- Zebra
- Apple
- Mango
- Banana
```

**After:**
```markdown
- Apple
- Banana
- Mango
- Zebra
```

### When to Use

- Organizing alphabetical lists (glossaries, indexes)
- Sorting task lists
- Maintaining consistent ordering

## Usage

1. Place your cursor on the list or section you want to transform
2. Open the code actions menu:
   - **VS Code**: `Ctrl+.` / `Cmd+.`
   - **Neovim**: `:lua vim.lsp.buf.code_action()`
   - **Helix**: `space` + `a`
3. Select the desired transformation from the menu
