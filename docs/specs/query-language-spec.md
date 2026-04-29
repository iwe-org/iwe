# IWE Query Language Spec

## 1. Overview

This document specifies the IWE query language: a YAML-based, MongoDB-style language for selecting, shaping, and mutating documents in an IWE workspace. It defines:

- The **corpus model** — what a document is, what reserved prefixes the engine holds.
- The four **operations** — find, count, update, delete — and the shape of an operation document.
- The **language constructs** — filter operators, projection, sort, limit, update operators — plus how they compose. Graph operators that extend filter with cross-document selection live in `query-graph-spec.md`.

## 2. Corpus model

### 2.1 Documents

A **document** is the parsed frontmatter of one note. Documents are mappings from string keys to YAML-typed values: strings, numbers, booleans, null, lists, mappings, dates, datetimes.

Notes with no frontmatter participate in the corpus as documents with an empty mapping (`{}`). They never match presence-style filters like `{status: draft}` but do match `{status: {$exists: false}}`.

### 2.2 The corpus

The **corpus** is every document in the IWE workspace.

### 2.3 Reserved field-name prefixes

Frontmatter field names whose **first character is `_`, `$`, `.`, `#`, or `@`** are reserved by the engine. They are **invisible to user-facing query operations**: filter, sort, and projection paths that reference such names resolve as missing; reserved entries never appear in `find` output (with or without projection); and `update` strips them from each affected document before the new frontmatter is rendered.

A reserved-prefix entry may exist in a file's raw frontmatter on disk — the engine does not refuse to load it — but every user-visible touchpoint (queries, results, mutated output) behaves as if it weren't there. Update writeback is the round-trip moment when such entries are dropped: any document the user mutates loses its reserved-prefix entries on the way out.

User frontmatter field names must not begin with any of the five reserved characters. Any other leading character — letter, digit, hyphen, slash, parenthesis, etc. — is unreserved and addressable as a regular field. Subsequent characters within a name are unconstrained per YAML rules.

The reserved prefixes have distinct roles:

- `$`-prefixed names — operator expressions everywhere in the language (`$eq`, `$set`, `$walk`, etc.). Never user field names.
- `_`, `.`, `#`, `@` — held for future use. The v1 spec defines no concrete pseudo-fields; future spec amendments may introduce them without language changes.

This rule is what makes the operator vocabulary safe: `$`-prefixed keys in a filter or update document never collide with a user field of the same name, because such user fields cannot be referenced by query operations.

## 3. Operations and operation documents

### 3.1 Operations

| Operation | Returns / effect |
|---|---|
| `find` | Returns matched documents (subject to `project`, §5). |
| `count` | Returns the integer count of matched documents. |
| `update` | Mutates each matched document by applying an update document (§8). |
| `delete` | Removes each matched note. |

### 3.2 Operation-document structure

Every operation document is one YAML mapping. Top-level fields:

| Field | Operations | Purpose |
|---|---|---|
| `filter` | all | Predicate document (§4). Required on `update` / `delete`. Graph operators that extend filter with cross-document selection are defined in `query-graph-spec.md`. |
| `project` | find | Projection (§5). |
| `sort` | all | §6. On `update` / `delete`, bounds iteration order before mutation. |
| `limit` | all | §7. On `update` / `delete`, bounds the number of mutated / removed docs. |
| `update` | update | Update document (§8). Required on `update`. |

Operation-inappropriate fields are an error. The valid field set per operation:

| Operation | Allowed fields |
|---|---|
| `find` | `filter`, `project`, `sort`, `limit` |
| `count` | `filter`, `sort`, `limit` |
| `update` | `filter` (required), `sort`, `limit`, `update` (required) |
| `delete` | `filter` (required), `sort`, `limit` |

E.g. `project` in a `count` / `update` / `delete` operation, or `update` in a `find` / `count` / `delete` operation, are parse-time errors.

`filter` is required on both `update` and `delete` to prevent accidental whole-corpus mutation. The empty filter `{}` matches all documents and must be passed explicitly.

Example — a complete `find` operation document combining selection, projection, sort, and limit:

