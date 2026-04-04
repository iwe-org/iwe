# IWE Inline

Replace an [Inclusion Links](inclusion-links.md) with the referenced document content.

## Usage

``` bash
iwe inline <KEY> [OPTIONS]
```

## Arguments

| Argument | Description                                     |
| -------- | ----------------------------------------------- |
| `<KEY>`  | Document key containing the reference to inline |


## Options

| Flag                | Description                                 |
| ------------------- | ------------------------------------------- |
| `--reference <KEY>` | Reference key or title to inline            |
| `--block <N>`       | Block number to inline (1-indexed)          |
| `--list`            | List all inclusion links with numbers       |
| `--action <NAME>`   | Action name from config to use for inlining |
| `--as-quote`        | Inline as blockquote instead of section     |
| `--keep-target`     | Keep the target document after inlining     |
| `--dry-run`         | Preview changes without writing to disk     |
| `--quiet`           | Suppress progress output                    |
| `--keys`            | Print affected document keys (one per line) |


## How It Works

The `inline` command embeds referenced content directly into the source document:

1.  **Select a reference** - Use `--list` to see available references, then select with `--reference` or `--block`
2.  **Embed content** - The referenced document's content replaces the inclusion link
3.  **Delete target** (optional) - By default, the referenced document is deleted
4.  **Clean up references** - Other references to the deleted document are updated

## Workflow

### Step 1: List Available References

``` bash
$ iwe inline my-document --list
1: [Introduction](introduction)
2: [Getting Started](getting-started)
3: [Configuration](config)
```

### Step 2: Inline by Reference or Block Number

``` bash
# By reference key/title (case-insensitive, partial match)
$ iwe inline my-document --reference "getting-started"
Inlining [Getting Started](getting-started) into 'my-document'
Done

# By block number (unambiguous)
$ iwe inline my-document --block 2
Inlining [Getting Started](getting-started) into 'my-document'
Done
```

## Inline Types

### Section (Default)

Embeds the full content of the referenced document:

Before:

``` markdown
# My Document

[Introduction](introduction)
```

After:

``` markdown
# My Document

## Introduction

Content from the introduction document...
```

### Quote (`--as-quote`)

Wraps the content in a blockquote:

Before:

``` markdown
# My Document

[Quote Source](quote-source)
```

After:

``` markdown
# My Document

> Content from the quote source document...
```

## Target Document Handling

### Default Behavior (Delete Target)

By default, the target document is deleted and other references are cleaned up:

``` bash
$ iwe inline index --reference "old-section"
Inlining [Old Section](old-section) into 'index'
Done
```

### Keep Target (`--keep-target`)

Preserve the target document after inlining:

``` bash
$ iwe inline index --reference "shared-content" --keep-target
Inlining [Shared Content](shared-content) into 'index'
Done
```

## Configuration

The `--action` flag uses inline settings from `.iwe/config.toml`:

``` toml
[actions]
inline_section = { type = "inline", title = "Inline Section", inline_type = "section", keep_target = false }
inline_quote = { type = "inline", title = "Inline as Quote", inline_type = "quote", keep_target = true }
```

## Output Modes

### Default Output

Shows progress:

``` bash
$ iwe inline my-document --reference "config"
Inlining [Configuration](config) into 'my-document'
Done
```

### Dry Run (`--dry-run`)

Preview what would happen:

``` bash
$ iwe inline my-document --reference "config" --dry-run
Would inline [Configuration](config) into 'my-document'
Would delete 'config'
Would update 2 additional document(s)
```

### Keys Output (`--keys`)

Print affected document keys:

``` bash
$ iwe inline my-document --block 1 --keys
my-document
introduction
other-referencing-doc
```

## Examples

``` bash
# List all inclusion links with numbers
iwe inline notes/index --list

# Inline by reference key
iwe inline notes/index --reference "architecture"

# Inline by block number (unambiguous)
iwe inline notes/index --block 1

# Preview changes without writing
iwe inline notes/index --reference "design" --dry-run

# Keep the target document after inlining
iwe inline notes/index --block 2 --keep-target

# Inline as blockquote instead of section
iwe inline notes/index --reference "notes" --as-quote

# Use a specific action from config
iwe inline notes/index --reference "design" --action "inline_quote"
```

## Use Cases

### Consolidating Documents

Merge related documents into a single comprehensive document:

``` bash
# See what references exist
iwe inline comprehensive-guide --list

# Inline each section
iwe inline comprehensive-guide --reference "intro"
iwe inline comprehensive-guide --reference "setup"
iwe inline comprehensive-guide --reference "usage"
```

### Embedding Quotes

Add cited content as blockquotes:

``` bash
iwe inline article --reference "source-material" --as-quote --keep-target
```

### Preview Impact

Check what would be affected before inlining:

``` bash
# See all affected documents
iwe inline index --reference "shared-doc" --dry-run

# Get keys for further analysis
iwe inline index --reference "shared-doc" --keys --dry-run
```

### Scripting

Use with other commands:

``` bash
# Inline all references in a document
iwe inline my-doc --list | while read line; do
  NUM=$(echo "$line" | cut -d: -f1)
  iwe inline my-doc --block "$NUM" --quiet
done
```

## Error Handling

The command fails with an error if:

- The document does not exist
- No reference matches the provided key/title (with `--reference`)
- Multiple references match (use `--block` instead)
- The block number is out of range (with `--block`)
- Must specify `--reference`, `--block`, or `--list`

## Technical Notes

- When inlining as a section, header levels are adjusted appropriately
- When deleting the target, all other references to it are cleaned up
- Inline references (not inclusion links) are converted to plain text
- The command uses the same inline logic as the LSP code action
