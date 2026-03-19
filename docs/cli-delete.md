# IWE Delete

Delete a document and clean up all references to it across the knowledge base.

## Usage

```bash
iwe delete <KEY> [OPTIONS]
```

## Arguments

| Argument | Description |
|----------|-------------|
| `<KEY>` | Document key to delete |

## Options

| Flag | Description |
|------|-------------|
| `--dry-run` | Preview changes without writing to disk |
| `--quiet` | Suppress progress output |
| `--keys` | Print affected document keys (one per line) |
| `--force` | Skip confirmation prompt |

## How It Works

The `delete` command performs a safe document deletion with full reference cleanup:

1. **Deletes the document file** - Removes the document from the filesystem
2. **Removes [inclusion links](inclusion-links.md)** - Inclusion links pointing to the deleted document are removed
3. **Converts inline links** - Inline references are converted to plain text (preserving readability)
4. **Maintains integrity** - Ensures no broken references remain

## Reference Cleanup

### Inclusion Links

Before:
```markdown
# Index

[Overview](overview)

[Deleted Topic](deleted-topic)

[Other Topic](other-topic)
```

After deleting `deleted-topic`:
```markdown
# Index

[Overview](overview)

[Other Topic](other-topic)
```

### Inline Links

Before:
```markdown
For more details, see [Deleted Topic](deleted-topic) and [Other](other).
```

After deleting `deleted-topic`:
```markdown
For more details, see Deleted Topic and [Other](other).
```

## Output Modes

### Default Output

Shows confirmation prompt and progress:

```bash
$ iwe delete my-document
Delete 'my-document' and update 2 reference(s)? [y/N] y
Deleting 'my-document'
Updated 2 document(s)
```

### Dry Run (`--dry-run`)

Preview what would happen without making changes:

```bash
$ iwe delete my-document --dry-run
Would delete 'my-document'
Would update 2 document(s)
  index
  overview
```

### Keys Output (`--keys`)

Print affected document keys for scripting:

```bash
$ iwe delete my-document --keys
my-document
index
overview
```

### Force Mode (`--force`)

Skip the confirmation prompt:

```bash
$ iwe delete my-document --force
Deleting 'my-document'
Updated 2 document(s)
```

### Quiet Mode (`--quiet`)

Suppress all output except errors:

```bash
$ iwe delete my-document --quiet --force
```

## Examples

```bash
# Delete with confirmation
iwe delete old-notes

# Preview changes first
iwe delete obsolete-doc --dry-run

# Force delete without confirmation
iwe delete temp-note --force

# Get affected keys for processing
iwe delete my-doc --keys --dry-run

# Silent delete (for scripts)
iwe delete doc --quiet --force
```

## Use Cases

### Cleaning Up Obsolete Documents

Remove outdated documents while preserving references:

```bash
# Check what would be affected
iwe delete old-feature --dry-run

# Delete if satisfied
iwe delete old-feature
```

### Batch Deletion

Delete multiple documents based on a pattern:

```bash
# Find and delete all temp documents
iwe find temp -f keys | while read key; do
  iwe delete "$key" --force --quiet
done
```

### Safe Cleanup

Preview all changes before committing:

```bash
# See full impact
iwe delete important-doc --dry-run

# Check affected documents
iwe delete important-doc --keys --dry-run
```

## Error Handling

The command fails with an error if:
- The document does not exist
- There are filesystem permission issues

## Technical Notes

- Inclusion links are completely removed from documents
- Inline references are converted to plain text (the link text is preserved)
- The operation is atomic - either all changes succeed or none are applied
- Use `--dry-run` to preview changes before committing