```yaml
filter:
  $or:
    - $key: projects/alpha
    - $child_of: { projects/alpha: { depth: 5 } }
  status: draft
  priority: { $gte: 5 }
project:
  title: 1
  modified_at: 1
sort:
  modified_at: -1
limit: 100
```

Example — an `update` operation document:

```yaml
filter:
  $or:
    - $key: projects/alpha
    - $child_of: { projects/alpha: { depth: 5 } }
  status: draft
  priority: { $gte: 5 }
sort:
  modified_at: -1
limit: 100
update:
  $set:
    flagged: true
    review_state: needs-review
```

Example — a `delete` operation document:

```yaml
filter:
  $or:
    - $key: archive/2024
    - $child_of: { archive/2024: { depth: 5 } }
  status: archived
limit: 500
```

## 4. Filter language

A filter document is a predicate evaluated against each document in the corpus. A document matches when every top-level key matches.

Filter top-level keys are either user frontmatter field names (e.g. `status`, `priority`, `tags`) or `$`-prefixed operator names. The operator family includes the logical operators (`$and`, `$or`, `$not`; §4.6) and the **graph operators** (`$child_of`, `$is_root`, `$key`, etc.) defined in `query-graph-spec.md`. Both kinds compose freely with frontmatter predicates under the same algebra.

### 4.1 Implicit equality (bare values)

A bare value at a field key is an equality predicate:

```yaml
filter:
  status: draft
```

Matches documents where `status` equals `"draft"`. The behavior of "equals" depends on the value type and the field type — see §4.5 for the full rule. The short version:

| Predicate value | Field value | Matches when... |
|---|---|---|
| Scalar (string / number / bool / null / date) | Scalar | Values are deeply equal. |
| Scalar | Array | Any element of the array deeply equals the scalar (membership). |
| Array | Array | Arrays are deeply equal (same elements, same order). |
| Mapping | Mapping | Mappings are deeply equal. |
| Anything | Missing field | Never matches. |
| Anything | Type mismatch | Never matches. |

### 4.2 Operator expressions

A mapping value whose keys are all `$`-prefixed is an **operator expression**:

```yaml
priority: { $gt: 3 }
```

This is unambiguous because user field names cannot begin with `$` (see §2.3). Any `$`-prefixed key in a filter is always an operator, never a field reference.

Multiple operators in one expression are ANDed:

```yaml
priority: { $gte: 3, $lte: 7 }      # 3 ≤ priority ≤ 7
```

A mapping with **mixed** `$`-prefixed and bare keys at the same level is an error — it's ambiguous whether the bare keys are nested fields or part of the operator expression. Use one form per level:

```yaml
# OK — operator expression
author: { $eq: dmytro }

# OK — nested field
author:
  name: dmytro

# ERROR — mixed
author:
  $eq: dmytro
  name: dmytro
```

### 4.3 Multiple keys are ANDed

Multiple top-level keys in a filter combine with AND:

```yaml
status: draft
priority: { $gt: 3 }
tags: rust
```

A document matches if every top-level key matches. To express OR, wrap with `$or` (§4.6).

### 4.4 Nested fields

Nested fields can be addressed two ways. Both forms are equivalent:

**Nested mapping:**

```yaml
author:
  name: dmytro
```

**Dotted-key shorthand:**

```yaml
author.name: dmytro
```

Mixing forms in a single filter is allowed:

```yaml
status: draft
author.name: dmytro
review:
  reviewer: alice
```

Dots inside the key string always denote path separators. User frontmatter field names that contain a literal `.` cannot use the shorthand — fall back to the nested-mapping form (or quote each segment in YAML if a tooling roundtrip preserves the dotted name as one segment).

Operator expressions on a dotted key carry the same shape as on a nested key:

```yaml
priority: { $gt: 3 }                       # top-level
author.priority: { $gt: 3 }                # nested via dotted shorthand
author: { priority: { $gt: 3 } }           # equivalent
```

#### Resolution rules

When evaluating a nested-field predicate:

- If any intermediate path component is missing, or is present but not a mapping, the leaf is treated as **missing** (never matches an equality / comparison; matches `$exists: false`).
- If the intermediate path leads to a mapping, evaluation continues recursively.

