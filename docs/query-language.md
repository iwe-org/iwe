# Query Language

> **Experimental.** The query language is under active development. Syntax, operators, defaults, and CLI flag names may change without warning. It is **not exposed as a public API** — there is no library entry point, no MCP tool that accepts a raw filter document, and no stable on-disk format. The only supported surface today is the CLI subcommands listed below; build automation against them at your own risk.

IWE has a YAML-based, MongoDB-style query language for selecting, shaping, and mutating documents in a workspace by their frontmatter and graph relationships. Today it is reachable only through the CLI subcommands `iwe find`, `iwe count`, `iwe update`, and `iwe delete` (plus the read-only selectors on `iwe retrieve`, `iwe tree`, and `iwe export`). The MCP server still exposes the legacy structural-selector parameters and does not accept query-language filter documents.

This page is a quick reference. The formal source of truth lives under `docs/specs/`:

- `docs/specs/query-language-spec.md` — operations, filter, projection, sort, limit, update operators
- `docs/specs/query-graph-spec.md` — `$key`, `$includes`, `$includedBy`, `$references`, `$referencedBy`
- `docs/specs/query-cli-spec.md` — flag set, lowering rules, deprecation table
- `docs/specs/query-language-grammar.md` — BNF grammar

## Operations

| Operation | CLI subcommand | What it does |
| --- | --- | --- |
| `find` | [`iwe find`](cli-find.md) | Returns matched documents (subject to projection). |
| `count` | [`iwe count`](cli-count.md) | Returns the integer count of matched documents. |
| `update` | [`iwe update`](cli-update.md) | Mutates frontmatter on each matched document. |
| `delete` | [`iwe delete`](cli-delete.md) | Removes each matched document and cleans up references. |

`update` and `delete` require an explicit filter — passing `{}` on purpose is the only way to operate on the whole corpus.

## Filter syntax

A filter document is YAML. A document matches when every top-level key matches; multiple top-level keys are AND-composed.

### Bare equality

```yaml
status: draft
```

Matches documents whose `status` field equals `draft`. For arrays, a bare scalar tests membership: `tags: rust` matches when `rust` is in the `tags` array. Cross-type comparisons are always false (no implicit coercion: `priority: "3"` does not match an integer field).

### Operator expressions

A mapping with `$`-prefixed keys is an operator expression:

```yaml
priority: { $gt: 3 }
priority: { $gte: 3, $lte: 7 }       # closed range [3, 7]
status:   { $in: [draft, review] }
status:   { $nin: [archived, deleted] }
reviewed: { $exists: true }
tags:     { $all: [rust, async] }
tags:     { $size: 0 }
```

Operators in one expression are ANDed together. User frontmatter fields cannot start with `$`, so an operator and a field name never collide.

### Logical composition

```yaml
$and:
  - status: draft
  - priority: { $gt: 3 }

$or:
  - status: draft
  - status: review

$not:
  status: archived

$nor:
  - status: archived
  - status: deleted
```

Top-level AND is implicit. Use explicit `$and` when you need the same field name on multiple sub-clauses (a YAML mapping cannot have duplicate keys).

### Nested fields

Nested fields can be addressed via nested mapping or dotted shorthand; both forms are equivalent:

```yaml
author.name: alice
author:
  name: alice
```

Field names that themselves contain a literal `.` are not addressable in v1 — the engine always splits paths on `.`.

## Graph operators

Graph operators live alongside frontmatter predicates inside the same filter. They walk inclusion edges (block-reference inclusion links) or reference edges (inline links).

### `$key` — identity

```yaml
$key: notes/foo                                # implicit $eq
$key: { $in: [a, b, c] }                       # any of these
$key: { $nin: [drafts/scratch, drafts/temp] }  # none of these
```

### Relational operators

| Operator | Reads as | Edge type | Walk parameters |
| --- | --- | --- | --- |
| `$includes` | this doc includes an anchor | inclusion | `maxDepth`, `minDepth` |
| `$includedBy` | this doc is included by an anchor | inclusion | `maxDepth`, `minDepth` |
| `$references` | this doc references an anchor | reference | `maxDistance`, `minDistance` |
| `$referencedBy` | this doc is referenced by an anchor | reference | `maxDistance`, `minDistance` |

