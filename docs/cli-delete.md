# IWE Delete

Delete one or many documents and clean up references to them across the knowledge base.

## Usage

``` bash
iwe delete <KEY> [OPTIONS]
iwe delete --filter "EXPR" [OPTIONS]
iwe delete <KEY> --filter "EXPR" [OPTIONS]
```

Either a positional `KEY` or `--filter` is required. When both are given, the union is deleted.

## Arguments

| Argument | Description                                          |
| -------- | ---------------------------------------------------- |
| `<KEY>`  | Document key to delete (sugar for `--filter '$key: K'`). |

## Options

| Flag                 | Description                                                                                  | Default    |
| -------------------- | -------------------------------------------------------------------------------------------- | ---------- |
| `--filter <EXPR>`    | Inline YAML filter expression. See [Query Language](query-language.md).                      | none       |
| `--dry-run`          | Preview changes without writing to disk.                                                     | false      |
| `--quiet`            | Suppress progress output.                                                                    | false      |
| `-f, --format <FMT>` | Output format: `markdown` or `keys`. `keys` prints affected keys (one per line).             | `markdown` |

`delete` does not prompt before writing. Use `--dry-run` to preview the matched set before applying.

## How it works

1. **Resolve the target set** — positional `KEY` and `--filter` matches are unioned, deduplicated, and sorted.
2. **Delete the document files** — every target is removed from the filesystem.
3. **Remove inclusion links** — [Inclusion Links](inclusion-links.md) pointing at any deleted document are dropped.
4. **Convert inline references** — inline links to deleted documents become plain text (link text is preserved).
5. **Maintain integrity** — the operation runs once across the whole matched set; no broken references remain after.

The reference cleanup step runs once over the union of targets, not per document. Documents that reference each other are handled correctly even when both are in the matched set.

## Reference cleanup

### Inclusion Links

Before:

``` markdown
# Index

[Overview](overview)

[Deleted Topic](deleted-topic)

[Other Topic](other-topic)
```

After deleting `deleted-topic`:

``` markdown
# Index

[Overview](overview)

[Other Topic](other-topic)
```

### Inline Links

Before:

``` markdown
For more details, see [Deleted Topic](deleted-topic) and [Other](other).
```

After deleting `deleted-topic`:

``` markdown
For more details, see Deleted Topic and [Other](other).
```

## Output modes

### Default (`-f markdown`)

Shows progress as documents are deleted:

``` bash
$ iwe delete my-document
Deleting 'my-document'
Updated 2 document(s)
```

### Dry run (`--dry-run`)

Preview without writing:

``` bash
$ iwe delete my-document --dry-run
Would delete 'my-document'
Would update 2 document(s)
  index
  overview
```

### Keys (`-f keys`)

Print affected document keys (the deleted target plus every doc whose references were rewritten):

``` bash
$ iwe delete my-document -f keys
my-document
index
overview
```

Suitable for scripting. `--dry-run` combines with `-f keys` to preview affected keys.

### Quiet (`--quiet`)

Suppress all non-error output:

``` bash
$ iwe delete my-document --quiet
```

## Examples

``` bash
# Single doc
iwe delete old-notes

# Preview first
iwe delete obsolete-doc --dry-run

# Bulk delete by filter
iwe delete --filter 'status: archived'

# Bulk delete with preview
iwe delete --filter 'status: archived' --dry-run

# Delete every descendant of a hub (unbounded)
iwe delete --filter '$includedBy: { match: { $key: archive/2024 } }'

# Affected keys for downstream processing
iwe delete my-doc -f keys --dry-run
```

## Use cases

### Cleaning up obsolete documents

``` bash
# Check what would be affected
iwe delete old-feature --dry-run

# Delete if satisfied
iwe delete old-feature
```

### Bulk archival cleanup

Use a filter to match many documents at once:

``` bash
# Archive everything under a hub, then verify
iwe delete --filter 'status: archived' --dry-run -f keys > affected.txt
iwe delete --filter 'status: archived' --quiet
```

### Pipeline integration

Pair with [`iwe find`](cli-find.md) for ad-hoc selections:

``` bash
iwe find temp -f keys | while read key; do
  iwe delete "$key" --quiet
done
```

For a single-pass equivalent, prefer `--filter`:

``` bash
iwe delete --filter '$key: { $in: [a, b, c] }'
```

## Deprecated aliases

| Deprecated | Use instead                  |
| ---------- | ---------------------------- |
| `--keys`   | `-f keys` (equivalent; `--keys` is still accepted silently) |

## Error handling

The command fails (exit code 1) when:

- Neither a positional `KEY` nor `--filter` is provided.
- The `--filter` expression cannot be parsed.
- A target document does not exist.
- Filesystem permissions prevent writing.

## Technical notes

- The operation is atomic over the full matched set — either every change succeeds or none are applied.
- Inclusion links to deleted documents are removed entirely; inline references are converted to plain text (link text preserved).
- `--dry-run` prints the would-be changes and exits without touching disk.

## Related

- [Query Language](query-language.md) — filter syntax.
- [`iwe find`](cli-find.md) — preview which documents a filter selects.
- [`iwe count`](cli-count.md) — count the matched set before deleting.
