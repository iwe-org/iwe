# IWE Update

Modify existing documents. `update` has two mutually-exclusive modes:

- **Body-overwrite** — replace the full markdown content of one document.
- **Mutation** — apply frontmatter operators (`$set` / `$unset`) and block operators (`$replace`, `$replaceText`, `$insertBefore`, `$insertAfter`, `$append`, `$delete`) to one or many documents matched by a filter.

Combining body flags with mutation flags in one invocation is a parse-time error.

## Usage

``` bash
# Body-overwrite mode — replace one document's content
iwe update -k <KEY> -c <CONTENT>
iwe update -k <KEY> -c -                                   # read content from stdin

# Mutation mode — frontmatter and block edits on matched documents
iwe update -k <KEY> --set FIELD=VALUE [--set ...] [--unset FIELD ...]
iwe update --filter "EXPR" --set FIELD=VALUE [--set ...] [--unset FIELD ...]
iwe update --filter "EXPR" --replace-text "{ <selector>, to: ... }"
```

## Options

| Flag                       | Description                                                                                                  | Mode               |
| -------------------------- | ------------------------------------------------------------------------------------------------------------ | ------------------ |
| `-k, --key <KEY>`          | Document key. Required for body-overwrite. Optional in mutation mode (combined with `--filter` via AND).     | both               |
| `-c, --content <STR>`      | New full markdown content. Use `-` to read from stdin.                                                       | body-overwrite     |
| `--filter <EXPR>`          | Inline YAML filter. Required if `-k` omitted in mutation mode.                                               | mutation           |
| `--set <FIELD=VALUE>`      | `$set` assignment. `VALUE` is parsed as a YAML scalar. Repeatable.                                           | mutation           |
| `--unset <FIELD>`          | `$unset` field. Repeatable.                                                                                  | mutation           |
| `--replace <ARG>`          | `$replace`: replace each selected block. `ARG` is `{ <selector>, content: <markdown> }`.                     | mutation           |
| `--replace-text <ARG>`     | `$replaceText`: rewrite own text of each selected block. `ARG` is `{ <selector>, from: X, to: Y }`; omit `from` and `to` replaces the entire own text. | mutation |
| `--insert-before <ARG>`    | `$insertBefore`: insert sibling content before each selected block. `ARG` is `{ <selector>, content: <markdown> }`. | mutation      |
| `--insert-after <ARG>`     | `$insertAfter`: insert sibling content after each selected block. `ARG` is `{ <selector>, content: <markdown> }`.   | mutation      |
| `--append <ARG>`           | `$append`: append child content to each selected container. `ARG` is `{ <selector>, content: <markdown> }`.  | mutation           |
| `--delete <ARG>`           | `$delete`: remove each selected block. `ARG` is the `{ <selector> }` mapping (`{}` selects every block).     | mutation           |
| `--expect <ARG>`           | Document-level guard: assert the number of matched documents. `ARG` is `N` or `{ min: M, max: N }`.          | mutation           |
| `--strict`                 | Require an `expect` guard on every mutating application (document-level `--expect` and each block operator's `expect`). Aborts before writing if any is missing. Exempt under `--dry-run`. | mutation |
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

This mode does not touch frontmatter. It is also the escape hatch when block predicates cannot address a target — two byte-identical sibling blocks, for instance, are indistinguishable to any predicate.

## Mutation mode

In mutation mode, `update` builds one `update` operation document of the [Query Language](query-language.md) and applies it to every document matched by the filter. The selector can be a single key (`-k`), a filter expression (`--filter`), or both (ANDed). All operators present in one invocation — frontmatter and block — validate and apply atomically: per document, every edit commits as one rewrite, and any validation failure anywhere aborts the whole operation before anything is written.

`update` does not prompt before writing. Use `--dry-run` to inspect the matched set before applying.

### `--set FIELD=VALUE`

`VALUE` is parsed as a YAML 1.2 scalar. Type inference follows YAML 1.2 rules: `5` is an integer, `true` is a boolean, `2026-04-26` is a date, `draft` is a string, `[a, b]` is a list. To force a string, quote it as YAML: `--set 'count="5"'`. Note that `yes`, `no`, `on`, and `off` are plain strings in YAML 1.2 (not booleans as in YAML 1.1). Frontmatter is re-serialized as YAML 1.2 after mutation, so quote styles may change on untouched fields.

### `--unset FIELD`

Removes `FIELD` from every matched document's frontmatter. Absent on a given document is a no-op.

### Frontmatter examples

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

### Reserved-prefix protection

Frontmatter field names whose first character is `_`, `$`, `.`, `#`, or `@` are reserved by the engine. Targeting a reserved-prefix segment in a `$set` or `$unset` path — at any depth — is a parse-time error. See `docs/spec.md` §2.3 / §9.2 for the full rules.

### Conflict detection

`$set` and `$unset` paths are checked for prefix overlap; conflicts (e.g. `--set a.b=1 --unset a`) are parse-time errors. Mapping values in `$set` replace wholesale — use dotted shorthand (`--set 'author.name=alice'`) to write a leaf without dropping siblings.

## Block operators

Each block operator has its own flag whose argument is that operator's `{ <selector>, <payload> }` mapping: the `$`-prefixed keys are a [block predicate](query-language.md#block-predicates) selecting the blocks, the bare keys carry the payload (`content`, `from`, `to`) and the optional `expect` guard. The operator name is the flag, so the `$replaceText:` wrapper is dropped. Full semantics — targets and coalescing, header retitle/dissolve behavior, `$replaceText` anchor rules, disjointness — are in [Block update operators](query-language.md#block-update-operators).

``` bash
# Retitle a section: the heading line changes, contents stay
iwe update -k projects/roadmap --replace '{ $header: Goals, content: "## Goals 2026", expect: 1 }'

# Remove a section wholesale: header and everything below it
iwe update -k projects/roadmap --delete '{ $section: Goals, expect: 1 }'

# Dissolve a header: the heading line goes, its contents merge into the enclosing section
iwe update -k projects/roadmap --delete '{ $header: Goals, expect: 1 }'

# Precision text edit, guarded to exactly one block
iwe update --filter '$content: { $text: "Q3 Milestones" }' \
           --replace-text '{ $within: Goals, $text: "Q3 Milestones", from: "Q3 Milestones", to: "Q3 2026 Milestones", expect: 1 }'

# Rename a header (from omitted: to replaces the entire own text)
iwe update -k projects/roadmap --replace-text '{ $header: Goals, to: Aims, expect: 1 }'

# Bulk maintenance: delete every paragraph referencing a retired doc
iwe update --filter '{}' --delete '{ $paragraph: { $references: archive/old-plan } }'

# Bounded cleanup: refuse a runaway match
iwe update --filter 'type: meeting' --delete '{ $paragraph: { $matches: "^DONE " }, expect: { max: 20 } }'

# Clear a document's body, keep its frontmatter
iwe update --filter '$key: notes/scratch' --delete '{}'
```

The block-operator flags combine with each other and with `--set` / `--unset`; every present flag becomes one operator entry in a single `update` document. Each operator flag appears at most once per invocation. Frontmatter operators apply to every filter-matched document regardless of block selections — to restrict a combined edit to documents where the block edit lands, conjoin `$content` into the filter with the same predicate:

``` bash
# Stamp frontmatter and append a status line under every active
# project's Status section, in one atomic operation
iwe update --filter '{ type: project, status: active, $content: { $header: Status } }' \
           --set reviewed=true \
           --append '{ $header: Status, content: "Reviewed 2026-07-06." }'
```

Use [`iwe find --blocks`](cli-find.md#block-projection) with the same predicate to locate the targets (and learn the counts) before mutating.

## Guards and strict mode

`--expect` asserts the number of matched documents the operation will write; each block operator's `expect` key asserts the number of targets it will act on (selected blocks for `--replace-text`). The two are independent quantities. Any violated guard aborts the whole operation before anything is written, listing the actual targets. See [`expect` guards](query-language.md#expect-guards).

Under `--strict`, every mutating application must carry its guard — a missing one is an error before anything runs. `--dry-run` is exempt: it is how the counts are learned.

``` bash
# Learn the counts, then pin them
iwe update --filter 'status: draft' --set status=published --strict --dry-run
iwe update --filter 'status: draft' --set status=published --strict --expect 3
```

## Composition order

Within one invocation, the operation document is built in this order:

1. **Filter** — narrows by `--filter` and `-k`.
2. **Sort** / **limit** — not exposed on the CLI for `update`; the engine processes the matched set in deterministic key order.
3. **Update** — frontmatter and block operators validated together, then applied atomically per document.

## Relationship to MCP

The `iwe_update` MCP tool exposes body-overwrite. The full mutation surface — frontmatter and block operators — is the `iwe_query` tool, which accepts an `update` operation document verbatim and is always strict: every mutating application must carry its `expect` guard. See [MCP Server](mcp.md).

## Related

- [Query Language](query-language.md) — filter syntax, block predicates, and update operator semantics.
- [`iwe find`](cli-find.md) — preview which documents a filter selects; locate blocks with `--blocks`.
- [`iwe count`](cli-count.md) — count the matched set before mutating.
- [`iwe delete`](cli-delete.md) — remove the matched set instead of mutating it.
