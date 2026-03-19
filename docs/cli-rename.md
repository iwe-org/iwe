# IWE Rename

Rename a document and update all references to it across the knowledge base.

## Usage

```bash
iwe rename <OLD_KEY> <NEW_KEY> [OPTIONS]
```

## Arguments

| Argument | Description |
|----------|-------------|
| `<OLD_KEY>` | Current document key |
| `<NEW_KEY>` | New document key |

## Options

| Flag | Description |
|------|-------------|
| `--dry-run` | Preview changes without writing to disk |
| `--quiet` | Suppress progress output |
| `--keys` | Print affected document keys (one per line) |

## How It Works

The `rename` command performs a safe document rename with full reference tracking:

1. **Renames the document file** - Moves the document from old key to new key
2. **Updates [inclusion links](inclusion-links.md)** - All inclusion links pointing to the old key are updated
3. **Updates inline links** - All inline references to the old key are updated
4. **Maintains integrity** - Ensures no broken references after renaming

## Output Modes

### Default Output

Shows progress and summary:

```bash
$ iwe rename old-document new-document
Renaming 'old-document' to 'new-document'
Updated 3 document(s)
```

### Dry Run (`--dry-run`)

Preview what would happen without making changes:

```bash
$ iwe rename old-document new-document --dry-run
Would rename 'old-document' to 'new-document'
Would update 3 document(s)
  index
  overview
  related-topic
```

### Keys Output (`--keys`)

Print affected document keys for scripting:

```bash
$ iwe rename old-document new-document --keys
old-document
new-document
index
overview
related-topic
```

### Quiet Mode (`--quiet`)

Suppress all output except errors:

```bash
$ iwe rename old-document new-document --quiet
```

## Examples

```bash
# Basic rename
iwe rename my-note renamed-note

# Preview changes first
iwe rename old-key new-key --dry-run

# Get affected keys for processing
iwe rename document new-document --keys

# Silent rename (for scripts)
iwe rename doc new-doc --quiet
```

## Use Cases

### Reorganizing Knowledge Base

Rename documents to follow a new naming convention:

```bash
# Check what would be affected
iwe rename user-auth authentication --dry-run

# Perform the rename
iwe rename user-auth authentication
```

### Moving to Subdirectories

Move a document into a subdirectory:

```bash
# Move from root to subdirectory
iwe rename config settings/config
```

### Scripting

Use with other commands for batch operations:

```bash
# Get list of affected documents for further processing
iwe rename old-api api-v2 --keys | while read key; do
  echo "Affected: $key"
done
```

## Error Handling

The command fails with an error if:
- The source document does not exist
- The target key already exists
- There are filesystem permission issues

## Technical Notes

- References include both inclusion links and inline links
- The operation is atomic - either all changes succeed or none are applied
- Directory structure is preserved when renaming with path components
