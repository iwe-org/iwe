# IWE Query Graph Operators Spec

## 1. Overview

This document specifies a focused subset of the IWE query language graph operators: an identity predicate, two count predicates over the inclusion graph, and four relational walk operators over inclusion and reference edges.

Graph operators live inside filter documents alongside frontmatter predicates. They share the predicate algebra of filter: AND-composed at top level, composable under `$and` / `$or` / `$not`, with the same operator-expression vocabulary as numeric frontmatter fields. Selection by graph relationship and selection by frontmatter content are written in the same filter document, distinguished only by whether the predicate key is a `$`-prefixed graph operator or a user frontmatter field name. The reserved-prefix rule (`query-language-spec.md` §2.3) makes this safe: user frontmatter fields cannot begin with `$`.

The reader who needs the full picture should read `query-language-spec.md` for the operation document, the filter language, and the predicate algebra these operators extend.

## 2. Edge model

IWE's corpus graph contains two kinds of directed edges between documents:

- **Inclusion edges** — structural transclusion links. When document A includes document B, B's content is rendered inline as part of A. Inclusion edges form a directed acyclic graph (cycles are normalized away at parse time by liwe's graph builder).
- **Reference edges** — non-structural links, including inline mentions inside text. A document can reference another without including it.

Both edge kinds are directed. Direction-of-read convention for all four relational operators: the operator predicates the relationship from the perspective of the document being filtered.

| Operator | Reads as | This doc → anchor? | Anchor → this doc? |
|---|---|---|---|
| `$includes` | this doc includes an anchor | yes (outbound inclusion) | no |
| `$includedBy` | this doc is included by an anchor | no | yes (inbound inclusion) |
| `$references` | this doc references an anchor | yes (outbound reference) | no |
| `$referencedBy` | this doc is referenced by an anchor | no | yes (inbound reference) |

The "anchor" is one of the documents selected by the operator's argument. Relational operators take a `match` filter that resolves to an anchor set (§6); a relational predicate matches when this document stands in the named relation to at least one document in that set.

## 3. Operator inventory

| Category | Operator | Predicate over... |
|---|---|---|
| Identity (§4) | `$key` | the document's own key |
| Count (§5) | `$includesCount` | count of documents reachable via outbound inclusion edges |
| Count (§5) | `$includedByCount` | count of documents reachable via inbound inclusion edges |
| Relational (§6) | `$includes` | the document's outbound inclusion relation to an anchor set |
| Relational (§6) | `$includedBy` | the document's inbound inclusion relation to an anchor set |
| Relational (§6) | `$references` | the document's outbound reference relation to an anchor set |
| Relational (§6) | `$referencedBy` | the document's inbound reference relation to an anchor set |

The vocabulary is closed in v1. Unknown `$`-prefixed operator names inside a filter are parse-time errors.

Naming conventions: all operator names are camelCase, `$`-prefixed. The `$`-prefix is reserved for operators that evaluate; **walk parameters and payload field names inside operator arguments are bare-named** (`match`, `count`, `maxDepth`, `minDepth`, `maxDistance`, `minDistance`). They are configuration of the operator's walk, not operators in their own right.

## 4. Identity operator

`$key` predicates the document's own key.

### 4.1 Argument shape

`$key` accepts either a scalar key (implicit `$eq`) or an operator expression.

```
key_op ::= key | key_expr

key_expr ::=
    { $eq:  key }
  | { $ne:  key }
  | { $in:  [key, key, ...] }    # non-empty array
  | { $nin: [key, key, ...] }    # non-empty array
```

### 4.2 Examples

```yaml
filter:
  $key: notes/foo                              # implicit $eq
  $key: { $eq: notes/foo }                     # explicit
  $key: { $ne: drafts/scratch }                # exclude one
  $key: { $in: [a, b, c] }                     # any of these
  $key: { $nin: [drafts/a, drafts/b] }         # none of these
```

### 4.3 Constraints

