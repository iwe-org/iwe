# IWE Query CLI Spec

## 1. Overview

This document specifies the `iwe` CLI surface for the four query operations defined in `query-language-spec.md`: `find`, `count`, `update`, `delete`. It covers:

- The flag set that maps each spec operator to a CLI flag.
- The `--filter` inline expression form.
- The deprecation table for legacy CLI flags that predate the language.
- How each command lowers its flags into a spec operation document.

Companion specs:

- **Language semantics:** `query-language-spec.md`
- **Graph operators:** `query-graph-spec.md`
- **Grammar reference:** `query-language-grammar.md`

## 2. Subcommands

| Subcommand | Spec operation | Notes |
|---|---|---|
| `iwe find [QUERY]` | `find` | Combines fuzzy `QUERY` (positional, on title/key) with filter flags via AND. Supports `--project`, `--sort`, `--limit`. |
| `iwe count` | `count` | Prints integer matches to stdout. Supports `--sort`, `--limit`. |
| `iwe update` | `update` (mutation mode) | Two modes: body overwrite (`-k -c`) or frontmatter mutation (`--filter`/`-k` + `--set`/`--unset`). Modes are mutually exclusive. |
| `iwe delete [KEY]` | `delete` | Positional `KEY` is sugar for `$key: K`. Combine with `--filter` to widen. Either `KEY` or `--filter` is required. |
| `iwe tree`, `retrieve`, `export` | (selection only) | Reuse the same filter flag set to narrow what they operate on. They are not spec operations. |

## 3. Filter flags

Each flag mirrors the spec operator name (camelCase → kebab-case). All flags are AND-composed at the top level.

```
--filter "EXPR"             inline YAML; wrapped in `{}` if not already a mapping
-k, --key KEY               $key match. 1 key = $eq, 2+ = $in.
--includes        KEY[:DEPTH]  $includes anchor; DEPTH defaults to --max-depth (1)
--included-by     KEY[:DEPTH]  $includedBy anchor; DEPTH defaults to --max-depth (1)
--references      KEY[:DIST]   $references anchor; DIST defaults to --max-distance (1)
--referenced-by   KEY[:DIST]   $referencedBy anchor; DIST defaults to --max-distance (1)
--includes-count    N           $includesCount: N (direct edge equality)
--included-by-count N           $includedByCount: N
--max-depth         N           default maxDepth applied to inclusion anchor flags without
                                a colon-suffix (and to count predicates). Default 1.
--max-distance      N           default maxDistance applied to reference anchor flags without
                                a colon-suffix. Default 1.
```

### 3.1 `--filter` lowering

The argument to `--filter` is parsed as a YAML value. If the parsed value is a mapping, it is used directly as a filter document. Otherwise the engine wraps the input in `{` and `}` and re-parses. This lets users write either form:

```
--filter 'status: draft'                # block-style mapping (preferred)
--filter '{status: draft, priority: 5}' # flow-style mapping
--filter '$key: notes/foo'              # graph operator at top level
```

The resulting filter document is parsed by the same builder that handles full operation documents, so all errors defined in `query-language-spec.md` (mixed `$`/bare keys, double-`$not`, etc.) are surfaced verbatim.

### 3.2 Anchor depth syntax

Inclusion anchors (`--includes`, `--included-by`) accept `KEY[:DEPTH]` where DEPTH is a positive integer that becomes `maxDepth`. Reference anchors (`--references`, `--referenced-by`) use the same `KEY[:DIST]` syntax, lowered to `maxDistance`.

**Default values.** The CLI carries two session-level defaults, both starting at 1:

- `--max-depth N` — applied to inclusion anchor flags (`--includes`, `--included-by`) and count predicates that omit a per-flag value.
- `--max-distance N` — applied to reference anchor flags (`--references`, `--referenced-by`) that omit a per-flag value.

A colon-suffix on a single anchor (`--includes KEY:5`) overrides the session default for that anchor only. The lowered shape depends on the default:

- When the session default equals 1 (no `--max-depth` / `--max-distance` set), a bare `--includes KEY` lowers to **scalar shorthand** `$includes: KEY` — the language defines this as `{ match: { $key: KEY }, maxDepth: 1 }` (`query-graph-spec.md` §6.1).
- When the user passes `--max-depth N` with N ≠ 1, a bare `--includes KEY` lowers to the **full form** `$includes: { match: { $key: KEY }, maxDepth: N }`. The session default appears explicitly in the lowered document; scalar shorthand is reserved for the depth-1 case.
- A per-flag colon-suffix always wins over the session default.

