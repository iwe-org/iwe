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
--max-depth         N           default maxDepth applied to inclusion anchor flags without
                                a colon-suffix. Default 1.
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

Inclusion anchors (`--includes`, `--included-by`) accept `KEY[:DEPTH]` where DEPTH is a non-negative integer that becomes `maxDepth`. Reference anchors (`--references`, `--referenced-by`) use the same `KEY[:DIST]` syntax, lowered to `maxDistance`. **DEPTH / DIST `0` is the unbounded sentinel** — see below.

**Default values.** The CLI carries two session-level defaults, both starting at 1:

- `--max-depth N` — applied to inclusion anchor flags (`--includes`, `--included-by`) that omit a per-flag value.
- `--max-distance N` — applied to reference anchor flags (`--references`, `--referenced-by`) that omit a per-flag value.

`0` is the **unbounded sentinel** for both flags and the colon-suffix: passing `--max-depth 0`, `--max-distance 0`, or `KEY:0` lowers to the full form with `maxDepth` / `maxDistance` omitted (the language's "unbounded" form, `query-graph-spec.md` §5.3). This mirrors `limit: 0` in the language (`query-language-spec.md` §7). Positive integers behave as today.

A colon-suffix on a single anchor (`--includes KEY:5`) overrides the session default for that anchor only. The lowered shape depends on the effective depth:

- When the effective depth is 1 (the default, with no `--max-depth` / `--max-distance` and no colon-suffix, or an explicit `:1`), a bare `--includes KEY` lowers to **scalar shorthand** `$includes: KEY` — the language defines this as `{ match: { $key: KEY }, maxDepth: 1 }` (`query-graph-spec.md` §5.1).
- When the effective depth is `0` (unbounded sentinel), the lowering is the full form **without** a `maxDepth` key: `$includes: { match: { $key: KEY } }`.
- When the effective depth is any other positive integer N, a bare `--includes KEY` lowers to the **full form** `$includes: { match: { $key: KEY }, maxDepth: N }`. The session default appears explicitly in the lowered document; scalar shorthand is reserved for the depth-1 case.
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

Lowering examples with the `0` (unbounded) sentinel:

```
--includes roadmap/q2:0        →   $includes: { match: { $key: roadmap/q2 } }
                                   (full form, maxDepth omitted → unbounded)

--max-depth 0 --includes roadmap/q2
                               →   $includes: { match: { $key: roadmap/q2 } }
                                   (session default 0 → unbounded)

--max-depth 0 --includes roadmap/q2:3
                               →   $includes: { match: { $key: roadmap/q2 }, maxDepth: 3 }
                                   (per-flag colon wins over the session default)

--references people/alice:0    →   $references: { match: { $key: people/alice } }
                                   (full form, maxDistance omitted → unbounded)
```

For range bounds (`minDepth` / `maxDepth`, `minDistance` / `maxDistance`), anchoring by frontmatter predicate (`match: { status: draft }`), or any combination not expressible as a single keyed anchor, use `--filter` directly.

## 4. Shape flags

### 4.1 Format flags matrix

| Subcommand | `-f` / `--format` accepted values | Default |
|---|---|---|
| `iwe find` | `markdown`, `keys`, `json`, `yaml` | `markdown` |
| `iwe retrieve` | `markdown`, `keys`, `json`, `yaml` | `markdown` |
| `iwe tree` | `markdown`, `keys`, `json`, `yaml` | `markdown` |
| `iwe export` | `dot`, `markdown`, `keys`, `json`, `yaml` | `dot` |
| `iwe count` | (no format flag — output is always a single integer) | n/a |
| `iwe delete` | `markdown`, `keys` | `markdown` |
| `iwe rename`, `extract`, `inline` | `markdown`, `keys` | `markdown` |

Read-side commands (`find`, `retrieve`, `tree`, `export`) share one format set so a query written for one renders the same way under another. Mutation commands return a status report and only need `markdown` (human) or `keys` (machine) modes. `count`'s output is the integer match count and admits no format choice.

### 4.2 Other shape flags

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
| `--dry-run` | preview only; print the would-be changes per doc and exit |

`--set FIELD=VALUE` parses VALUE as a YAML scalar. `5` is an integer, `true` is a bool, `draft` is a string, `[a, b]` is a list. To force a string, quote it as YAML: `--set 'count="5"'`.

`iwe update` does not prompt for confirmation. The caller is responsible for passing the right `--filter` / `-k` selector; use `--dry-run` to inspect the matched set before applying. This matches the engine contract in `query-language-spec.md` §9.2 — the engine returns the patch and the host writes it without further interaction.

Body-overwrite mode (`-k KEY -c CONTENT`) is the existing single-doc body rewrite. It does not touch frontmatter and is not a spec `update` operation; the spec's `$content` body operator is deferred to v2 (`query-language-spec.md` §8.1). Body and mutation flags cannot be combined in one invocation. `--dry-run` applies to both modes.

## 6. Delete flags (`iwe delete`)

| Flag | Lowers to |
|---|---|
| Positional `KEY` | `$key: K` (sugar) |
| `--filter "EXPR"` | inline filter |
| `--dry-run` | preview |
| `-f, --format markdown\|keys` | output format (default `markdown`); `keys` prints affected document keys, suppresses progress |
| `--quiet` | suppress progress |

Either `KEY` or `--filter` (or both) must be present, matching the spec's required-filter rule (`query-language-spec.md` §3.2). When both are given, the union is deleted. `iwe delete` does not prompt; use `--dry-run` to preview before applying. Reference cleanup runs once over the whole matched set.

`-f keys` matches the same flag on `iwe find` / `iwe retrieve` / `iwe tree` / `iwe export`, but returns *affected* keys (the deleted target plus every doc whose references were rewritten) rather than *matched* keys. The same `-f markdown|keys` selector is also available on `iwe rename`, `iwe extract`, and `iwe inline` with identical semantics.

## 7. Deprecated aliases

These flags predate the language and remain accepted on the commands they originally appeared on. Selector aliases (`--in`, `--refs-to`, etc.) print a one-line `warning: --X is deprecated; use --Y` to stderr **each time the deprecated flag appears in a parsed command**. For one-shot CLI invocations this is one warning per run. For long-running hosts (LSP, MCP), the warning fires on every operation that uses the alias — making the deprecation visible across many requests instead of being suppressed after the first. Mutation-output aliases (`--keys`) are silent — `--keys` and `-f keys` behave identically and produce the same output.

| Deprecated | Lowers to |
|---|---|
| `--in KEY[:N]` | `--included-by KEY[:N]` |
| `--in-any K1 --in-any K2` | `$or: [{ $includedBy: K1 }, { $includedBy: K2 }]` (scalar shorthand for each) |
| `--not-in KEY` | `$not: { $includedBy: KEY }` (scalar shorthand) |
| `--refs-to KEY` | `$or: [{ $includes: KEY }, { $references: KEY }]` (scalar shorthand; legacy mixed-edge) |
| `--refs-from KEY` | `$or: [{ $includedBy: KEY }, { $referencedBy: KEY }]` (scalar shorthand; legacy mixed-edge) |
| `--keys` (on `delete`, `rename`, `extract`, `inline`) | `-f keys` |

The mixed-edge lowering of `--refs-to` / `--refs-from` preserves their pre-spec semantics (matching either inclusion or reference edges to the target). New code should pick the spec operators directly.

## 8. Composition rules

Within a single command:

1. All filter flags at the top level are AND-composed. The fuzzy positional `QUERY` (on `iwe find`) is also ANDed: the result is the **set intersection** of the fuzzy-match set and the filter-match set. Order of evaluation is implementation-defined (typically the more selective predicate is applied first for performance), but the result set is order-independent.
2. `--filter "EXPR"` contributes its top-level filter document to the same AND.
3. `-k` / positional `KEY` participate in the AND like any other clause.
4. `--sort`, `--limit`, `--project` apply after filtering, in the order defined by the spec (`query-language-spec.md` §10).

**`-k` / `$key` collision.** Combining `-k KEY` with a `--filter` whose top level contains a `$key` predicate is a CLI parse-time error: both clauses contribute to the document's key predicate, and silently AND-composing them would either produce a YAML mapping with two `$key` keys or quietly match the empty set. The error message points users at OR-composition (`--filter '$or: [{$key: a}, {$key: b}]'`) when they wanted a multi-key match, or at picking one source when they didn't. Multi-key match via `-k a -k b` (which lowers to `$key: { $in: [a, b] }`) remains valid.

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
iwe find --included-by projects/alpha:5             # descendants within 5 levels
iwe find --included-by projects/alpha:0             # all descendants of alpha (unbounded)
iwe find --references people/alice                  # docs that reference alice
iwe find --filter 'priority: { $gt: 3 }' --sort modified_at:-1
iwe find --project title,modified_at -f json        # only project two fields
iwe find --project title,modified_at -f yaml        # same, as YAML
```

### 9.2 Count

```
iwe count                                           # total documents
iwe count --filter 'status: draft'                  # count drafts
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

# Preview only — no writeback
iwe update --filter 'status: draft' --set status=published --dry-run
```

### 9.4 Delete

```
iwe delete document-key                             # single doc
iwe delete --filter 'status: archived'              # bulk delete by filter
iwe delete --filter '$key: drafts/scratch' --dry-run # preview a deletion by filter
```

## 10. Out of scope

- **Multi-key sort.** v1 of the language accepts exactly one sort key (`query-language-spec.md` §6); the CLI inherits the same constraint.
- **Compound update on body and frontmatter in one call.** Two passes (one body-overwrite, one mutation) are required.
- **Pattern matching on `$key`.** Glob / regex / prefix matching is reserved for a future revision (`query-graph-spec.md` §9).
- **Range bounds via flag.** `minDepth` / `minDistance`, mixed per-anchor `min`/`max` ranges, and anchoring by frontmatter predicate (`match: { ... }`) are only expressible via `--filter`. The flag set covers `maxDepth` / `maxDistance` (`--max-depth`, `--max-distance`, and the colon-suffix) but no lower bound.