Example: filter `author.name: dmytro` against document `{ author: "dmytro" }` (where `author` is a string, not a mapping) — the leaf `author.name` is missing; the predicate does not match.

### 4.5 Equality, types, and missing fields

These rules ground every operator in §4.6–§4.9.

#### Deep equality

Two values are equal when they are the same YAML type and deeply equal:

- **Scalars** — strings match by codepoint sequence; numbers by numeric value (integer and float interoperate: `3` equals `3.0`); booleans by identity; null by identity; dates / datetimes by chronological identity.
- **Arrays** — same length, element-wise deep equality, in order.
- **Mappings** — same key set, value-wise deep equality.

Cross-type comparisons are **always false** — there is no implicit coercion. `1` (number) does not equal `"1"` (string). `true` does not equal `"true"`. A YAML date does not equal a string of the same shape.

#### Array membership exception

When the predicate value is a **scalar** and the field's value is an **array**, equality tests membership: the scalar must deeply equal at least one element. This is the MongoDB convention. It applies to `$eq`, bare scalars, `$ne`, `$in`, `$nin`, and the comparison operators (`$gt`, etc.).

To test whole-array equality, write the predicate as an array literal:

```yaml
tags: [rust, async]                  # whole-array equality (length-2 array, in order)
tags: rust                           # membership ("rust" is one of the tags)
tags: { $eq: rust }                  # membership (same as bare scalar)
```

#### Null vs missing

A frontmatter field with explicit value `null` is **present** with value `null`:

- Matches `$eq: null` and bare `null`.
- Matches `$exists: true` and `$type: null`.
- Does NOT match `$exists: false`.

A field absent from frontmatter is **missing**:

- Does NOT match `$eq: null` (or any `$eq`).
- Matches `$exists: false`.
- Does NOT match `$type` of any kind (use `$exists: false` for absence).
- Comparison operators (`$gt`, `$gte`, `$lt`, `$lte`) are always false against missing fields.
- `$ne: x` and `$nin: [...]` match missing fields (consistent with MongoDB: "not equal to x" includes "doesn't exist").

#### Type bracketing for ordering

`$gt`, `$gte`, `$lt`, `$lte` only compare values within a comparable type group:

| Group | Members | Order |
|---|---|---|
| numeric | integer, float | numerical |
| string | string | Unicode codepoint |
| temporal | date, datetime | chronological |
| boolean | boolean | `false < true` |

Cross-group comparison is always false (e.g. comparing a number with a string is false; a boolean with a number is false). Null is not orderable; ordering operators against null are always false. Use `$exists` / `$eq: null` to test for null explicitly.

### 4.6 Logical operators

Three operators compose filters: `$and`, `$or`, `$not`.

#### `$and: [filter1, filter2, ...]`

All listed filters must match.

```yaml
$and:
  - status: draft
  - priority: { $gt: 3 }
```

- Every list element is a filter document.
- A document matches if every sub-filter matches.
- **Empty list** `$and: []` is a parse-time error.
- `$and` is **implicit at the top level** — multiple top-level keys in a filter are already ANDed (§4.3). Use explicit `$and` when you need to wrap a sub-expression for use inside `$or` / `$not`, or when you need to repeat a field name across multiple sub-filters (a YAML mapping cannot have duplicate keys).

```yaml
# Two ranges on `priority` — needs $and to repeat the key
$and:
  - priority: { $lt: 3 }
  - priority: { $gt: 0 }
```

#### `$or: [filter1, filter2, ...]`

At least one of the listed filters must match.

```yaml
$or:
  - status: draft
  - status: review
```

- Every list element is a filter document.
- A document matches if at least one sub-filter matches.
- **Empty list** `$or: []` is a parse-time error.
- Sub-filters are independent — each is evaluated against the whole document.

#### `$not: filter`

The contained filter must not match.

Top-level form:

```yaml
$not:
  status: archived
```

Per-field form (wraps a sub-expression for one field):

```yaml
priority: { $not: { $gt: 5 } }
```