Lowering examples without `--max-depth` / `--max-distance` (defaults at 1):

```
--includes roadmap/q2          →   $includes: roadmap/q2
                                   (scalar shorthand; expands to depth 1 by language rule)

--includes roadmap/q2:2        →   $includes: { match: { $key: roadmap/q2 }, maxDepth: 2 }

--included-by projects/alpha:5 →   $includedBy: { match: { $key: projects/alpha }, maxDepth: 5 }

--references people/alice      →   $references: people/alice
                                   (scalar shorthand; expands to distance 1 by language rule)

--referenced-by archive/index:2 → $referencedBy: { match: { $key: archive/index }, maxDistance: 2 }
```

Lowering examples with `--max-depth 3 --max-distance 2`:

```
--max-depth 3 --includes roadmap/q2     →   $includes: { match: { $key: roadmap/q2 }, maxDepth: 3 }

--max-depth 3 --includes roadmap/q2:1   →   $includes: { match: { $key: roadmap/q2 }, maxDepth: 1 }
                                            (per-flag colon wins over the session default)

--max-distance 2 --references people/alice
                                        →   $references: { match: { $key: people/alice }, maxDistance: 2 }
```

For range bounds (`minDepth` / `maxDepth`, `minDistance` / `maxDistance`), anchoring by frontmatter predicate (`match: { status: draft }`), or any combination not expressible as a single keyed anchor, use `--filter` directly.

### 3.3 Count predicates

`--includes-count N` and `--included-by-count N` accept a non-negative integer literal and lower to the direct-edge count shorthand:

```
--included-by-count 0   →   $includedByCount: 0
--includes-count    5   →   $includesCount:   5
```

The bare-integer shorthand expands to `{ count: N, maxDepth: 1 }` per `query-graph-spec.md` §5.1. When the user passes `--max-depth M`, the CLI lowers count flags to the full form `$includesCount: { count: N, maxDepth: M }` (the session default applies to count predicates as well). Comparison predicates (`{ $gte: 3 }`) require `--filter`.

## 4. Shape flags

| Flag | Lowers to | Operations |
|---|---|---|
| `--project f1,f2[,f3]` | `project: { f1: 1, f2: 1, f3: 1 }` | `find` only |
| `--sort field:1`, `--sort field:-1` | `sort: { field: 1 }` / `sort: { field: -1 }` | `find`, `count` |
| `-l, --limit N` | `limit: N` (0 = unlimited, matching spec §7) | `find`, `count` |

`--project` accepts a comma-separated list of dotted-path field names. Each entry maps to `field: 1` in the projection document.

`--sort` accepts exactly one `field:DIR` pair, matching the spec's "exactly one sort key in v1" rule (§6).

## 5. Update flags (`iwe update` mutation mode)

| Flag | Lowers to |
|---|---|
| `--set FIELD=VALUE` | `$set: { FIELD: VALUE }` (repeatable) |
| `--unset FIELD` | `$unset: { FIELD: "" }` (repeatable) |
| `--filter "EXPR"` | required if `-k` is omitted |

`--set FIELD=VALUE` parses VALUE as a YAML scalar. `5` is an integer, `true` is a bool, `draft` is a string, `[a, b]` is a list. To force a string, quote it as YAML: `--set 'count="5"'`.

Body-overwrite mode (`-k KEY -c CONTENT`) is the existing single-doc body rewrite. It does not touch frontmatter and is not a spec `update` operation; the spec's `$content` body operator is deferred to v2 (`query-language-spec.md` §8.1). Body and mutation flags cannot be combined in one invocation.

## 6. Delete flags (`iwe delete`)

| Flag | Lowers to |
|---|---|
| Positional `KEY` | `$key: K` (sugar) |
| `--filter "EXPR"` | inline filter |
| `--force` | skip confirmation |
| `--dry-run` | preview |
| `-f, --format markdown\|keys` | output format (default `markdown`); `keys` prints affected document keys, suppresses progress |
| `--quiet` | suppress progress |

Either `KEY` or `--filter` (or both) must be present, matching the spec's required-filter rule (`query-language-spec.md` §3.2). When both are given, the union is deleted. The confirmation prompt and reference cleanup apply once over the whole matched set.