- `$key` accepts strings only. Operator expressions on `$key` use the comparison set above; `$gt` / `$gte` / `$lt` / `$lte` are parse-time errors (keys are identifiers, not ordered values).
- Empty `$in: []` and `$nin: []` are parse-time errors.
- Pattern matching on `$key` (glob, regex, prefix) is reserved for a future revision.

`$key` has only one role in the language: a top-level filter predicate over this document's own key. It also appears inside the `match` filter of relational operators (§6.2), but only because `match` is itself a filter document — there it carries the same semantics as any other filter-level `$key` predicate.

## 5. Count operators

The count operators predicate over the number of documents reachable from the document being filtered along inclusion edges, within configurable depth bounds.

| Operator | Counts documents... |
|---|---|
| `$includesCount` | reachable via outbound inclusion edges (descendants in the inclusion DAG) |
| `$includedByCount` | reachable via inbound inclusion edges (ancestors in the inclusion DAG) |

Both operators take either a numeric predicate (sugar for "direct count") or a structured argument with a numeric expression on the bare `count` field plus optional walk parameters. The walk parameters determine which documents are included in the count; the numeric expression filters the count value.

### 5.1 Argument shape

```
count_arg ::= int | num_expr | count_obj

count_obj ::= {
    count:    int | num_expr       (required)
    maxDepth: pos_int              (optional; absent = unbounded)
    minDepth: pos_int              (optional; absent = 1)
}
```

Field names inside `count_obj` are bare-named — `$`-prefix is reserved for operators that evaluate, not configuration.

Bare integer form (sugar for direct count equality):

```yaml
$includesCount: 0
```

is equivalent to

```yaml
$includesCount: { count: 0, maxDepth: 1 }
```

Bare expression form (sugar for direct count predicate):

```yaml
$includesCount: { $gte: 3 }
```

is equivalent to

```yaml
$includesCount: { count: { $gte: 3 }, maxDepth: 1 }
```

The shorthand fixes `maxDepth: 1` (direct edges). It is recognized whenever the argument is a bare integer or a mapping whose keys are all `$`-prefixed (a numeric operator expression). A mapping that contains the bare `count` key — alone or with bare walk parameters — is the full form.

Full form (any combination of count predicate and walk parameters):

```yaml
$includesCount:
  count:    { $gte: 3 }
  maxDepth: 5
```

### 5.2 Walk-parameter semantics

`maxDepth: N` includes levels 1 through N inclusive in the count. `minDepth: M` excludes levels 1 through M-1. The combination `minDepth: M, maxDepth: N` counts documents at levels M through N inclusive.

Defaults in the full `count_obj` form:

- `maxDepth` absent → unbounded (the count covers every transitively reachable document).
- `minDepth` absent → 1 (the count starts at direct edges).

The two shorthand forms (bare integer, numeric operator expression) bypass the unbounded default and fix `maxDepth: 1` instead — see §5.1.

Transitive counts use the full form with `maxDepth` omitted:

```yaml
$includesCount:
  count: { $gte: 10 }                            # 10+ descendants reachable at any depth
```

### 5.3 Value constraints

- `count` accepts a non-negative integer literal (implicit `$eq`) or a numeric operator expression. Same grammar as `num_expr` in §5.4.
- `maxDepth` accepts a positive integer ≥ 1.
- `minDepth` accepts a positive integer ≥ 1.
- `minDepth > maxDepth` (when both are present) is a parse-time error.
- An empty argument `$includesCount: {}` is a parse-time error — `count` is required when the structured form is used.
- `maxDistance` / `minDistance` inside count operators are parse-time errors — count operators count over inclusion edges only.
- No `-1` sentinel; absence is the unbounded signal in the full form.

### 5.4 Numeric expression grammar

```
num_expr ::=
    { ($eq | $ne | $gt | $gte | $lt | $lte) : int }
  | { ($in | $nin) : [int, ...] }                  # non-empty array
  | { num_expr_op: V, num_expr_op: V, ... }        # range, AND-composed comparisons
```

Combination rules:

- Multiple comparison operators in a single expression are AND-composed: `{ $gte: 2, $lte: 5 }` matches counts in [2, 5].
- `$in` and `$nin` cannot be combined with other operators in the same expression.
- Empty `$in: []` and `$nin: []` are parse-time errors.