- Takes a single filter document (not a list).
- Negates the result.
- **Missing-field interaction:** `$not: { reviewed: true }` matches docs without a `reviewed` field, because the inner predicate doesn't match (missing field), and `$not` flips that to true. To require presence and inequality, combine: `reviewed: { $exists: true, $ne: true }`.
- `$not` cannot wrap a `$not` directly (`$not: { $not: ... }`) — double negation is redundant; use the inner expression.

### 4.7 Comparison operators

#### `$eq: VALUE`

Matches when the field's value equals VALUE.

```yaml
status: { $eq: draft }
```

- Equivalent to bare value (`status: draft`); see §4.1.
- Type-aware deep equality (§4.5).
- Array membership rule applies when VALUE is scalar and the field is an array (§4.5).
- Missing field never matches.

#### `$ne: VALUE`

Matches when the field's value does not equal VALUE.

```yaml
status: { $ne: archived }
```

- Logical negation of `$eq`.
- **Missing field matches** `$ne` (consistent with MongoDB).
- For arrays with a scalar VALUE: `$ne: rust` matches arrays that do not contain `"rust"`.

#### `$gt: VALUE` / `$gte: VALUE` / `$lt: VALUE` / `$lte: VALUE`

Ordering comparisons.

```yaml
priority: { $gt: 3 }
modified_at: { $gte: 2026-01-01 }
priority: { $gte: 3, $lte: 7 }       # closed range [3, 7]
```

- `$gt` / `$lt` are exclusive; `$gte` / `$lte` are inclusive.
- Type bracketing applies (§4.5): cross-group comparisons are always false.
- Missing field is always false.
- Arrays with scalar VALUE: matches if any element of the array satisfies the comparison.
- Combining `$gt` and `$lt` (or `$gte` / `$lte`) in one operator expression yields a range; both endpoints must hold (operator expression is ANDed, §4.2).

#### `$in: [v1, v2, ...]`

Matches when the field's value equals any element of the list.

```yaml
status: { $in: [draft, review] }
tags: { $in: [rust, async] }         # array → membership intersection
```

- Each list element is compared with the same equality rules as `$eq`.
- The list elements may be of different types; each is tested independently.
- Arrays with scalar list elements: matches if the field's array shares at least one element with the list (set intersection non-empty).
- **Empty list** `$in: []` is a parse-time error.
- Missing field never matches.

#### `$nin: [v1, v2, ...]`

Matches when the field's value is not in the list.

```yaml
status: { $nin: [archived, deleted] }
```

- Negation of `$in`.
- **Missing field matches** `$nin` (consistent with `$ne`).
- **Empty list** `$nin: []` is a parse-time error.

### 4.8 Element operators

#### `$exists: true | false`

Tests presence vs. absence of the field.

```yaml
reviewed_at: { $exists: true }
draft_notes: { $exists: false }
```

- `$exists: true` matches when the field is present in the document. The value may be anything, including null.
- `$exists: false` matches when the field is absent.
- A field with explicit null is **present**: matches `$exists: true`. To distinguish, combine: `reviewed_at: { $exists: true, $ne: null }`.
- For nested paths, the test is on the leaf. If any intermediate is missing or non-mapping, the leaf is treated as absent (§4.4).

#### `$type: TYPE` or `$type: [TYPE, TYPE, ...]`

Matches when the field's value has one of the given YAML types.

```yaml
priority: { $type: number }
ids: { $type: [string, number] }      # accepts either type
```

Accepted type names:

| Type | Matches |
|---|---|
| `string` | YAML strings (any encoding, any length, including the empty string). |
| `number` | Integers and floats together. No sub-distinction in v1. |
| `boolean` | `true` / `false`. |
| `null` | Explicit null value. |
| `array` | Sequences (any length, any element type). |
| `object` | Mappings. |
| `date` | YAML date scalars (e.g. `2026-04-26`). |
| `datetime` | YAML timestamp scalars (e.g. `2026-04-26T10:30:00Z`). |

- A field with explicit null matches `$type: null` and no other type.
- Missing field does not match any `$type`. Use `$exists: false` for absence.
- The list form is OR over types: `$type: [string, number]` matches if the value is either.
- **Empty list** `$type: []` is a parse-time error.

