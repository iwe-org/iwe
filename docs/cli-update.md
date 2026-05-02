# IWE Update

Modify an existing document. `update` has two mutually-exclusive modes:

- **Body-overwrite** — replace the full markdown content of one document.
- **Frontmatter mutation** — apply `$set` / `$unset` to one or many documents matched by a filter.

Combining body and frontmatter flags in one invocation is a parse-time error.

## Usage

``` bash
# Body-overwrite mode — replace one document's content
iwe update -k <KEY> -c <CONTENT>
iwe update -k <KEY> -c -                                   # read content from stdin

# Frontmatter mutation mode — set / unset fields on matched documents
iwe update -k <KEY> --set FIELD=VALUE [--set ...] [--unset FIELD ...]
iwe update --filter "EXPR" --set FIELD=VALUE [--set ...] [--unset FIELD ...]
```

## Options

| Flag                       | Description                                                                                                  | Mode               |
| -------------------------- | ------------------------------------------------------------------------------------------------------------ | ------------------ |
| `-k, --key <KEY>`          | Document key. Required for body-overwrite. Optional in mutation mode (combined with `--filter` via AND).     | both               |
| `-c, --content <STR>`      | New full markdown content. Use `-` to read from stdin.                                                       | body-overwrite     |
| `--filter <EXPR>`          | Inline YAML filter. Required if `-k` omitted in mutation mode.                                               | mutation           |
| `--set <FIELD=VALUE>`      | `$set` assignment. `VALUE` is parsed as a YAML scalar. Repeatable.                                           | mutation           |
| `--unset <FIELD>`          | `$unset` field. Repeatable.                                                                                  | mutation           |
| `--dry-run`                | Preview changes without writing.                                                                             | both               |
| `--quiet`                  | Suppress progress output.                                                                                    | both               |

## Body-overwrite mode

`update -k KEY -c CONTENT` overwrites the document at `<KEY>` with the provided content. The graph is not mutated by `update` itself; the file change is picked up on the next read. Use [`iwe normalize`](cli-normalize.md) afterward if you want to canonicalize the output.

``` bash
# Replace a doc with a fixed string
iwe update -k notes/draft -c "# Draft\n\nNew content."

# Pipe content from another command
cat new-content.md | iwe update -k notes/draft -c -

# Preview without writing
iwe update -k notes/draft -c "..." --dry-run
```

This mode does not touch frontmatter. The body operator (`$content`) is reserved for a future revision of the query language.

## Frontmatter mutation mode

In mutation mode, `update` applies the `$set` and `$unset` operators of the [Query Language](query-language.md) to every document matched by the filter. The selector can be a single key (`-k`), a filter expression (`--filter`), or both (ANDed).

### `--set FIELD=VALUE`

`VALUE` is parsed as a YAML 1.2 scalar. Type inference follows YAML 1.2 rules: `5` is an integer, `true` is a boolean, `2026-04-26` is a date, `draft` is a string, `[a, b]` is a list. To force a string, quote it as YAML: `--set 'count="5"'`. Note that `yes`, `no`, `on`, and `off` are plain strings in YAML 1.2 (not booleans as in YAML 1.1). Frontmatter is re-serialized as YAML 1.2 after mutation, so quote styles may change on untouched fields.

### `--unset FIELD`

Removes `FIELD` from every matched document's frontmatter. Absent on a given document is a no-op.

### Examples

``` bash
# Single-doc frontmatter mutation
iwe update -k notes/draft --set status=published

# Bulk mutation across a filter
iwe update --filter 'status: draft' --set 'reviewed=true'

# Multiple set / unset in one call
iwe update --filter 'status: archived' --unset draft_notes --unset temporary

# Promote drafts to published, with a date
iwe update --filter 'status: draft' --set status=published --set 'published_at=2026-04-26'

# Preview only — no writeback
iwe update --filter 'status: draft' --set status=published --dry-run

# Combine -k and --filter (ANDed)
iwe update -k projects/alpha --filter 'status: draft' --set reviewed=true
```

`update` does not prompt before writing. Use `--dry-run` to inspect the matched set before applying.

### Reserved-prefix protection

Frontmatter field names whose first character is `_`, `$`, `.`, `#`, or `@` are reserved by the engine. Targeting a reserved-prefix segment in a `$set` or `$unset` path — at any depth — is a parse-time error. See `docs/specs/query-language-spec.md` §2.3 / §8.2 for the full rules.

### Conflict detection

`$set` and `$unset` paths are checked for prefix overlap; conflicts (e.g. `--set a.b=1 --unset a`) are parse-time errors. Mapping values in `$set` replace wholesale — use dotted shorthand (`--set 'author.name=alice'`) to write a leaf without dropping siblings.

## Composition order

Within one invocation, the operation document is built in this order:

1. **Filter** — narrows by `--filter` and `-k`.
2. **Sort** / **limit** — not exposed on the CLI for `update`; the engine processes the matched set in deterministic key order.
3. **Update** — `$set` / `$unset` operators applied atomically per document.

## Relationship to MCP

The `iwe_update` MCP tool exposes both modes. Pass `key` + `content` for body-overwrite, or `filter` / `key` + `set` / `unset` for frontmatter mutation. The dry-run flag is honored by both modes.

## Related

- [Query Language](query-language.md) — filter syntax and operator vocabulary.
- [`iwe find`](cli-find.md) — preview which documents a filter selects.
- [`iwe count`](cli-count.md) — count the matched set before mutating.
- [`iwe delete`](cli-delete.md) — remove the matched set instead of mutating it.