Value constraints:

- Comparison operator values must be non-negative integers. Negative values, floats, strings, and null are parse-time errors.
- `$in` / `$nin` array elements must be non-negative integers. Mixed-type arrays and arrays containing negatives, floats, or null are parse-time errors.

### 5.5 Examples

Direct count predicates (shorthand defaults to `maxDepth: 1`):

```yaml
filter:
  $includesCount: 0                              # no direct sub-documents
  $includesCount: { $gte: 3 }                    # 3+ direct sub-documents
  $includedByCount: 0                            # roots
  $includedByCount: { $gte: 2 }                  # polyhierarchy
```

Transitive count predicates (full form, walk parameters omitted):

```yaml
filter:
  $includesCount:
    count: { $gte: 10 }                          # 10+ descendants anywhere below
```

```yaml
filter:
  $includedByCount:
    count: { $lte: 2 }                           # at most 2 ancestors total
```

Bounded-depth count predicates:

```yaml
filter:
  $includesCount:
    count:    { $gte: 5 }
    maxDepth: 3                                  # 5+ documents within 3 levels below
```

```yaml
filter:
  $includedByCount:
    count:    { $eq: 1 }
    maxDepth: 5                                  # exactly one ancestor within 5 levels
```

Range-bounded count predicates:

```yaml
filter:
  $includesCount:
    count:    { $gte: 1 }
    minDepth: 2
    maxDepth: 4                                  # at least one descendant 2-4 levels deep
```

```yaml
filter:
  $includesCount:
    count:    0
    minDepth: 2                                  # no descendants beyond direct children
```

## 6. Relational operators

The four relational operators predicate that the document being filtered stands in a graph relation to documents matching an anchor specification. The anchor specification is a `match` filter document — the full filter language, evaluated to select an anchor set.

| Operator | True when this document... | Edge type | Walk parameters |
|---|---|---|---|
| `$includes` | has outbound inclusion edges to anchor docs within bounds | inclusion | `maxDepth`, `minDepth` |
| `$includedBy` | has inbound inclusion edges from anchor docs within bounds | inclusion | `maxDepth`, `minDepth` |
| `$references` | has outbound reference edges to anchor docs within bounds | reference | `maxDistance`, `minDistance` |
| `$referencedBy` | has inbound reference edges from anchor docs within bounds | reference | `maxDistance`, `minDistance` |

`$includes` and `$includedBy` walk only inclusion edges. `$references` and `$referencedBy` walk only reference edges. The two edge kinds are kept on separate operators; there is no combined-edge walk operator in v1.

### 6.1 Argument shape

Each relational operator accepts either a scalar key (shorthand) or a mapping with a `match` field and optional walk parameters:

```
relational_arg ::= key | relational_obj

relational_obj ::= {
  match:       filter        (required)
  maxDepth:    pos_int       (inclusion ops only; optional, absent = unbounded)
  minDepth:    pos_int       (inclusion ops only; optional, absent = 1)
  maxDistance: pos_int       (reference ops only; optional, absent = unbounded)
  minDistance: pos_int       (reference ops only; optional, absent = 1)
}
```

Field names inside `relational_obj` are bare-named — `$`-prefix is reserved for operators that evaluate, not configuration. The `match` field's value is a filter document; any `$`-prefixed names appearing inside it are filter-language operators, not walk configuration.

A scalar key K is shorthand that fixes a direct-edge walk:

- For inclusion operators: `K` is equivalent to `{ match: { $key: K }, maxDepth: 1 }`.
- For reference operators: `K` is equivalent to `{ match: { $key: K }, maxDistance: 1 }`.

Use the full mapping form to anchor by predicate, to widen the walk, or to use range bounds. In the full form, walk parameters are independent: `maxDepth` / `maxDistance` absent → unbounded; `minDepth` / `minDistance` absent → 1.

Examples:

```yaml
# Scalar shorthand — single-document anchor at depth/distance 1
$includes:     roadmap/q2
$includedBy:   projects/alpha
$references:   people/alice
$referencedBy: archive/index

# Full form, maxDepth omitted — fully unbounded walk
$includedBy: { match: { $key: projects/alpha } }

# Anchor by identity with explicit bounds
$includes:   { match: { $key: roadmap/q2 },     maxDepth: 2 }
$includedBy: { match: { $key: projects/alpha }, maxDepth: 5 }

# Anchor by frontmatter predicate
$includes:   { match: { status: draft },                       maxDepth: 2 }
$includedBy: { match: { status: active, type: project },       maxDepth: 5 }

# Anchor by OR over predicates
$includes:
  match:
    $or:
      - status: draft
      - tag: important
  maxDepth: 2

# Anchor by nested relational predicate
$includes:
  match:
    $includedBy: { match: { $key: projects/alpha }, maxDepth: 5 }
  maxDepth: 2

# Range bounds
$includedBy:   { match: { $key: projects/alpha }, minDepth: 2, maxDepth: 5 }
$referencedBy: { match: { $key: archive/index },  minDistance: 1, maxDistance: 3 }
```

### 6.2 The `match` field

`match` is a filter document. It accepts the full filter language: bare frontmatter fields, `$`-prefixed filter operators (`$key`, `$or`, `$and`, `$not`, comparison operators, element operators, array operators), graph operators including `$includesCount` / `$includedByCount`, and **nested relational operators**. Nesting allows walks anchored at the result of another walk:

```yaml
# Documents transitively included by anything that's a descendant of projects/alpha
$includedBy:
  match:
    $includedBy: { match: { $key: projects/alpha }, maxDepth: 5 }
  maxDepth: 5
```

`match` and the surrounding filter share one definition. Inside `match`, `$key` is the top-level identity operator from §4 — it accepts a scalar or any of the §4 key expressions (`$in`, `$nin`, `$eq`, `$ne`):

```yaml
# Anchor set is two named documents
$includedBy:
  match:
    $key: { $in: [projects/alpha, projects/beta] }
  maxDepth: 5
```

This subsumes what previous revisions of the spec called "OR-of-anchors" — write the OR inside `match`.

### 6.3 Walk parameters

Walk parameters constrain how far the walk extends from the anchor set.

Inclusion-edge operators (`$includes`, `$includedBy`) use `maxDepth` / `minDepth`:

- `maxDepth: N` — walk includes levels 1 through N inclusive.
- `minDepth: M` — walk excludes levels 1 through M-1; only levels ≥ M match.
- Combining `minDepth: M, maxDepth: N` matches levels M through N inclusive (M ≤ N required; M > N is a parse-time error).

Reference-edge operators (`$references`, `$referencedBy`) use `maxDistance` / `minDistance`:

- `maxDistance: N` — walk includes hops 1 through N inclusive.
- `minDistance: M` — walk excludes hops 1 through M-1; only hops ≥ M match.
- Combining `minDistance: M, maxDistance: N` matches hops M through N inclusive (same M ≤ N constraint).

Defaults in the full mapping form:

- `maxDepth` / `maxDistance` absent → unbounded (the walk reaches every transitively related document).
- `minDepth` / `minDistance` absent → 1 (the walk starts at level / hop 1).
- Both absent → fully unbounded walk over the relevant edge kind.

Scalar-key shorthand bypasses the unbounded default and fixes `maxDepth: 1` (or `maxDistance: 1`); see §6.1.

Wrong-category walk parameters (`maxDistance` / `minDistance` inside an inclusion-edge operator, or `maxDepth` / `minDepth` inside a reference-edge operator) are parse-time errors.

Value constraints on walk parameters:

- All values are positive integers (≥ 1). Zero, negatives, floats, strings, null, and operator expressions are parse-time errors.
- Operator expressions inside walk-parameter values (`maxDepth: { $lte: 5 }`) are reserved for a future revision.
- No `-1` sentinel — absence in the full form is the unbounded signal.