### 4.9 Array operators

These operators apply to fields whose value is an array. On non-array values (scalars, mappings, missing) they evaluate to **false** (no error).

#### `$all: [v1, v2, ...]`

Matches when the field's array contains every listed value.

```yaml
tags: { $all: [rust, async] }
```

- Field must be an array.
- Every element of the listed values must appear at least once in the field's array. Order is irrelevant; duplicates are irrelevant.
- Element equality follows §4.5 (deep equality, type-strict).
- **Empty list** `$all: []` is a parse-time error.
- For matching elements by predicate (not by literal equality), see `$elemMatch` in v2 (`query-language-v2-spec.md` §2.1) — deferred.

#### `$size: N`

Matches when the field's array has exactly N elements.

```yaml
tags: { $size: 0 }                   # no tags
authors: { $size: 1 }                # exactly one author
```

- N must be a non-negative integer (`$size: -1` is an error; `$size: 1.5` is an error).
- Field must be an array; non-arrays and missing fields → false.
- v1 does not accept ranges: `$size: { $gt: 3 }` is **not** supported. Use a comparison operator on a derived field, or filter post-hoc.

#### `$elemMatch` — deferred to v2

Per-element predicate matching against array fields (`tags: { $elemMatch: { $gt: 5 } }`, or per-element constraints on arrays of mappings) is **not in v1**. See `query-language-v2-spec.md` §2.1.

In v1, common cases collapse to v1 operators: bare-scalar membership (`tags: rust`), `$in` for any-of-many, `$all` for all-of-many.

### 4.10 String operators — deferred

String predicates beyond equality (regex / pattern matching) are **not supported** in v1. They may be added in a future revision. Until then, string fields are matched via `$eq`/`$ne`/`$in`/`$nin` only.

### 4.11 Filter requirements (use-case checklist)

The language MUST express the following queries directly:

| Question | Filter |
|---|---|
| All drafts | `{status: draft}` |
| Drafts modified this year | `{status: draft, modified_at: {$gte: 2026-01-01}}` |
| Tagged either rust or async | `{tags: {$in: [rust, async]}}` |
| Tagged with both rust and async | `{tags: {$all: [rust, async]}}` |
| Has no tags | `{$or: [{tags: {$exists: false}}, {tags: {$size: 0}}]}` |
| Reviewed but no reviewer | `{reviewed_at: {$exists: true}, reviewed_by: {$exists: false}}` |
| Drafts not by dmytro | `{status: draft, author: {$ne: dmytro}}` |
| Recent high-priority | `{$and: [{modified_at: {$gte: 2026-04-01}}, {$or: [{priority: {$gte: 8}}, {tags: urgent}]}]}` |

## 5. Projection

A projection document specifies which fields to return on `find`. Projection is a read-only construct — it does not appear in `count`, `update`, or `delete` operation documents.

Projection appears as the `project` field on a `find` operation document; passing `project` on a non-`find` operation is an error.

Projection is **inclusion-only** in v1: the document lists the fields you want to keep, and only those appear in the output. Any frontmatter field not named in `project` is omitted from the result.

```yaml
project:
  title: 1
  tags:
  modified_at: true
```

Each entry's value indicates "include this field". Three forms are accepted and behave identically:

| Value | Meaning |
|---|---|
| `1` | Include the field. |
| `true` | Include the field. |
| `null` (YAML `~` or empty value) | Include the field. |

There is no exclusion mode in v1 (you cannot say "give me everything except X"). To omit fields from the output, simply leave them out of `project`. Exclusion-style projection may be added in a future revision; out of scope here.

### 5.1 Nested fields

Nested fields can be addressed via nested mappings or dotted-key shorthand. Both forms behave identically:

```yaml
project:
  title: 1
  author:
    name: 1
  "review.reviewer": 1
```

Mixing the two forms in one projection is allowed. The semantic rules and field-name caveats match the filter language (§4.4).

### 5.2 Computed projection — deferred to v2

v1 projection is purely inclusion / exclusion. There are no computed operators: no array slicing, no derived fields, no expressions. `$slice` (array slicing) is deferred to v2 

