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

| Operator | Reads as | This doc → K? | K → This doc? |
|---|---|---|---|
| `$includes` | this doc includes K | yes (outbound inclusion) | no |
| `$includedBy` | this doc is included by K | no | yes (inbound inclusion) |
| `$references` | this doc references K | yes (outbound reference) | no |
| `$referencedBy` | this doc is referenced by K | no | yes (inbound reference) |

## 3. Operator inventory

| Category | Operator | Predicate over... |
|---|---|---|
| Identity (§4) | `$key` | the document's own key |
| Count (§5) | `$includesCount` | count of documents reachable via outbound inclusion edges |
| Count (§5) | `$includedByCount` | count of documents reachable via inbound inclusion edges |
| Relational (§6) | `$includes` | the document's outbound inclusion relation to keyed anchors |
| Relational (§6) | `$includedBy` | the document's inbound inclusion relation to keyed anchors |
| Relational (§6) | `$references` | the document's outbound reference relation to keyed anchors |
| Relational (§6) | `$referencedBy` | the document's inbound reference relation to keyed anchors |

The vocabulary is closed in v1. Unknown `$`-prefixed operator names inside a filter are parse-time errors.

Naming conventions: all operator names are camelCase, `$`-prefixed. Bound modifiers inside walk arguments (`$maxDepth`, `$minDistance`, etc.) are sub-operators following the same convention.

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

### 4.4 Distinguishing top-level `$key` from walk-argument `$key`

The same name `$key` appears in two positions with different rules:

| Position | Accepts | Purpose |
|---|---|---|
| Top-level `$key` | scalar OR operator expression | predicate over this document's own key |
| Walk-argument `$key` (inside `$includes` / `$includedBy` / `$references` / `$referencedBy`) | scalar only | names the anchor document for the walk |

Walk-argument `$key` is defined in §6.2 and does not accept operator expressions.

## 5. Count operators

The count operators predicate over the number of documents reachable from the document being filtered along inclusion edges, within configurable depth bounds.

| Operator | Counts documents... |
|---|---|
| `$includesCount` | reachable via outbound inclusion edges (descendants in the inclusion DAG) |
| `$includedByCount` | reachable via inbound inclusion edges (ancestors in the inclusion DAG) |

Both operators take either a numeric predicate (sugar for "direct count") or a structured argument with a numeric expression on `$count` plus optional depth bounds. The depth bounds determine which documents are included in the count; the numeric expression filters the count value.

### 5.1 Argument shape

```
count_op  ::= int | num_expr | count_arg

count_arg ::= {
    $count:    int | num_expr      (required)
    $maxDepth: pos_int | -1        (optional, default 1)
    $minDepth: pos_int             (optional, default 1)
}
```

Bare integer form (sugar for direct count equality):

```yaml
$includesCount: 0
```

is equivalent to

```yaml
$includesCount: { $count: 0, $maxDepth: 1 }
```

Bare expression form (sugar for direct count predicate):

```yaml
$includesCount: { $gte: 3 }
```

is equivalent to

```yaml
$includesCount: { $count: { $gte: 3 }, $maxDepth: 1 }
```

The shorthand forms are permitted only when no depth bounds are needed. When `$maxDepth` or `$minDepth` is specified, the full `$count`-keyed form is required to disambiguate the count predicate from depth predicates.

Full form (any combination of count predicate and bounds):

```yaml
$includesCount:
  $count:    { $gte: 3 }
  $maxDepth: 5
```

### 5.2 Bound semantics

`$maxDepth: N` includes levels 1 through N inclusive in the count. `$minDepth: M` excludes levels 1 through M-1. The combination `$minDepth: M, $maxDepth: N` counts documents at levels M through N inclusive.

Defaults:

- `$maxDepth` absent → `$maxDepth: 1` (direct edges only).
- `$minDepth` absent → `$minDepth: 1` (start at direct edges).

The defaults make `$includesCount: 0` and `$includesCount: { $gte: 3 }` behave as direct-edge counts.

Special value for unbounded transitive count:

`$maxDepth: -1` walks the entire reachable subgraph. This is the only non-positive value accepted in `$maxDepth`. Equivalent to "no upper bound."

```yaml
$includesCount: { $count: { $gte: 10 }, $maxDepth: -1 }
```

This expresses transitive count predicates (counting all descendants / ancestors regardless of depth).

### 5.3 Value constraints