Anchor exclusion: a relational operator never matches a document in its anchor set. `$includedBy: { match: { $key: K }, maxDepth: 5 }` matches the documents that K transitively includes within 5 levels but does not match K itself. More generally, a `match` that selects a set S contributes anchors S, and the walk's matches are documents reached *from* S — never S itself. To include the anchor set in the result, compose at the filter level:

```yaml
$or:
  - $key: projects/alpha
  - $includedBy: { match: { $key: projects/alpha }, maxDepth: 5 }
```

### 6.4 Composition

A filter document may contain at most one occurrence of each top-level relational operator key (a YAML mapping cannot have duplicate keys). To express AND, OR, or NOT of multiple predicates using the same operator key, use the filter-level logical operators:

```yaml
# AND of two $includedBy predicates with different bounds
$and:
  - $includedBy: { match: { $key: projects/alpha }, maxDepth: 5 }
  - $includedBy: { match: { type: research, status: active }, maxDepth: 3 }
```

```yaml
# OR of two anchor sets — same edge type, different bounds
$or:
  - $includedBy: { match: { $key: projects/alpha }, maxDepth: 5 }
  - $includedBy: { match: { $key: research/q2 },    maxDepth: 2 }
```

The previous revision had a per-operator array form (`$includedBy: [anchor, anchor]`) for AND-of-anchors. That form is removed in this revision; the filter-level `$and` is the canonical composition path. AND of multiple keyed anchors with the same bounds is also expressible by widening `match`:

```yaml
# Documents descendant of either projects/alpha or projects/beta within 5 levels
$includedBy:
  match:
    $key: { $in: [projects/alpha, projects/beta] }
  maxDepth: 5
```

### 6.5 Empty argument

The empty mapping `$includedBy: {}` is a parse-time error — `match` is required. A mapping without `match` is also a parse-time error, regardless of which walk parameters are present. The array form `$includedBy: []` (and the array form generally) is no longer accepted; passing an array is a parse-time error.

A `match` filter that selects no documents is well-formed but contributes an empty anchor set; the relational predicate then matches nothing.

## 7. Composition

These operators participate in the filter language's predicate algebra exactly like other operators.

Top-level AND — multiple top-level keys in a filter are AND-composed:

```yaml
filter:
  $key:           { $nin: [drafts/scratch, drafts/temp] }
  $includesCount: { $gte: 3 }
  $includedBy:    { match: { $key: projects/alpha }, maxDepth: 5 }
  status: draft
```

`$and` / `$or` / `$not` — the logical operators wrap any filter document, including ones containing these operators:

```yaml
filter:
  $or:
    - $includedByCount: 0
    - $includedBy: archive/index
```

Empty `filter: {}` matches every document.

## 8. Worked examples

### 8.1 Identity-based queries

```yaml
# Direct lookup
filter:
  $key: people/alice
```

```yaml
# Bulk fetch by key set
filter:
  $key: { $in: [projects/alpha, projects/beta, projects/gamma] }
```

```yaml
# Anchor + descendants
filter:
  $or:
    - $key: projects/alpha
    - $includedBy: { match: { $key: projects/alpha }, maxDepth: 5 }
```

```yaml
# Exclusion within a result set
filter:
  $includedBy: { match: { $key: projects/alpha }, maxDepth: 5 }
  $key:        { $ne: projects/alpha/private }
```

### 8.2 Count-based queries

```yaml
# Roots
filter:
  $includedByCount: 0
```

```yaml
# Polyhierarchy
filter:
  $includedByCount: { $gte: 2 }
```

```yaml
# Hub documents
filter:
  $includesCount: { $gte: 5 }
```

```yaml
# Documents with 20+ descendants anywhere (full form; maxDepth omitted → unbounded)
filter:
  $includesCount:
    count: { $gte: 20 }
```

```yaml
# Documents with no descendants beyond direct children (range bound)
filter:
  $includesCount:
    count:    0
    minDepth: 2
```

### 8.3 Walk-based queries

```yaml
# Documents directly under alpha — scalar shorthand fixes maxDepth: 1
filter:
  $includedBy: projects/alpha
```

```yaml
# Documents anywhere under alpha — full form, maxDepth omitted → unbounded
filter:
  $includedBy: { match: { $key: projects/alpha } }
```