### 5.3 Reserved-prefix interaction

Reserved-prefix segments are invisible to projection (§2.3): naming one as the head of an inclusion path is equivalent to naming a missing field — nothing is copied for that path, no error is raised. Projection output therefore never contains reserved-prefix entries, regardless of what the source frontmatter held on disk. Future spec amendments may add engine-populated pseudo-fields with reserved prefixes; if so, this section will be amended to specify their inclusion behavior in projections.

## 6. Sort

```yaml
sort:
  modified_at: -1
```

| Value | Meaning |
|---|---|
| `1` | Ascending |
| `-1` | Descending |

**v1 accepts exactly one sort key.** A `sort` mapping with two or more entries is a parse-time error. Multi-key sort (compound ordering, applied left-to-right) is deferred to a future revision.

Documents missing the sort key sort as if the value were null. Null sorts before all other values ascending, last descending. Sort applies to all four operations (on `update` / `delete` it bounds the iteration order before mutation).

Ties — including the no-`sort` case — are broken by document key in ascending lexicographic order. The engine sorts the matched set by key first, then applies the user-provided sort with a stable algorithm; the result is deterministic given the same corpus and operation.

## 7. Limit

A non-negative integer cap.

```yaml
limit: 20
```

`limit: 0` means no limit. Negative values are an error. Limit applies to all four operations; on `update` / `delete` it bounds the number of mutated / removed documents.

## 8. Update operators

The `update` field of a mutation operation document specifies the mutations to apply to each matched document. It must contain at least one update operator at the top level. All operators in one update document apply atomically per matched document (§9).

### 8.1 Frontmatter operators

v1 defines two frontmatter operators:

| Operator | Effect |
|---|---|
| `$set` | Set fields to values |
| `$unset` | Remove fields |

Other update operators — `$rename`, `$inc`, `$push`, `$pull`, `$addToSet`, and the body operator `$content` — are deferred to v2. See `query-language-v2-spec.md` §5.

#### `$set`

```yaml
update:
  $set:
    reviewed: true
    audited_at: 2026-04-26
    author:
      email: dmytro@example.com
    "review.reviewer": alice
```

Adds the field if absent, replaces it otherwise. Nested paths can be expressed via nested mappings or dotted-key shorthand (matching §4.4). Intermediate mappings are auto-created when a dotted path writes through a missing parent: `$set: { "a.b.c": 1 }` on a doc without `a` produces `a: { b: { c: 1 } }`.

#### `$unset`

```yaml
update:
  $unset:
    draft_notes: ""
    temporary: ""
```

Values are ignored. Absent field → no-op.

### 8.2 Reserved-prefix protection

Reserved-prefix names (`_`, `$`, `.`, `#`, `@`) are invisible to query operations and are dropped on update writeback — see §2.3. On the mutation side, **operators that target a reserved-prefix field are parse-time errors**:

```yaml
# ERROR — any reserved-prefix name is rejected
update:
  $set:
    _hidden: 1
    .secret: 2
    "#tag": foo
    "@user": bar
```

The error is detected during update-document validation. Without it, a `$set: { _hidden: 1 }` would be silently lost when writeback strips reserved-prefix entries from the rendered frontmatter — the parse-time error makes the failure loud instead. It also reserves the namespace against collision with future engine pseudo-fields introduced by spec amendments or extensions like `tree-spec.md`.

### 8.3 Combining operators

Multiple operators in one update document apply atomically per matched document. Conflicts are errors:

- `$set` and `$unset` on the same field.

### 8.4 Update requirements (use-case checklist)

The language MUST express the following mutations directly in v1:

| Operation | Update document |
|---|---|
| Mark all drafts reviewed | `$set: {reviewed: true}` |
| Promote drafts to published | `$set: {status: published, published_at: 2026-04-26}, $unset: {draft_notes: ""}` |

Other patterns — rename a field, increment a counter, add / remove array elements, idempotent additions, body overwrite (`$content`) — are deferred to v2. See `query-language-v2-spec.md` §5.

## 9. Atomicity

### 9.1 Per-document