- `$count` accepts a non-negative integer literal (implicit `$eq`) or a numeric operator expression. Same grammar as `num_expr` in §5.4.
- `$maxDepth` accepts a positive integer ≥ 1, or the literal `-1` for unbounded.
- `$minDepth` accepts a positive integer ≥ 1.
- `$minDepth > $maxDepth` (with `-1` treated as unbounded) is a parse-time error.
- An empty argument `$includesCount: {}` is a parse-time error — `$count` is required when the structured form is used.
- Bound modifiers from the relational vocabulary (`$maxDistance`, `$minDistance`) are parse-time errors here — count operators count over inclusion edges only.

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

Direct count predicates (depth defaults to 1):

```yaml
filter:
  $includesCount: 0                              # leaves
  $includesCount: { $gte: 3 }                    # 3+ direct sub-documents
  $includedByCount: 0                            # roots
  $includedByCount: { $gte: 2 }                  # polyhierarchy
```

Transitive count predicates (full reachable subgraph):

```yaml
filter:
  $includesCount:
    $count:    { $gte: 10 }
    $maxDepth: -1                                # 10+ descendants anywhere below
```

```yaml
filter:
  $includedByCount:
    $count:    { $lte: 2 }
    $maxDepth: -1                                # at most 2 ancestors total
```

Bounded-depth count predicates:

```yaml
filter:
  $includesCount:
    $count:    { $gte: 5 }
    $maxDepth: 3                                 # 5+ documents within 3 levels below
```

```yaml
filter:
  $includedByCount:
    $count:    { $eq: 1 }
    $maxDepth: 5                                 # exactly one ancestor within 5 levels
```

Range-bounded count predicates:

```yaml
filter:
  $includesCount:
    $count:    { $gte: 1 }
    $minDepth: 2
    $maxDepth: 4                                 # at least one descendant 2-4 levels deep
```

```yaml
filter:
  $includesCount:
    $count:    0
    $minDepth: 2
    $maxDepth: -1                                # no descendants beyond direct children
```

## 6. Relational operators

The four relational operators predicate that the document being filtered stands in a graph relation to one or more keyed anchors.

| Operator | True when this document... | Edge type | Bound modifier vocabulary |
|---|---|---|---|
| `$includes` | has outbound inclusion edges to K within bounds | inclusion | `$maxDepth`, `$minDepth` |
| `$includedBy` | has inbound inclusion edges from K within bounds | inclusion | `$maxDepth`, `$minDepth` |
| `$references` | has outbound reference edges to K within bounds | reference | `$maxDistance`, `$minDistance` |
| `$referencedBy` | has inbound reference edges from K within bounds | reference | `$maxDistance`, `$minDistance` |

`$includes` and `$includedBy` walk only inclusion edges. `$references` and `$referencedBy` walk only reference edges. The two edge kinds are kept on separate operators; there is no combined-edge walk operator in v1.

### 6.1 Argument shape

Each relational operator accepts either a single anchor or an array of anchors:

```
relational_op ::= anchor | [ anchor, anchor, ... ]

anchor ::= {
  $key:         key       (required)
  $maxDepth:    pos_int   (inclusion ops only; required if $minDepth absent)
  $minDepth:    pos_int   (inclusion ops only; optional)
  $maxDistance: pos_int   (reference ops only; required if $minDistance absent)
  $minDistance: pos_int   (reference ops only; optional)
}
```

Examples:

```yaml
# Single anchor with $maxDepth
$includes:      { $key: roadmap/q2,     $maxDepth: 2 }
$includedBy:    { $key: projects/alpha, $maxDepth: 5 }
$references:    { $key: people/dmytro,  $maxDistance: 1 }
$referencedBy:  { $key: archive/index,  $maxDistance: 2 }

# Single anchor with range bounds
$includedBy:    { $key: projects/alpha, $minDepth: 2, $maxDepth: 5 }
$referencedBy:  { $key: archive/index,  $minDistance: 1, $maxDistance: 3 }

# Multi-anchor (AND)
$includedBy:
  - { $key: projects/alpha, $maxDepth: 5 }
  - { $key: research/q2,    $maxDepth: 2 }
```

### 6.2 The `$key` sub-operator

Inside a relational operator's anchor argument, `$key` names the anchor document. It accepts a scalar key only — operator expressions (`$in`, `$ne`, etc.) are not permitted here.

