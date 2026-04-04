# IWE Normalize

Performs comprehensive document normalization across all markdown files in your knowledge base.

## Usage

``` bash
iwe normalize
```

## Operations Performed

| Operation                | Description                                        |
| ------------------------ | -------------------------------------------------- |
| Link title sync          | Updates link text to match target document headers |
| Header leveling          | Adjusts header levels for consistent hierarchy     |
| List renumbering         | Fixes ordered list numbering (1, 2, 3...)          |
| Whitespace normalization | Standardizes newlines and indentation              |
| List formatting          | Ensures consistent list item formatting            |
| Structure cleanup        | Removes redundant empty lines                      |


## Before/After Examples

### Link Title Sync

Before:

``` markdown
See the [old title](project-docs) for details.
```

After (if `project-docs.md` has header `# Project Documentation`):

``` markdown
See the [Project Documentation](project-docs) for details.
```

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
refs_extension = ""  # Extension for reference links
```

## Idempotency

Running `normalize` multiple times produces the same result - once files are normalized, subsequent runs make no changes.