All operators in one update document apply atomically per matched document: either every operator succeeds and the engine emits a single rewritten frontmatter for that document, or the document is reported in the failure list with an error and no replacement is emitted. There is no half-applied frontmatter — a field-level error (e.g. a `$set` / `$unset` conflict caught at parse time, or any future runtime error in `update::apply`) aborts the operation for that document only.

### 9.2 Across-document

Across-document atomicity is **not** provided. The engine itself is a pure function: given an `update` operation it returns `(changes, failed)` — `changes` lists `(key, new markdown)` pairs the host should write, `failed` lists `(key, error)` pairs that did not produce a change. A `delete` operation returns the list of keys to remove. The host applies these effects to its storage; how it sequences writes, recovers from partial application, or surfaces partial success is host-defined.

Because the engine never writes itself, a "preview-only mode" requires no special flag: the host simply consumes the outcome without applying it. Engine output contains everything needed to render the post-operation state in memory.

## 10. Composition order

Within one operation, predicates compose in this order — each step intersects with the previous:

1. **Filter** (`filter`) — narrows by per-document predicate. Includes both frontmatter predicates (§4) and graph operators (`query-graph-spec.md`). *(all four operations)*

After selection:

2. **Sort** (§6) orders the matched set.
3. **Limit** (§7) caps the matched set.
4. **Action**: `find` projects (§5) and returns matches; `count` returns the integer; `update` applies the update operators (§8) atomically per document and returns the rendered patch (§9); `delete` returns the keys to remove. For mutating actions the host applies the returned effects to its storage.

## 11. Out of scope (v1)

- **Aggregations beyond count.** No sum, avg, min/max, group-by. `count` is the only aggregate.
- **Joins / cross-document references.** Filters operate on a single document at a time.
- **String regex operators.** Not in v1 (§4.10); may land in a future revision.
- **`$elemMatch` (per-element predicate on arrays).** Deferred (§4.9); see `query-language-v2-spec.md` §2.1.
- **`$slice` (computed projection).** Deferred (§5.2); see `query-language-v2-spec.md` §4.1.
- **Compound sort.** v1 accepts exactly one sort key (§6); multi-key tie-breaking ordering is deferred to a future revision.
- **Fuzzy text search.** Not part of the language; it's a CLI / host feature on `iwe find`. The language exposes only `filter`, `project`, `sort`, `limit`, `update`. Graph-topology predicates that used to be CLI-only (e.g. "roots-only") are now part of the language via `query-graph-spec.md`.
- **Concrete reserved pseudo-fields.** No engine-populated fields are defined in v1. The reserved-prefix rule (§2.3) holds the namespace for future amendments.
- **Computed update values.** All operator values are literal YAML scalars or simple structures. No `$now()`, no field-to-field copy, no expression language.
- **Conditional updates per document** (e.g. "set X only if Y > 3 on this doc"). Use a tighter `filter` instead.
- **Across-document atomicity.** Per-document only.
- **Bulk operations on reserved fields.** Renaming, key changes, etc. go through dedicated graph commands, not through update operators.
- **`update` in a `delete` operation.** Delete is filter-driven removal; mutating fields on docs being deleted is incoherent.

## 12. Companion specs

- **Graph operators:** `query-graph-spec.md` — `$`-prefixed operators that extend filter with cross-document selection (`$key`, `$includes`, `$includedBy`, `$references`, `$referencedBy`, `$includesCount`, `$includedByCount`).
- **Grammar reference:** `query-language-grammar.md` — full BNF covering operation documents, filter, projection, sort, limit, update operators, and graph operators.
- **CLI surface:** `query-cli-spec.md` — `iwe find`, `iwe update`, `iwe delete` flags.
- **MCP guide:** `query-language-mcp.md` — combined queries for AI agents.
- **Tree extension:** `tree-spec.md` — `$walk` operator. The `$do` action verbs (`$keep`, `$remove`, `$set`, `$replace`, `$replace_with`) belong to the update context and live in mutation operation documents.
- **v2 deferred operators:** `query-language-v2-spec.md` — `$elemMatch`, `$regex`, `$slice`.