```yaml
# Documents under alpha within 10 levels
filter:
  $includedBy: { match: { $key: projects/alpha }, maxDepth: 10 }
```

```yaml
# Documents at exactly depth 3 under alpha
filter:
  $includedBy: { match: { $key: projects/alpha }, minDepth: 3, maxDepth: 3 }
```

```yaml
# Documents within 1 hop of alice
filter:
  $references: people/alice
```

```yaml
# Documents 2 to 3 hops from the archive
filter:
  $referencedBy: { match: { $key: archive/index }, minDistance: 2, maxDistance: 3 }
```

```yaml
# Documents under any active project
filter:
  $includedBy:
    match:
      type:   project
      status: active
    maxDepth: 5
```

### 8.4 Combined queries

```yaml
# Hub documents under alpha
filter:
  $includedBy:    { match: { $key: projects/alpha }, maxDepth: 10 }
  $includesCount: { $gte: 5 }
```

```yaml
# Documents under alpha that reference alice
filter:
  $includedBy: { match: { $key: projects/alpha }, maxDepth: 5 }
  $references: people/alice
```

```yaml
# Documents under alpha with 20+ descendants, excluding the private namespace
filter:
  $includedBy: { match: { $key: projects/alpha }, maxDepth: 10 }
  $includesCount:
    count: { $gte: 20 }                          # full form, maxDepth omitted → unbounded
  $key: { $nin: [projects/alpha/private] }
```

```yaml
# Roots with rich content
filter:
  $includedByCount: 0
  $includesCount:   { $gte: 5 }
```

## 9. Edge cases

- **Empty corpus** — every relational predicate matches nothing; count predicates evaluate against the empty graph and return the empty set.
- **Empty anchor set** — a `match` filter that selects no documents (e.g. `match: { $key: typo }` against a corpus with no such key, or `match: { status: nonsense }`) contributes no anchors; the relational predicate matches nothing. Typos and stale references narrow the result rather than failing the operation.
- **Cycles** — IWE's inclusion graph is a DAG by construction. Walks terminate without explicit cycle handling.
- **Disconnected graph** — walks operate per connected component; a walk anchored at K matches only documents reachable from K within bounds.
- **Self-references** — inclusion edges cannot be self-referencing by IWE's data model. Reference edges can; an inline self-link counts toward both the document's outbound and inbound reference counts.
- **Anchor exclusion** — a walk never matches a document in its anchor set. Use filter-level `$or` with `$key` (or with another predicate) to include the anchor set in the result.
- **Default walk depth** — scalar-key shorthand fixes `maxDepth: 1` / `maxDistance: 1` (direct edges only). The full mapping form treats omitted `maxDepth` / `maxDistance` as unbounded; omitted `minDepth` / `minDistance` always default to 1.
- **Operators inside `$not`** — `$not: { $includesCount: { $gte: 3 } }` matches documents with fewer than 3 outbound inclusion edges. `$not: { $includedBy: { match: { $key: K }, maxDepth: 5 } }` matches documents that are *not* descendants of K within 5 levels.

## 10. Out of scope (v1)

- **Operator expressions inside walk parameters** — `maxDepth: { $lte: 5 }` is reserved for a future revision. v1 accepts positive integer literals only.
- **Pattern matching on `$key`** — glob, regex, and prefix matching are deferred to v2.
- **Combined-edge walks** — no operator that walks both inclusion and reference edges in a single predicate. Use separate predicates.
- **Edge-kind filtering on inclusion / reference operators** — `$includes` is always inclusion edges; `$references` is always reference edges. No mixing within an operator.
- **Path predicates** — operators like "on the shortest path between A and B" or "reachable from K via any path" are deferred to a future graph-algorithms spec.
- **Transitive count operators on references** — there are no `$referencesCountAll` / `$referencedByCountAll` operators. Reference graphs can fan out arbitrarily; transitive reference counts are deferred.

## 11. Grammar reference

The full grammar covering operation documents, filter, projection, sort, limit, update operators, and graph operators lives in `query-language-grammar.md`.
