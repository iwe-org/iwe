# IWE Find

Search and discover documents in your knowledge base. Combines a fuzzy `QUERY` (matched against title and key) with a YAML-based filter language.

## Usage

``` bash
iwe find [QUERY] [OPTIONS]
```

## Options

| Flag                            | Description                                                                                  | Default    |
| ------------------------------- | -------------------------------------------------------------------------------------------- | ---------- |
| `[QUERY]`                       | Fuzzy search on document title and key                                                       | none       |
| `--filter <EXPR>`               | Inline YAML filter expression. See [Query Language](query-language.md).                      | none       |
| `-k, --key <KEY>`               | Match by document key. Repeatable: 1 key uses `$eq`, 2+ uses `$in`.                          | none       |
| `--includes <KEY[:DEPTH]>`      | `$includes` anchor. Repeatable; anchors are ANDed.                                           | none       |
| `--included-by <KEY[:DEPTH]>`   | `$includedBy` anchor. Repeatable; anchors are ANDed.                                         | none       |
| `--references <KEY[:DIST]>`     | `$references` anchor. Repeatable; anchors are ANDed.                                         | none       |
| `--referenced-by <KEY[:DIST]>`  | `$referencedBy` anchor. Repeatable; anchors are ANDed.                                       | none       |
| `--max-depth <N>`               | Session default for inclusion anchor flags without a colon-suffix. `0` = unbounded.          | 1          |
| `--max-distance <N>`            | Session default for reference anchor flags without a colon-suffix. `0` = unbounded.          | 1          |
| `--project <f1,f2,...>`         | Frontmatter fields to include in JSON / YAML output (comma-separated).                       | none       |
| `--sort <field:DIR>`            | Sort by frontmatter field. `DIR` is `1` (asc) or `-1` (desc).                                | none       |
| `-l, --limit <N>`               | Maximum number of results (`0` = unlimited).                                                 | unlimited  |
| `-f, --format <FMT>`            | Output format: `markdown`, `keys`, `json`, `yaml`.                                           | `markdown` |

All filter clauses (positional `QUERY` plus every flag above) are AND-composed at the top level. For OR or NOT, write it inside `--filter`. See [Query Language](query-language.md).

## Filter language

`--filter` accepts a YAML mapping. Bare scalars become equality predicates; `$`-prefixed keys are operators.

``` bash
# Bare equality on a frontmatter field
iwe find --filter 'status: draft'

# Multiple top-level keys are ANDed
iwe find --filter '{status: draft, priority: { $gt: 3 }}'

# OR composition
iwe find --filter '$or: [{ status: draft }, { status: review }]'

# Comparison and ordering
iwe find --filter 'priority: { $gte: 3, $lte: 7 }' --sort modified_at:-1

# Membership
iwe find --filter 'tags: rust'                                    # array membership
iwe find --filter 'status: { $in: [draft, review] }'              # any of these

# Presence
iwe find --filter 'reviewed_by: { $exists: false }'
```

See [Query Language](query-language.md) for the full operator vocabulary.

## Structural anchors

The four `$`-prefixed graph operators have CLI shortcuts. They walk inclusion edges (block-reference inclusion links) or reference edges (inline links), starting at the named anchor key.

``` bash
# Direct sub-documents only — scalar shorthand fixes maxDepth: 1
iwe find --included-by projects/alpha

# Walk inclusion edges down 5 levels
iwe find --included-by projects/alpha:5

# Every descendant (unbounded — depth `0` is the unbounded sentinel)
iwe find --included-by projects/alpha:0

# Anchors are ANDed: documents under both alpha and dmytro
iwe find --included-by projects/alpha --included-by people/dmytro

# Per-anchor depth: alpha within 3 levels, dmytro direct only
iwe find --included-by projects/alpha:3 --included-by people/dmytro:1

# Session default for un-suffixed anchors
iwe find --max-depth 5 --included-by projects/alpha --included-by research/q2

# Documents that reference a specific document (inline links)
iwe find --references people/alice

# Walk reference edges within 3 hops
iwe find --referenced-by archive/index:3

# OR composition of structural anchors via --filter
iwe find --filter '$or: [{ $includedBy: projects/alpha }, { $includedBy: projects/beta }]'
```

