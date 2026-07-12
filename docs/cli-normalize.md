# IWE Normalize

Performs comprehensive document normalization across all markdown files in your knowledge base.

## Usage

``` bash
iwe normalize
```

## Operations Performed

| Operation                | Description                                                        |
| ------------------------ | ----------------------------------------------------------------- |
| Link title sync          | Rewrites link text to the target document's title (opt-in, off by default; enable with `refs_text = "normalize"`) |
| Link path rewriting      | Writes each link path per the `refs_path` setting                 |
| Header leveling          | Adjusts header levels for consistent hierarchy     |
| List renumbering         | Fixes ordered list numbering (1, 2, 3...)          |
| Whitespace normalization | Standardizes newlines and indentation              |
| List formatting          | Ensures consistent list item formatting            |
| Structure cleanup        | Removes redundant empty lines                      |


## Before/After Examples

### Link Title Sync

By default link text is preserved exactly as written. Set `refs_text = "normalize"` in `[markdown]` to rewrite each link's text to the linked document's title.

Before:

``` markdown
See the [old title](project-docs) for details.
```

After, with `refs_text = "normalize"` (and `project-docs.md` has header `# Project Documentation`):

``` markdown
See the [Project Documentation](project-docs) for details.
```

### Link Path Rewriting

Every markdown link is rewritten according to the `refs_path` setting (see below). With `refs_path = "absolute"`, a link written relative to the current document is rewritten as a root-absolute path from the library root.

Before (in `guide/intro.md`, with `refs_path = "absolute"`):

``` markdown
See the [API](../reference/api) for details.
```

After:

``` markdown
See the [API](/reference/api) for details.
```

Regardless of the setting, a link that already starts with `/` is resolved from the library root, and any `#section` fragment is preserved.

### List Renumbering

Before:

``` markdown
1. First item
1. Second item
5. Third item
```

After:

``` markdown
1. First item
2. Second item
3. Third item
```

### Whitespace Normalization

Before:

``` markdown
# Title


Some paragraph.



Another paragraph.
```

After:

``` markdown
# Title

Some paragraph.

Another paragraph.
```

## Examples

``` bash
# Basic normalization
iwe normalize

# With INFO level logging
iwe -v 1 normalize

# With DEBUG level logging
iwe -v 2 normalize
```

## Safety Warning

**Important:** The normalize command modifies files in place. Always ensure you have a backup or version control before running:

``` bash
# Recommended: commit changes before normalizing
git add -A && git commit -m "Before normalization"

# Run normalization
iwe normalize

# Review changes
git diff
```

## Configuration

Normalization behavior is controlled by `.iwe/config.toml`:

``` toml
[markdown]
refs_extension = ""       # Extension for reference links
refs_path = "relative"    # Link path form: "relative" or "absolute"
refs_text = "preserve"    # Link text: "preserve" or "normalize"
```

## Idempotency

Running `normalize` multiple times produces the same result - once files are normalized, subsequent runs make no changes.
