# IWE Count

Count documents in your knowledge base that match a filter. Output is a single integer on stdout.

## Usage

``` bash
iwe count [OPTIONS]
```

## Options

| Flag                       | Description                                                                                                  | Default       |
| -------------------------- | ------------------------------------------------------------------------------------------------------------ | ------------- |
| `--filter <EXPR>`          | Inline YAML filter expression. See [Query Language](query-language.md).                                      | none          |
| `-k, --key <KEY>`          | Match by document key. Repeatable: 1 key uses `$eq`, 2+ uses `$in`.                                          | none          |
| `--includes <KEY[:DEPTH]>` | `$includes` anchor. Repeatable; anchors are ANDed. DEPTH defaults to `--max-depth`.                          | none          |
| `--included-by <KEY[:DEPTH]>` | `$includedBy` anchor. Repeatable; anchors are ANDed.                                                     | none          |
| `--references <KEY[:DIST]>` | `$references` anchor. Repeatable; anchors are ANDed. DIST defaults to `--max-distance`.                     | none          |
| `--referenced-by <KEY[:DIST]>` | `$referencedBy` anchor. Repeatable; anchors are ANDed.                                                  | none          |
| `--max-depth <N>`          | Session default for inclusion anchor flags without a colon-suffix. `0` = unbounded.                          | 1             |
| `--max-distance <N>`       | Session default for reference anchor flags without a colon-suffix. `0` = unbounded.                          | 1             |
| `--sort <field:DIR>`       | Sort by frontmatter field before applying `--limit`. `DIR` is `1` (asc) or `-1` (desc).                      | none          |
| `-l, --limit <N>`          | Cap the number of matches counted (`0` = unlimited).                                                         | none          |

`count` does not accept `-f / --format` or `--project`. Output is always a single integer terminated by a newline.

## How it works

`count` runs the same filter pipeline as [`iwe find`](cli-find.md) but returns just the integer count of matched documents. All filter flags AND together at the top level; combine with `--filter '$or: [...]'` for OR composition. The `KEY[:DEPTH]` / `KEY[:DIST]` colon-suffix on an anchor flag overrides the session default for that anchor only; `0` is the unbounded sentinel.

## Examples

``` bash
# Total documents
iwe count

# Drafts
iwe count --filter 'status: draft'

# Drafts with a high priority
iwe count --filter '{status: draft, priority: { $gt: 3 }}'

# Descendants of an anchor, within 10 levels
iwe count --included-by projects/alpha:10

# Every descendant of an anchor (unbounded)
iwe count --included-by projects/alpha:0

# Multi-key match
iwe count -k projects/alpha -k projects/beta

# OR-composition via --filter
iwe count --filter '$or: [{ status: draft }, { status: review }]'

# Cap the count for very large corpuses
iwe count --filter 'tags: rust' -l 1000
```

## Use cases

### Quick health checks

``` bash
# Are there still drafts in the queue?
iwe count --filter 'status: draft'

# Has the archive grown beyond a threshold?
iwe count --filter 'status: archived'
```

### Exit-code-friendly assertions in CI

``` bash
# Fail CI if any draft remains
test "$(iwe count --filter 'status: draft')" -eq 0
```

### Coverage of a structural set

``` bash
# How many documents are below this hub?
iwe count --included-by projects/alpha:0
```

## Related

- [Query Language](query-language.md) — the YAML filter syntax.
- [`iwe find`](cli-find.md) — same filter shape, returns the matched documents.
- [`iwe update`](cli-update.md) — apply mutations to the same matched set.
- [`iwe delete`](cli-delete.md) — remove the same matched set.
