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
--includes        KEY[:DEPTH]  $includes anchor (DEPTH defaults to 1)
--included-by     KEY[:DEPTH]  $includedBy anchor
--references      KEY[:DIST]   $references anchor
--referenced-by   KEY[:DIST]   $referencedBy anchor
--includes-count    N           $includesCount: N (direct edge equality)
--included-by-count N           $includedByCount: N
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

Inclusion anchors (`--includes`, `--included-by`) accept `KEY[:DEPTH]` where DEPTH is a positive integer that becomes `$maxDepth`. Reference anchors (`--references`, `--referenced-by`) use the same `KEY[:DIST]` syntax, lowered to `$maxDistance`. Bare `KEY` (no colon) defaults to depth/distance 1, supplying the bound modifier the spec §6.3 requires.

For ranges (`$minDepth`/`$maxDepth`), `$minDistance`/`$maxDistance`, the unbounded count flag (`$maxDepth: -1`), and any combinations not expressible as a single anchor, use `--filter` directly.

### 3.3 Count predicates

`--includes-count N` and `--included-by-count N` accept a non-negative integer literal and lower to a direct-edge count equality:

```
--included-by-count 0   →   $includedByCount: 0
--includes-count    5   →   $includesCount:   5
```

Comparison predicates (`{ $gte: 3 }`), depth-bounded counts, and unbounded counts (`$maxDepth: -1`) require `--filter`.

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
| `--in-any K1 --in-any K2` | `$or: [{ $includedBy: K1, ... }, { $includedBy: K2, ... }]` |
| `--not-in KEY[:N]` | `$not: { $includedBy: KEY:N }` |
| `--refs-to KEY` | `$or: [{ $includes: KEY:1 }, { $references: KEY:1 }]` (legacy mixed-edge) |
| `--refs-from KEY` | `$or: [{ $includedBy: KEY:1 }, { $referencedBy: KEY:1 }]` (legacy mixed-edge) |
| `--roots` | `$includedByCount: 0` |
| `--max-depth N` | global default for legacy anchors that omit `:N` |
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
iwe find --references people/dmytro                 # docs that reference dmytro
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
- **Range bounds via flag.** `$minDepth`/`$minDistance` and unbounded count (`$maxDepth: -1`) are only expressible via `--filter`.