`-f keys` matches the same flag on `iwe find` / `iwe retrieve` / `iwe tree` / `iwe export`, but returns *affected* keys (the deleted target plus every doc whose references were rewritten) rather than *matched* keys. The same `-f markdown|keys` selector is also available on `iwe rename`, `iwe extract`, and `iwe inline` with identical semantics.

## 7. Deprecated aliases

These flags predate the language and remain accepted on the commands they originally appeared on. Selector aliases (`--in`, `--refs-to`, etc.) print a one-line `warning: --X is deprecated; use --Y` to stderr the first time they appear in a process. Mutation-output aliases (`--keys`) are silent — `--keys` and `-f keys` behave identically and produce the same output.

| Deprecated | Lowers to |
|---|---|
| `--in KEY[:N]` | `--included-by KEY[:N]` |
| `--in-any K1 --in-any K2` | `$or: [{ $includedBy: K1 }, { $includedBy: K2 }]` (scalar shorthand for each) |
| `--not-in KEY[:N]` | `$not: { $includedBy: KEY[:N] }` |
| `--refs-to KEY` | `$or: [{ $includes: KEY }, { $references: KEY }]` (scalar shorthand; legacy mixed-edge) |
| `--refs-from KEY` | `$or: [{ $includedBy: KEY }, { $referencedBy: KEY }]` (scalar shorthand; legacy mixed-edge) |
| `--roots` | `$includedByCount: 0` |
| `--keys` (on `delete`, `rename`, `extract`, `inline`) | `-f keys` |

The mixed-edge lowering of `--refs-to` / `--refs-from` preserves their pre-spec semantics (matching either inclusion or reference edges to the target). New code should pick the spec operators directly.

## 8. Composition rules

Within a single command:

1. All filter flags at the top level are AND-composed.
2. `--filter "EXPR"` contributes its top-level filter document to the same AND.
3. `-k` / positional `KEY` participate in the AND like any other clause.
4. `--sort`, `--limit`, `--project` apply after filtering, in the order defined by the spec (`query-language-spec.md` §10).

For OR or NOT compositions, write the filter inside `--filter`:

```
--filter '$or: [{ status: draft }, { status: review }]'
--filter '$not: { status: archived }'
```

## 9. Examples

### 9.1 Find

```
iwe find rust                                       # fuzzy on "rust"
iwe find --filter 'status: draft'                   # all drafts
iwe find rust --filter 'status: draft'              # fuzzy AND status==draft
iwe find --included-by-count 0                      # roots
iwe find --included-by projects/alpha:5             # descendants within 5 levels
iwe find --references people/alice                  # docs that reference alice
iwe find --filter 'priority: { $gt: 3 }' --sort modified_at:-1
iwe find --project title,modified_at -f json        # only project two fields
iwe find --project title,modified_at -f yaml        # same, as YAML
```

### 9.2 Count

```
iwe count                                           # total documents
iwe count --filter 'status: draft'                  # count drafts
iwe count --included-by-count 0                     # count roots
iwe count --included-by projects/alpha:10           # count descendants of alpha
```

### 9.3 Update

```
# Body overwrite (existing behavior)
iwe update -k notes/draft -c "# New body"
cat new.md | iwe update -k notes/draft -c -

# Single-doc frontmatter mutation
iwe update -k notes/draft --set status=published

# Bulk frontmatter mutation
iwe update --filter 'status: draft' --set 'reviewed=true'
iwe update --filter 'status: archived' --unset draft_notes

# Preview
iwe update --filter 'status: draft' --set status=published --dry-run
```

### 9.4 Delete

```
iwe delete document-key                             # single doc (with prompt)
iwe delete document-key --force                     # single doc (no prompt)
iwe delete --filter 'status: archived'              # bulk delete by filter
iwe delete --filter '$includedByCount: 0' --dry-run # preview deleting roots
```

## 10. Out of scope

- **Multi-key sort.** v1 of the language accepts exactly one sort key (`query-language-spec.md` §6); the CLI inherits the same constraint.
- **Compound update on body and frontmatter in one call.** Two passes (one body-overwrite, one mutation) are required.
- **Pattern matching on `$key`.** Glob / regex / prefix matching is reserved for a future revision (`query-graph-spec.md` §10).
- **Range bounds via flag.** `minDepth` / `minDistance`, mixed per-anchor `min`/`max` ranges, and anchoring by frontmatter predicate (`match: { ... }`) are only expressible via `--filter`. The flag set covers `maxDepth` / `maxDistance` (`--max-depth`, `--max-distance`, and the colon-suffix) but no lower bound.