The same selector flags are accepted by [`iwe count`](cli-count.md), [`iwe retrieve`](cli-retrieve.md), [`iwe tree`](cli-tree.md), [`iwe export`](cli-export.md), [`iwe update`](cli-update.md), and [`iwe delete`](cli-delete.md).

## How it works

1. **Fuzzy matching** — `QUERY` is matched against both the key and the title using SkimMatcherV2.
2. **Filter** — `--filter` and the structural-anchor flags evaluate per document; results are intersected.
3. **Sort** — `--sort field:DIR` orders the matched set; ties are broken by document key.
4. **Limit** — applied last.
5. **Project** — for `json` / `yaml` output, `--project` selects which frontmatter fields appear in each result.

Without a query, results are sorted by incoming-reference popularity. With a query, they are sorted by fuzzy match score.

## Output formats

### Markdown (default)

``` markdown
## Documents

Found 3 results:

- [User Authentication](authentication) (root)
- [Login Flow](login-flow) <- [User Authentication](authentication)
- [Session Management](session-management) <- [User Authentication](authentication)
```

### Keys (`-f keys`)

```
authentication
login-flow
session-management
```

One key per line; suitable for piping.

### JSON (`-f json`)

``` json
{
  "query": "auth",
  "total": 3,
  "results": [
    {
      "key": "authentication",
      "title": "User Authentication",
      "is_root": true,
      "incoming_refs": 5,
      "outgoing_refs": 2,
      "parent_documents": []
    }
  ]
}
```

`--project title,modified_at` projects only those frontmatter fields into each result.

### YAML (`-f yaml`)

Same shape as JSON, rendered as YAML.

## Examples

``` bash
# All documents, default markdown
iwe find

# Fuzzy search
iwe find authentication

# Fuzzy search AND a frontmatter filter
iwe find auth --filter 'status: draft'

# Roots — documents with no incoming inclusion edges
iwe find --filter '$not: { $includedBy: { match: {} } }'

# Limit
iwe find --limit 10

# JSON for programmatic use, project two fields
iwe find --project title,modified_at -f json

# Pipe keys to retrieve
iwe find --filter 'status: draft' -f keys | xargs -I {} iwe retrieve -k {}
```

## Deprecated aliases

The following flags pre-date the query language and remain accepted for backward compatibility. Each invocation prints a one-line `warning: ... is deprecated` to stderr.

| Deprecated         | Use instead                                                                 |
| ------------------ | --------------------------------------------------------------------------- |
| `--in KEY[:N]`     | `--included-by KEY[:N]`                                                     |
| `--in-any K1 K2`   | `--filter '$or: [{ $includedBy: K1 }, { $includedBy: K2 }]'`                |
| `--not-in KEY`     | `--filter '$not: { $includedBy: KEY }'`                                     |
| `--refs-to KEY`    | `--references KEY` (legacy semantics: ORs `$includes` and `$references`)    |
| `--refs-from KEY`  | `--referenced-by KEY` (legacy semantics: ORs `$includedBy` and `$referencedBy`) |
| `--roots`          | `--filter '$not: { $includedBy: { match: {} } }'`                           |

## Technical notes

- All filter flags AND together at the top level. To compose with OR or NOT, wrap inside `--filter`.
- The colon-suffix on an anchor flag (`KEY:N`) overrides `--max-depth` / `--max-distance` for that anchor only. `0` is the unbounded sentinel.
- Combining `-k KEY` with a `--filter` whose top level also contains `$key` is a parse-time error. Use `-k a -k b` for multi-key match (lowers to `$key: { $in: [a, b] }`), or write the OR inside `--filter`.
- Both [Inclusion Links](inclusion-links.md) and inline references count toward `incoming_refs`.