Each takes either a scalar key (shorthand for direct edges) or a mapping with `match` and walk parameters:

```yaml
# Direct edges only — scalar shorthand fixes maxDepth: 1
$includedBy: projects/alpha

# Walk inclusion edges from a single anchor, bounded
$includedBy: { match: { $key: projects/alpha }, maxDepth: 5 }

# Anchor by frontmatter predicate (every active project)
$includedBy:
  match:
    type: project
    status: active
  maxDepth: 5

# Anchor set is two named documents
$includedBy:
  match:
    $key: { $in: [projects/alpha, projects/beta] }
  maxDepth: 5

# Range bounds — descendants 2 to 5 levels under archive/index
$referencedBy: { match: { $key: archive/index }, minDistance: 2, maxDistance: 3 }
```

In the full mapping form, **omitting `maxDepth` / `maxDistance` means unbounded** — the walk reaches every transitively-related document. Walks are BFS and de-duplicate via a visited set, so cycles terminate.

A relational operator never matches a document in its own anchor set. To include the anchor, OR it in:

```yaml
$or:
  - $key: projects/alpha
  - $includedBy: { match: { $key: projects/alpha }, maxDepth: 5 }
```

## Projection (`find` only)

Inclusion-only in v1: list the fields you want; everything else is omitted. `1`, `true`, and YAML null are all accepted as the include marker.

```yaml
project:
  title: 1
  modified_at: 1
  author.name: 1
```

## Sort and limit

```yaml
sort:  { modified_at: -1 }   # 1 = ascending, -1 = descending
limit: 100                   # 0 = no limit
```

v1 accepts exactly one sort key. Ties (and the no-sort case) are broken by document key in ascending lexicographic order.

## Update operators

```yaml
update:
  $set:
    reviewed: true
    audited_at: 2026-04-26
    "review.reviewer": alice
  $unset:
    draft_notes: ""
```

`$set` adds the field if absent, replaces it otherwise. Mapping values replace wholesale; use dotted shorthand to write subset leaves without dropping siblings. `$unset` removes fields; values are ignored.

### Reserved-prefix protection

Frontmatter field names whose first character is `_`, `$`, `.`, `#`, or `@` are reserved by the engine. They are invisible to filters, projections, and sort, and `update` strips them on writeback. Targeting a reserved-prefix segment in a `$set` or `$unset` path — at any depth — is a parse-time error.

## CLI lowering

On the CLI, structural anchor flags lower to graph operators. A `KEY[:DEPTH]` suffix sets `maxDepth` (or `maxDistance`) for that anchor; depth `0` is the unbounded sentinel.

| CLI flag | Lowers to |
| --- | --- |
| `-k KEY` | `$key: KEY` (1 key = `$eq`; 2+ = `$in`) |
| `--includes KEY` | `$includes: KEY` (scalar shorthand, depth 1) |
| `--included-by KEY:5` | `$includedBy: { match: { $key: KEY }, maxDepth: 5 }` |
| `--references KEY:0` | `$references: { match: { $key: KEY } }` (unbounded) |
| `--referenced-by KEY` | `$referencedBy: KEY` |
| `--max-depth N` | session default for `--includes` / `--included-by` (default 1) |
| `--max-distance N` | session default for `--references` / `--referenced-by` (default 1) |
| `--filter "EXPR"` | inline YAML filter document |
| `--project f1,f2` | `project: { f1: 1, f2: 1 }` (find only) |
| `--sort field:1` / `--sort field:-1` | `sort: { field: 1 / -1 }` |
| `-l, --limit N` | `limit: N` |
| `--set FIELD=VALUE` | `$set: { FIELD: VALUE }` (update only; repeatable) |
| `--unset FIELD` | `$unset: { FIELD: "" }` (update only; repeatable) |

All filter flags AND together. For OR or NOT, write the composition inside `--filter`:

```bash
iwe find --filter '$or: [{ status: draft }, { status: review }]'
iwe find --filter '$not: { status: archived }'
```

Combining `-k KEY` with a `--filter` whose top level also contains `$key` is a parse-time error — pick one source, or use `-k a -k b` for multi-key match.