```yaml
# Valid
$includedBy: { $key: projects/alpha, $maxDepth: 5 }

# Invalid — operator expression not allowed in walk-argument $key
$includedBy: { $key: { $in: [projects/alpha, projects/beta] }, $maxDepth: 5 }
```

For OR-of-anchors (match documents reachable from any of K1, K2, ...), use `$or` at the filter level:

```yaml
$or:
  - $includedBy: { $key: projects/alpha, $maxDepth: 5 }
  - $includedBy: { $key: projects/beta,  $maxDepth: 5 }
```

For AND-of-anchors (match documents reachable from all of K1, K2, ...), use the array form. See §6.4.

The walk-argument `$key` is syntactically distinct from the top-level `$key` operator in §4: top-level `$key` predicates the document's own identity and accepts operator expressions; walk-argument `$key` names an anchor and accepts a scalar only. The two share a name but are distinguished by position.

### 6.3 Bound modifiers

Bound modifiers constrain how far the walk extends from the anchor.

Inclusion-edge operators (`$includes`, `$includedBy`) use `$maxDepth` / `$minDepth`:

- `$maxDepth: N` — walk includes levels 1 through N inclusive.
- `$minDepth: M` — walk excludes levels 1 through M-1; only levels ≥ M match.
- Combining `$minDepth: M, $maxDepth: N` matches levels M through N inclusive (M ≤ N required; M > N is a parse-time error).

Reference-edge operators (`$references`, `$referencedBy`) use `$maxDistance` / `$minDistance`:

- `$maxDistance: N` — walk includes hops 1 through N inclusive.
- `$minDistance: M` — walk excludes hops 1 through M-1; only hops ≥ M match.
- Combining `$minDistance: M, $maxDistance: N` matches hops M through N inclusive (same M ≤ N constraint).

Required-modifier rule: every anchor must carry at least one bound modifier. `{ $key: K }` with no bounds is a parse-time error. For inclusion-edge operators, at least one of `$maxDepth` or `$minDepth` must be present. Reference-edge bound modifiers (`$maxDistance`, `$minDistance`) inside an inclusion-edge operator are parse-time errors, and vice versa.

Value constraints on bounds:

- All bound values are positive integers (≥ 1). Zero, negatives, floats, strings, null, and operator expressions are parse-time errors.
- Operator expressions inside bound values (`$maxDepth: { $lte: 5 }`) are reserved for a future revision.
- Unlike count operators, relational bounds do not accept `-1` (unbounded).

Anchor exclusion: a relational operator never matches its anchor itself. `$includedBy: { $key: K, $maxDepth: 5 }` matches K's descendants (in the inclusion sense — documents that K transitively includes) within 5 levels but does not match K. To include the anchor in the result, combine with the top-level `$key` operator:

```yaml
$or:
  - $key: projects/alpha
  - $includedBy: { $key: projects/alpha, $maxDepth: 5 }
```

### 6.4 Multi-anchor semantics: AND

When the operator value is an array of anchors, the anchors AND together. The filter

```yaml
$includedBy:
  - { $key: projects/alpha, $maxDepth: 5 }
  - { $key: research/q2,    $maxDepth: 3 }
```

matches documents that are descendants of both `projects/alpha` (within 5 levels) and `research/q2` (within 3 levels).

This matches the AND-by-default convention of the surrounding filter language. For OR over anchors, lift the relational operator into `$or` (see §6.2).

A filter document may contain at most one occurrence of each top-level relational operator key. The array form is the only way to AND multiple anchors of the same edge type and direction within a single filter level. To AND multiple `$includedBy` predicates with different bound semantics, use `$and`:

```yaml
$and:
  - $includedBy: { $key: projects/alpha, $maxDepth: 5 }
  - $includedBy: { $key: research/q2,    $minDepth: 2, $maxDepth: 3 }
```

### 6.5 Empty argument

The empty mapping `$includedBy: {}` and the empty array `$includedBy: []` are parse-time errors. Every relational predicate must list at least one anchor with at least one bound modifier.

## 7. Composition

These operators participate in the filter language's predicate algebra exactly like other operators.

Top-level AND — multiple top-level keys in a filter are AND-composed:

```yaml
filter:
  $key:           { $nin: [drafts/scratch, drafts/temp] }
  $includesCount: { $gte: 3 }
  $includedBy:    { $key: projects/alpha, $maxDepth: 5 }
  status: draft
```

`$and` / `$or` / `$not` — the logical operators wrap any filter document, including ones containing these operators:

```yaml
filter:
  $or:
    - $includedByCount: 0
    - $includedBy: { $key: archive/index, $maxDepth: 1 }
```

Empty `filter: {}` matches every document.

## 8. Worked examples

### 8.1 Identity-based queries

```yaml
# Direct lookup
filter:
  $key: people/dmytro
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
    - $includedBy: { $key: projects/alpha, $maxDepth: 5 }
```

```yaml
# Exclusion within a result set
filter:
  $includedBy: { $key: projects/alpha, $maxDepth: 5 }
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
# Documents with 20+ descendants anywhere
filter:
  $includesCount:
    $count:    { $gte: 20 }
    $maxDepth: -1
```

```yaml
# Documents with no descendants beyond direct children
filter:
  $includesCount:
    $count:    0
    $minDepth: 2
    $maxDepth: -1
```

### 8.3 Walk-based queries

```yaml
# Documents directly under alpha
filter:
  $includedBy: { $key: projects/alpha, $maxDepth: 1 }
```

```yaml
# Documents anywhere under alpha
filter:
  $includedBy: { $key: projects/alpha, $maxDepth: 10 }
```

```yaml
# Documents at exactly depth 3 under alpha
filter:
  $includedBy: { $key: projects/alpha, $minDepth: 3, $maxDepth: 3 }
```

```yaml
# Documents within 1 hop of dmytro
filter:
  $references: { $key: people/dmytro, $maxDistance: 1 }
```

```yaml
# Documents 2 to 3 hops from the archive
filter:
  $referencedBy: { $key: archive/index, $minDistance: 2, $maxDistance: 3 }
```

### 8.4 Combined queries

```yaml
# Hub documents under alpha
filter:
  $includedBy:    { $key: projects/alpha, $maxDepth: 10 }
  $includesCount: { $gte: 5 }
```

```yaml
# Documents under alpha that reference dmytro
filter:
  $includedBy: { $key: projects/alpha, $maxDepth: 5 }
  $references: { $key: people/dmytro,  $maxDistance: 1 }
```

```yaml
# Documents under alpha with 20+ descendants, excluding the private namespace
filter:
  $includedBy: { $key: projects/alpha, $maxDepth: 10 }
  $includesCount:
    $count:    { $gte: 20 }
    $maxDepth: -1
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
- **Missing anchor key** — a walk against a non-existent anchor key contributes "no doc matches" (does not raise an error). A typo in an anchor narrows the result rather than failing the operation.
- **Cycles** — IWE's inclusion graph is a DAG by construction. Walks terminate without explicit cycle handling.
- **Disconnected graph** — walks operate per connected component; a walk anchored at K matches only documents reachable from K within bounds.
- **Self-references** — inclusion edges cannot be self-referencing by IWE's data model. Reference edges can; an inline self-link counts toward both the document's outbound and inbound reference counts.
- **Anchor inclusion** — a walk never matches its anchor. Use `$or` with `$key` to include the anchor.
- **Operators inside `$not`** — `$not: { $includesCount: { $gte: 3 } }` matches documents with fewer than 3 outbound inclusion edges. `$not: { $includedBy: { $key: K, $maxDepth: 5 } }` matches documents that are *not* descendants of K within 5 levels.

## 10. Out of scope (v1)

- **Operator expressions inside bound modifiers** — `$maxDepth: { $lte: 5 }` is reserved for a future revision. v1 accepts positive integer literals (and `-1` for count operators only).
- **`$key` operator expressions inside walk arguments** — only scalar keys allowed. OR-of-anchors goes through `$or`.
- **Pattern matching on `$key`** — glob, regex, and prefix matching are deferred to v2.
- **Combined-edge walks** — no operator that walks both inclusion and reference edges in a single predicate. Use separate predicates.
- **Edge-kind filtering on inclusion / reference operators** — `$includes` is always inclusion edges; `$references` is always reference edges. No mixing within an operator.
- **Path predicates** — operators like "on the shortest path between A and B" or "reachable from K via any path" are deferred to a future graph-algorithms spec.
- **Transitive count operators on references** — there are no `$referencesCountAll` / `$referencedByCountAll` operators. Reference graphs can fan out arbitrarily; transitive reference counts are deferred.

## 11. Grammar reference

The full grammar covering operation documents, filter, projection, sort, limit, update operators, and graph operators lives in `query-language-grammar.md`.
