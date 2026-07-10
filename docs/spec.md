# IWE Query Language Specification

## 1. Overview

This document specifies the IWE query language: a YAML 1.2-based, MongoDB-style language for selecting, shaping, and mutating documents in an IWE workspace. It covers:

- The **corpus model** — what a document is, the edge model, reserved prefixes.
- The four **operations** — find, count, update, delete — and the shape of an operation document.
- The **filter language** — operators, types, composition.
- **Graph operators** — identity and relational walk operators over inclusion and reference edges.
- **Projection** — MongoDB-style output shaping with structural pseudo-field sources.
- **Sort, limit, update operators** — ordering, capping, and mutation constructs.
- The **CLI surface** — flags, lowering rules, deprecated aliases.
- **Output formats** — markdown, keys, JSON, YAML, dot, csv, and mutation prose.
- A **formal grammar** reference (Appendix A).

## 2. Corpus model

### 2.1 Documents

A **document** is the parsed frontmatter of one note. Documents are mappings from string keys to YAML 1.2-typed values: strings, numbers, booleans, null, lists, mappings, dates, datetimes. Under YAML 1.2, bare `yes`, `no`, `on`, and `off` are plain strings, not booleans (unlike YAML 1.1).

Notes with no frontmatter participate in the corpus as documents with an empty mapping (`{}`). They never match presence-style filters like `{status: draft}` but do match `{status: {$exists: false}}`.

### 2.2 The corpus

The **corpus** is every document in the IWE workspace.

### 2.3 Reserved field-name prefixes

Frontmatter field names whose **first character is `_`, `$`, `.`, `#`, or `@`** are reserved by the engine. They are **invisible to user-facing query operations**: filter, sort, and projection paths that reference such names resolve as missing; reserved entries never appear in `find` output (with or without projection); and `update` strips them from each affected document before the new frontmatter is rendered.

A reserved-prefix entry may exist in a file's raw frontmatter on disk — the engine does not refuse to load it — but every user-visible touchpoint (queries, results, mutated output) behaves as if it weren't there. Update writeback is the round-trip moment when such entries are dropped: any document the user mutates loses its reserved-prefix entries on the way out.

User frontmatter field names must not begin with any of the five reserved characters. Any other leading character — letter, digit, hyphen, slash, parenthesis, etc. — is unreserved and addressable as a regular field. Subsequent characters within a name are unconstrained per YAML rules, with one exception: a literal `.` is reserved as a path separator (§4.4) and cannot appear inside a single segment.

Beyond the reserved-prefix and dot rules, a field-path segment used in a filter, projection, sort, or update path must be a **non-empty** string with **no Unicode whitespace** (leading, trailing, or embedded) and **no Unicode control characters**. An empty-string segment, a whitespace-only segment, or a segment containing control characters is a parse-time error. Other characters — digits, hyphens, slashes, parentheses, Unicode letters — are unrestricted.

The reserved prefixes have distinct roles:

- `$`-prefixed names — operator expressions everywhere in the language (`$eq`, `$set`, `$includedBy`, etc.). Never user field names.
- `_`, `.`, `#`, `@` — held for engine use.

This rule is what makes the operator vocabulary safe: `$`-prefixed keys in a filter or update document never collide with a user field of the same name, because such user fields cannot be referenced by query operations.

### 2.4 Edge model

IWE's corpus graph contains two kinds of directed edges between documents:

- **Inclusion edges** — structural transclusion links. When document A includes document B, B's content is rendered inline as part of A.
- **Reference edges** — non-structural links, including inline mentions inside text. A document can reference another without including it.

Both edge kinds form general directed graphs that may contain cycles, including self-loops. Walks over them terminate via visited-set tracking; see §5.5.1.

Both edge kinds are directed. Direction-of-read convention for the relational operators (§5.2):

| Operator | Reads as | This doc → anchor? | Anchor → this doc? |
|---|---|---|---|
| `$includes` | this doc includes an anchor | yes (outbound inclusion) | no |
| `$includedBy` | this doc is included by an anchor | no | yes (inbound inclusion) |
| `$references` | this doc references an anchor | yes (outbound reference) | no |
| `$referencedBy` | this doc is referenced by an anchor | no | yes (inbound reference) |

The "anchor" is one of the documents selected by the operator's argument. Relational operators take a `match` filter that resolves to an anchor set (§5.2); a relational predicate matches when this document stands in the named relation to at least one document in that set.

## 3. Operations and operation documents

### 3.1 Operations

| Operation | Returns / effect |
|---|---|
| `find` | Returns matched documents (subject to `project`, §6). |
| `count` | Returns the integer count of matched documents. |
| `update` | Mutates each matched document by applying an update document (§9) — frontmatter operators, and the block operators of the block surface (§9.5). |
| `delete` | Removes each matched document. |

### 3.2 Operation-document structure

Every operation document is one YAML mapping. Top-level fields:

| Field | Operations | Purpose |
|---|---|---|
| `filter` | all | Predicate document (§4). Required on `update` / `delete`. Graph operators that extend filter with cross-document selection are defined in §5. |
| `search` | find | Relevance selection stage (§5A). Restricts membership to documents matching a `lexical` / `fuzzy` text query and supplies the default ordering. Semantics: [Search](query-language.md#search). |
| `project` | find | Projection (§6). Mutually exclusive with `addFields`. |
| `addFields` | find | Additive projection (§6.3). Mutually exclusive with `project`. |
| `sort` | all | §7. On `update` / `delete`, bounds iteration order before mutation. |
| `limit` | all | §8. On `update` / `delete`, bounds the number of mutated / removed docs. |
| `update` | update | Update document (§9). Required on `update`. |
| `expect` | update, delete | Document-level guard: asserts the number of matched documents the operation will write (post-`limit`), refusing the whole operation on violation. Grammar: §A.7. Semantics: [`expect` guards](query-language.md#expect-guards). |

Operation-inappropriate fields are an error. The valid field set per operation:

| Operation | Allowed fields |
|---|---|
| `find` | `filter`, `search`, `project`, `addFields`, `sort`, `limit` |
| `count` | `filter`, `sort`, `limit` |
| `update` | `filter` (required), `sort`, `limit`, `update` (required), `expect` |
| `delete` | `filter` (required), `sort`, `limit`, `expect` |

E.g. `project` in a `count` / `update` / `delete` operation, `update` in a `find` / `count` / `delete` operation, `search` in a `count` / `update` / `delete` operation, or `expect` in a `find` / `count` operation, are parse-time errors.

`filter` is required on both `update` and `delete` to prevent accidental whole-corpus mutation. The empty filter `{}` matches all documents and must be passed explicitly.

Example — a complete `find` operation document combining selection, projection, sort, and limit:

```yaml
filter:
  $or:
    - $key: projects/alpha
    - $includedBy: { match: { $key: projects/alpha }, maxDepth: 5 }
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
    - $includedBy: { match: { $key: projects/alpha }, maxDepth: 5 }
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
    - $includedBy: { match: { $key: archive/2024 }, maxDepth: 5 }
  status: archived
limit: 500
```

## 4. Filter language

A filter document is a predicate evaluated against each document in the corpus. A document matches when every top-level key matches.

Filter top-level keys are either user frontmatter field names (e.g. `status`, `priority`, `tags`) or `$`-prefixed operator names. The operator family includes the document-level logical operators (`$and`, `$or`, `$nor`; §4.6), the **graph operators** (`$key`, `$includes`, `$includedBy`, `$references`, `$referencedBy`) defined in §5, and the block-membership operator **`$content`** (grammar: §A.3; semantics: [Content membership](query-language.md#content--content-membership)). All of these compose freely with frontmatter predicates under the same algebra. `$not` exists only as a **field-level** operator (§4.6); document-level negation is expressed via `$nor`.

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

A mapping with `$`-prefixed and bare keys at the same level behaves differently depending on **where** the mapping appears:

- **At a document-matching position** — the filter root, every element of `$and` / `$or` / `$nor`, or the `match:` clause of a graph anchor — bare keys and operator keys may freely coexist. They are combined with **implicit AND** (consistent with §4.3 and §4.6).

  ```yaml
  type: tracker
  $or:
    - status: open
    - status: pending
  ```

  is equivalent to

  ```yaml
  $and:
    - type: tracker
    - $or:
        - status: open
        - status: pending
  ```

- **At a field-value position** — inside `{ field: { ... } }` — the mapping's keys must be either *all* `$`-prefixed operators or *all* bare nested-field references. Mixing is an error because a bare key inside a field-value mapping is ambiguous: it could be a nested-field path or an argument to a sibling operator.

  ```yaml
  # OK — operator expression
  author: { $eq: alice }

  # OK — nested fields
  author:
    name: alice
    role: admin

  # ERROR — mixed (is `name` a nested field of `author`, or an argument to `$eq`?)
  author:
    $eq: alice
    name: alice
  ```

  The same rule applies inside the body of a field-level `$not`: `{ field: { $not: { $gt: 5, x: 1 } } }` is rejected for the same reason.

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
  name: alice
```

**Dotted-key shorthand:**

```yaml
author.name: alice
```

Mixing forms in a single filter is allowed:

```yaml
status: draft
author.name: alice
review:
  reviewer: alice
```

Dots inside the key string always denote path separators. **Frontmatter fields whose name contains a literal `.` are not addressable**: neither the dotted shorthand nor the nested-mapping form can reference such a field, because path resolution always splits on `.` after YAML parsing. Quoting the dotted name in the source (`"foo.bar"`) does not change this — the parser still sees a string with a dot and splits it. Document authors should avoid creating field names that contain `.`.

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

Example: filter `author.name: alice` against document `{ author: "alice" }` (where `author` is a string, not a mapping) — the leaf `author.name` is missing; the predicate does not match.

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
- Matches `$exists: true` and `$type: "null"`.
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
| boolean | boolean | `false < true` |

Cross-group comparison is always false (e.g. comparing a number with a string is false; a boolean with a number is false). Null is not orderable; ordering operators against null are always false. Use `$exists` / `$eq: null` to test for null explicitly.

**Temporal values.** YAML date and datetime scalars are stored on the wire as strings — the engine's `Value` type does not carry a distinct temporal variant (§4.8 preserves the `date` / `datetime` names for `$type` matching only). Ordering operators on temporal-shaped values therefore reduce to the **string** group above: lexicographic Unicode-codepoint comparison. For ISO-8601 forms (`YYYY-MM-DD`, `YYYY-MM-DDTHH:MM:SS[Z|±HH:MM]`) lexicographic ordering is equivalent to chronological ordering, which is the only form documents and filters are expected to use. Mixing ISO-8601 with non-ISO date strings produces undefined ordering.

#### Common YAML pitfalls

Filter values are parsed by the YAML resolver before they reach the language. The resolver promotes bare scalars based on their lexical form, which can cause filters to silently never match documents whose stored values have a different type:

| Filter source | Parses as | Document stores | Match? |
|---|---|---|---|
| `modified_at: 2026-01-01` | date scalar | string `"2026-01-01"` | no — date vs string is cross-group, always false |
| `priority: "3"` | string `"3"` | integer `3` | no — string vs number |
| `active: "true"` | string `"true"` | boolean `true` | no — string vs boolean |

When in doubt, quote the value to force the string type, or leave it bare to accept YAML's auto-resolution. Equality is type-strict; there is no implicit coercion. Keep the document and the filter on the same side of the quoting boundary.

### 4.6 Logical operators

Three document-level operators compose filters: `$and`, `$or`, `$nor`. They mirror MongoDB's logical operators and may appear at the filter root or as siblings of bare field keys (§4.2). A fourth operator, `$not`, exists only at the **field level** for negating a single field-value operator expression.

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
- `$and` is **implicit at the top level** — multiple top-level keys in a filter are already ANDed (§4.3). Use explicit `$and` when you need to wrap a sub-expression for use inside `$or` / `$nor`, or when you need to repeat a field name across multiple sub-filters (a YAML mapping cannot have duplicate keys).

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

#### `$nor: [filter1, filter2, ...]`

None of the listed filters may match. Use `$nor` for document-level negation: `$nor: [filter]` reads "documents where `filter` does not match".

```yaml
$nor:
  - status: archived
  - status: deleted
  - tags: spam
```

- Every list element is a filter document.
- A document matches if **every** sub-filter fails to match.
- **Empty list** `$nor: []` is a parse-time error.
- Sub-filters are independent — each is evaluated against the whole document.
- **Missing-field interaction:** a sub-filter that fails because the field is missing counts as a non-match, contributing to a `$nor` match. To require presence and inequality, combine `$exists: true` with the negative predicate: `$nor: [{ reviewed: true }]` matches both "reviewed is false" and "reviewed is missing"; use `reviewed: { $exists: true, $ne: true }` to require the field be present.

#### `$not: { $op: ... }` — field-level only

`$not` is **field-level only** and wraps a single operator expression on the field:

```yaml
priority: { $not: { $gt: 5 } }                   # priority is not > 5 (and is present)
priority: { $not: { $gt: 5, $lt: 10 } }          # NOT (5 < priority < 10)
```

- Body must be an operator expression — a mapping of `$`-prefixed comparison/element operators on the field. Bare nested-field references inside the body are rejected (§4.2).
- Negates the inner field operator(s).
- For document-level negation (negating a whole-document predicate, including a graph anchor or compound `$or`), use `$nor: [filter]` — there is no document-level `$not`. Writing `$not:` at the filter root is a parse-time error with a hint pointing to `$nor`.

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
| `number` | Integers and floats together. |
| `boolean` | `true` / `false`. |
| `null` | Explicit null value. |
| `array` | Sequences (any length, any element type). |
| `object` | Mappings. |
| `date` | YAML date scalars (e.g. `2026-04-26`). |
| `datetime` | YAML timestamp scalars (e.g. `2026-04-26T10:30:00Z`). |

- A field with explicit null matches `$type: "null"` and no other type.
- Missing field does not match any `$type`. Use `$exists: false` for absence.
- The list form is OR over types: `$type: [string, number]` matches if the value is either.
- **Empty list** `$type: []` is a parse-time error.

Type names are matched as YAML strings. To test for the null type, write `$type: "null"` (quoted) — the bare YAML null literal `$type: null` is a parse-time error, because YAML resolves it to the null value rather than to a type-name string. The other names follow the same rule: `$type: number` is accepted because YAML resolves the bare word `number` to the string `"number"`; `$type: True` (which YAML resolves to a boolean) is a parse-time error.

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
#### `$size: N`

Matches when the field's array has exactly N elements.

```yaml
tags: { $size: 0 }                   # no tags
authors: { $size: 1 }                # exactly one author
```

- N must be a non-negative integer (`$size: -1` is an error; `$size: 1.5` is an error).
- Field must be an array; non-arrays and missing fields → false.
- `$size` does not accept ranges: `$size: { $gt: 3 }` is **not** supported.

### 4.10 Filter requirements (use-case checklist)

The language MUST express the following queries directly:

| Question | Filter |
|---|---|
| All drafts | `{status: draft}` |
| Drafts modified this year | `{status: draft, modified_at: {$gte: 2026-01-01}}` |
| Tagged either rust or async | `{tags: {$in: [rust, async]}}` |
| Tagged with both rust and async | `{tags: {$all: [rust, async]}}` |
| Has no tags | `{$or: [{tags: {$exists: false}}, {tags: {$size: 0}}]}` |
| Reviewed but no reviewer | `{reviewed_at: {$exists: true}, reviewed_by: {$exists: false}}` |
| Drafts not by alice | `{status: draft, author: {$ne: alice}}` |
| Recent high-priority | `{$and: [{modified_at: {$gte: 2026-04-01}}, {$or: [{priority: {$gte: 8}}, {tags: urgent}]}]}` |

## 5. Graph operators

Graph operators live inside filter documents alongside frontmatter predicates. They share the predicate algebra of filter: AND-composed at top level, composable under `$and` / `$or` / `$nor`, with the same operator-expression vocabulary as numeric frontmatter fields. Selection by graph relationship and selection by frontmatter content are written in the same filter document, distinguished only by whether the predicate key is a `$`-prefixed graph operator or a user frontmatter field name. The reserved-prefix rule (§2.3) makes this safe: user frontmatter fields cannot begin with `$`.

| Category | Operator | Predicate over... |
|---|---|---|
| Identity (§5.1) | `$key` | the document's own key |
| Relational (§5.2) | `$includes` | the document's outbound inclusion relation to an anchor set |
| Relational (§5.2) | `$includedBy` | the document's inbound inclusion relation to an anchor set |
| Relational (§5.2) | `$references` | the document's outbound reference relation to an anchor set |
| Relational (§5.2) | `$referencedBy` | the document's inbound reference relation to an anchor set |
| Membership | `$content` | the document's blocks — at least one matching the block predicate (grammar: §A.3; semantics: [Content membership](query-language.md#content--content-membership)) |

Unknown `$`-prefixed operator names inside a filter are parse-time errors.

Naming conventions: all operator names are camelCase, `$`-prefixed. The `$`-prefix is reserved for operators that evaluate; **walk parameters and payload field names inside operator arguments are bare-named** (`match`, `maxDepth`, `minDepth`, `maxDistance`, `minDistance`). They are configuration of the operator's walk, not operators in their own right.

### 5.1 Identity operator (`$key`)

`$key` predicates the document's own key.

#### 5.1.1 Argument shape

`$key` accepts either a scalar key (implicit `$eq`) or an operator expression.

```
key_op ::= key | key_expr

key_expr ::=
    { $eq:  key }
  | { $ne:  key }
  | { $in:  [key, key, ...] }    # non-empty array
  | { $nin: [key, key, ...] }    # non-empty array
```

#### 5.1.2 Examples

```yaml
filter:
  $key: notes/foo                              # implicit $eq
  $key: { $eq: notes/foo }                     # explicit
  $key: { $ne: drafts/scratch }                # exclude one
  $key: { $in: [a, b, c] }                     # any of these
  $key: { $nin: [drafts/a, drafts/b] }         # none of these
```

#### 5.1.3 Constraints

- `$key` accepts strings only. Operator expressions on `$key` use the comparison set above; `$gt` / `$gte` / `$lt` / `$lte` are parse-time errors (keys are identifiers, not ordered values).
- Empty `$in: []` and `$nin: []` are parse-time errors.

`$key` has only one role in the language: a top-level filter predicate over this document's own key. It also appears inside the `match` filter of relational operators (§5.2.2), but only because `match` is itself a filter document — there it carries the same semantics as any other filter-level `$key` predicate.

### 5.2 Relational operators

The four relational operators predicate that the document being filtered stands in a graph relation to documents matching an anchor specification. The anchor specification is a `match` filter document — the full filter language, evaluated to select an anchor set.

A **walk** is a BFS traversal from the anchor set over the operator's edge type, bounded by min/max depth (inclusion) or distance (reference). Each node appears at most once in a walk's result; cycles terminate via visited-set tracking. The detailed semantics — set vs path-multiset, BFS vs DFS, anchor exclusion, self-loops — are in §5.5.1.

| Operator | True when this document... | Edge type | Walk parameters |
|---|---|---|---|
| `$includes` | has outbound inclusion edges to anchor docs within bounds | inclusion | `maxDepth`, `minDepth` |
| `$includedBy` | has inbound inclusion edges from anchor docs within bounds | inclusion | `maxDepth`, `minDepth` |
| `$references` | has outbound reference edges to anchor docs within bounds | reference | `maxDistance`, `minDistance` |
| `$referencedBy` | has inbound reference edges from anchor docs within bounds | reference | `maxDistance`, `minDistance` |

`$includes` and `$includedBy` walk only inclusion edges. `$references` and `$referencedBy` walk only reference edges.

#### 5.2.1 Argument shape

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

#### 5.2.2 The `match` field

`match` is a filter document. It accepts the full filter language: bare frontmatter fields, `$`-prefixed filter operators (`$key`, `$or`, `$and`, `$nor`, comparison operators, element operators, array operators), and **nested relational operators**. Nesting allows walks anchored at the result of another walk:

```yaml
$includedBy:
  match:
    $includedBy: { match: { $key: projects/alpha }, maxDepth: 5 }
  maxDepth: 5
```

`match` and the surrounding filter share one definition. Inside `match`, `$key` is the top-level identity operator from §5.1 — it accepts a scalar or any of the §5.1 key expressions (`$in`, `$nin`, `$eq`, `$ne`):

```yaml
$includedBy:
  match:
    $key: { $in: [projects/alpha, projects/beta] }
  maxDepth: 5
```

This subsumes what previous revisions of the spec called "OR-of-anchors" — write the OR inside `match`.

#### 5.2.3 Walk parameters

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

Scalar-key shorthand bypasses the unbounded default and fixes `maxDepth: 1` (or `maxDistance: 1`); see §5.2.1.

Wrong-category walk parameters (`maxDistance` / `minDistance` inside an inclusion-edge operator, or `maxDepth` / `minDepth` inside a reference-edge operator) are parse-time errors.

Value constraints on walk parameters:

- All values are positive integers (≥ 1). Zero, negatives, floats, strings, null, and operator expressions are parse-time errors.
- No `-1` sentinel — absence in the full form is the unbounded signal.

Anchor exclusion: a relational operator never matches a document in its anchor set. `$includedBy: { match: { $key: K }, maxDepth: 5 }` matches the documents that K transitively includes within 5 levels but does not match K itself. More generally, a `match` that selects a set S contributes anchors S, and the walk's matches are documents reached *from* S — never S itself. To include the anchor set in the result, compose at the filter level:

```yaml
$or:
  - $key: projects/alpha
  - $includedBy: { match: { $key: projects/alpha }, maxDepth: 5 }
```

#### 5.2.4 Composition

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

AND of multiple keyed anchors with the same bounds is also expressible by widening `match`:

```yaml
$includedBy:
  match:
    $key: { $in: [projects/alpha, projects/beta] }
  maxDepth: 5
```

#### 5.2.5 Empty argument and unknown fields

The empty mapping `$includedBy: {}` is a parse-time error — `match` is required. A mapping without `match` is also a parse-time error, regardless of which walk parameters are present. The array form `$includedBy: []` is a parse-time error.

The set of recognized keys inside a `relational_obj` is closed: `match`, `maxDepth`, `minDepth`, `maxDistance`, `minDistance`. Any other key — including misspellings (`maxDepht`, `match_`), `$`-prefixed names, and reserved-prefix names — is a parse-time error. Implementations MUST reject unknown keys rather than silently ignoring them; this prevents typos from quietly widening or narrowing the walk.

A `match` filter that selects no documents is well-formed but contributes an empty anchor set; the relational predicate then matches nothing.

### 5.3 Graph operator composition with filter

These operators participate in the filter language's predicate algebra exactly like other operators.

Top-level AND — multiple top-level keys in a filter are AND-composed:

```yaml
filter:
  $key:        { $nin: [drafts/scratch, drafts/temp] }
  $includedBy: { match: { $key: projects/alpha }, maxDepth: 5 }
  status: draft
```

`$and` / `$or` / `$nor` — the logical operators wrap any filter document, including ones containing these operators:

```yaml
filter:
  $or:
    - $key: archive/index
    - $includedBy: archive/index
```

Empty `filter: {}` matches every document.

### 5.4 Worked examples

#### 5.4.1 Identity-based queries

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

#### 5.4.2 Walk-based queries

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

#### 5.4.3 Combined queries

```yaml
# Documents under alpha that reference alice
filter:
  $includedBy: { match: { $key: projects/alpha }, maxDepth: 5 }
  $references: people/alice
```

```yaml
# Documents under alpha, excluding the private namespace
filter:
  $includedBy: { match: { $key: projects/alpha }, maxDepth: 10 }
  $key: { $nin: [projects/alpha/private] }
```

### 5.5 Edge cases

#### 5.5.1 Cycle handling

IWE graphs (both inclusion and reference) are general directed graphs and may contain cycles, including self-loops. All walks — `$includes`, `$includedBy`, `$references`, `$referencedBy`, and any composition thereof — MUST track visited nodes and yield each node at most once per walk. A node already in the walk's visit set is skipped; its outgoing edges are not re-traversed.

Self-edges (a node that includes or references itself) are degenerate cycles of length 1 and follow the same rule.

Visited-set tracking is the primary termination mechanism. `maxDepth` / `maxDistance`, if specified, apply independently as an additional bound and do not substitute for cycle detection.

Implementation requirements:

- **Set semantics** — each node appears at most once per walk result. Path-multiset semantics (the same node appearing once per distinct path that reaches it) is not a valid implementation.
- **Visit-set scope** — per-walk. Each relational operator evaluation maintains its own visit set; visit sets are not shared across nested or composed predicates.
- **Traversal order** — BFS. Depth and distance bounds are well-defined under BFS and ambiguous under DFS.
- **Depth / distance measure** — the shortest path from anchor to candidate (BFS-natural).
- **Anchor self-results** — when a document has a self-edge, it does not appear in its own walk result. Anchor exclusion (§5.2.3) applies regardless of self-edges.

#### 5.5.2 Other edge cases

- **Empty corpus** — every relational predicate matches nothing.
- **Empty anchor set** — a `match` filter that selects no documents (e.g. `match: { $key: typo }` against a corpus with no such key, or `match: { status: nonsense }`) contributes no anchors; the relational predicate matches nothing. Typos and stale references narrow the result rather than failing the operation.
- **Disconnected graph** — walks operate per connected component; a walk anchored at K matches only documents reachable from K within bounds.
- **Anchor exclusion** — a walk never matches a document in its anchor set. Use filter-level `$or` with `$key` (or with another predicate) to include the anchor set in the result.
- **Default walk depth** — scalar-key shorthand fixes `maxDepth: 1` / `maxDistance: 1` (direct edges only). The full mapping form treats omitted `maxDepth` / `maxDistance` as unbounded; omitted `minDepth` / `minDistance` always default to 1.
- **Operators inside `$nor`** — `$nor: [{ $includedBy: { match: { $key: K }, maxDepth: 5 } }]` matches documents that are *not* descendants of K within 5 levels.

## 5A. Search stage (find only)

`search` is a top-level clause of a `find` operation (§3.2). It selects documents by relevance to a full-text query and, unless overridden by `sort`, supplies the result ordering. It is **not** a filter operator and **not** a sort variant: it both restricts membership and orders, jointly with `filter`.

```yaml
search:
  lexical: "broken links cleanup"
filter: { type: note }
limit: 5
```

The `search` document is a mapping with two optional keys:

| Key | Ranker |
|---|---|
| `lexical` | BM25 full-text over title + body (§ [Search Ranking](search-ranking.md)). |
| `fuzzy` | Skim subsequence match over title + key. |

At least one of `lexical` / `fuzzy` must be present; the empty mapping `search: {}` is a parse-time error. When both are present, their rankings are fused with Reciprocal Rank Fusion, exactly as `iwe find --fuzzy --lexical`.

**Selection contract.** The result is defined set-theoretically, with no evaluation order implied: a document is in the result **iff** it matches the search **and** passes the `filter` (search matches ∩ filter matches). Candidates with no BM25 hit / no skim score are dropped — a text query is a filter here, not merely a sort. Over that joint set the default ordering is by relevance (ties by key ascending). No implementation may apply a top-K cutoff before the filter mask: `limit: 5` always means the 5 best documents that pass the filter, never the filtered survivors of a globally-truncated top list.

**Scores use corpus-global statistics.** The BM25 index is fit to the whole workspace; IDF does not change under a filter (standard Lucene-style behavior, never re-fit per query). "Search within a filter" is therefore not identical to searching a corpus containing only the filtered documents.

**`search` + `sort`.** Legal, and not redundant: `search` contributes membership and the default ordering; an explicit `sort` overrides the ordering only. This keeps "the 5 newest documents matching Q" expressible (`search` + `sort: { date: -1 }` + `limit: 5`).

**No searchable terms.** A `lexical` query that reduces to nothing after stop-word removal and stemming matches nothing; the result carries a structured warning (surfaced in-band, like the truncation signal) so callers see why the result is empty.

**Requires the index.** `search` requires the search-indexed graph; running it against a graph built without a search language is an execution-time error naming the missing index.

Grammar: §A.4A. Full semantics: [Search](query-language.md#search).

## 6. Projection

Projection shapes *which fields* a `find` (or, by extension, `retrieve`) result carries, *under which output names*. It uses MongoDB-style `$project` semantics: the **left-hand side is the output field name**, the **right-hand side is the source**.

The motivation: `find` returns metadata and `retrieve` returns content. MongoDB-style projection collapses the two into one query+shape pipeline AND lets the caller pick the output names.

### 6.1 Structural pseudo-field sources

§2.3 reserves field names whose first character is `$`, `_`, `.`, `#`, or `@`. This section defines a concrete set of `$`-prefixed **pseudo-field source selectors** that are addressable as projection sources. They are not addressable in `sort` or `update`. On the filter side the single exception is `$content`, which doubles as the block-membership operator — it takes a block predicate and matches documents containing at least one matching block (grammar: §A.3; semantics: [Content membership](query-language.md#content--content-membership)).

The `$`-prefix is a **source-side marker**, not an output-side marker. It says "this name resolves against the engine, not against user frontmatter." Output names are always bare.

| Source selector | Type | Meaning |
|---|---|---|
| `$key` | string | The document's key. |
| `$title` | string | The document's title. |
| `$titleSlug` | string | Slugified form of `$title`: lowercase, ASCII, whitespace and non-alphanumerics replaced with `-`, leading/trailing `-` trimmed. Derived deterministically from `$title`; no separate corpus storage. |
| `$content` | string | Rendered markdown body, frontmatter stripped. |
| `$frontmatter` | mapping | Full user frontmatter, reserved entries already stripped per §2.3. |
| `$includedBy` | `[EdgeRef]` | Inbound inclusion edges. |
| `$includes` | `[EdgeRef]` | Outbound inclusion edges (= child documents). |
| `$referencedBy` | `[EdgeRef]` | Inbound reference edges (= backlinks). |
| `$references` | `[EdgeRef]` | Outbound reference edges. |

`EdgeRef` is the shape `{ key, title, sectionPath: [string] }` (canonical definition: §13.2.2). `EdgeRef` sub-fields are unprefixed — they are produced by projection (engine output), not addressed as sources.

These source selectors are reserved permanently. User frontmatter fields whose names start with `$` are already forbidden (§2.3), so there is no collision risk between source names and user data.

Consumers MUST tolerate unknown fields in result documents (per the schema-evolution rule in §13.1.3).

**All output keys are bare identifiers chosen by the projection author.** No `$`-prefix on output, ever. The `$`-prefix lives on the right-hand side (source selectors), not on the left (output keys). This applies recursively: `EdgeRef` sub-fields are bare (`key`, `title`, `sectionPath`), every level of the result document uses bare keys.

The visual rule: `$X` in a projection document is a *reference* to engine-side data; bare keys are output names that ship to the consumer.

### 6.2 Projection document

A projection document is a YAML mapping. Each entry has the shape:

```
<outputName>: <source>
```

Where:

- **`outputName`** — a bare identifier. The key under which the field appears in the result document. MUST NOT start with `$` (reserved for source selectors). MUST NOT contain `.` (output is always flat at the top level — nesting is determined by the source's value type). Casing is preserved as written; the default projections (§6.2.2) use camelCase, but user projections may use any casing.
- **`source`** — one of the forms below.

#### 6.2.1 Source forms

| RHS form | Meaning |
|---|---|
| `1` | Include a frontmatter field whose name equals `outputName`. Shorthand for `<outputName>: <outputName>`. `true` and YAML `null` are accepted as aliases of `1`. |
| `$<selector>` | Include the named structural pseudo-field source. |
| `path.to.fm.field` | Include a frontmatter value at a dotted path (per §4.4). |
| `{ $<selector>: { <options> } }` | Options form, used by the block surface: `{ $content: P }` narrows the body to blocks matching the predicate `P`, and the `$blocks` / `$matches` sources take their predicate or pattern this way (grammar: §A.4; semantics: [Block projection](query-language.md#block-projection)). |

Examples:

```yaml
project:
  status: 1                           # frontmatter.status → status
  priority: metadata.priority         # frontmatter.metadata.priority → priority
  body: $content                      # $content → body
  parents: $includedBy                # $includedBy → parents
  links: $references                  # $references → links
  fm: $frontmatter                    # full user frontmatter mapping → fm
```

Result document for the projection above:

```yaml
status: draft
priority: 5
body: "# Doc One\n\n..."
parents:
  - key: parent
    title: Parent Doc
    sectionPath: [Overview]
links: []
fm:
  status: draft
  metadata:
    priority: 5
```

`key` and `title` are absent because the projection does not select them. Add `key: $key` and `title: $title` (or rely on the default projection in §6.2.2) to include them.

#### 6.2.2 Default projection

When the operation document omits `project`, the command applies the **default projection** below. `find` and `retrieve` share the same default:

```yaml
project:
  key: $key
  title: $title
  references: $references
  includes: $includes
  referencedBy: $referencedBy
  includedBy: $includedBy
```

The four edge fields cover both inclusion directions (`includes` / `includedBy`) and both reference directions (`references` / `referencedBy`). User frontmatter is **not** wrapped under a `frontmatter:` key in the default — it is flat-merged at the top level of the result per §13.2.3. Callers who want the whole frontmatter map under a single key can project it explicitly with `addFields: { fm: $frontmatter }` or `project: { ..., frontmatter: $frontmatter }`.

`count` returns an integer and has no projection.

**Frontmatter precedence on name collision.** A default projection entry that maps a structural source onto the output names `key` or `title` does not overwrite a user frontmatter field of the same name: if the document's frontmatter has `key` or `title`, the frontmatter value wins and the structural value is suppressed. The rule is limited to these two names — for the four edge fields (`references`, `includes`, `referencedBy`, `includedBy`) the structural (graph-derived) value is authoritative even when user frontmatter happens to use the same name. The rule also applies to `addFields` (which flat-merges, like the default), but **not** to explicit `project`: an explicit `project` document emits exactly the listed fields with their resolved sources and does not flat-merge user frontmatter at all.

When `project` is set explicitly, it **replaces** the default — there is no merge. A user who wants to extend the default writes the full set:

```yaml
project:
  key: $key
  title: $title
  references: $references
  includes: $includes
  referencedBy: $referencedBy
  includedBy: $includedBy
  body: $content                # added
```

#### 6.2.3 Identity fields

`key` and `title` are not implicit. They appear in the result only when the projection — default or explicit — selects them. The default projection in §6.2.2 includes both, so callers who do not pass `--project` see them without effort. A caller who passes `--project status,priority` gets exactly `status` and `priority`; `key` and `title` are absent unless added.

A projection that maps `$key`, `$title`, or `$titleSlug` under a different output name (e.g. `slug: $titleSlug`, `heading: $title`) emits the alias only — there is no automatic duplicate `key` / `title` field. If the caller wants both the canonical name and an alias, both must be projected. Example:

```yaml
project:
  slug: $titleSlug
  heading: $title

# result:
slug: doc-one
heading: Doc One
```

#### 6.2.4 Conditional structural sources

Some pseudo-field sources require auxiliary graph computation. On `find` (the supported path), projecting any of `$content`, `$includes`, `$includedBy`, `$references`, `$referencedBy` *implies* the corresponding compute — no flags needed. The implied depth for `$includes` is 1 (immediate children only); deeper traversal is currently only available on `retrieve`.

`retrieve` exposes no projection flags (§6.4.1); its output follows the default projection, and the flag set gates which of these sources are computed. When a source's backing flag is not set, the field is still emitted, with its empty value (`[]`):

- `includedBy` is always populated.
- `referencedBy` without `-b` (default on) → `[]`.
- `includes` without `--children` → `[]`.
- `references` without a `references` expansion (`--expand-references N`, or the deprecated `-l`) → `[]`.

The empty form preserves stable schema (§13.1.3).

### 6.3 `addFields` — additive projection

For callers who want to *augment* the default projection rather than replace it, the operation document accepts an alternative key, `addFields`, alongside `project`.

`addFields` follows the same grammar as `project` (§6.2.1). It does **not** replace the default — it extends it. The baseline that `addFields` augments is exactly the default projection from §6.2.2:

```yaml
key: $key
title: $title
references: $references
includes: $includes
referencedBy: $referencedBy
includedBy: $includedBy
```

Combine rule (projection layer — applied before evaluation):

- Each entry in `addFields` is appended to the six default entries above.
- Output names absent from the default are appended.
- Output names that collide with a default-projection output name overwrite the default entry. For example, `addFields: { title: $key }` replaces the default `title: $title` *projection entry*; after this step the merged projection has `title: $key` in place of `title: $title`.

`project` and `addFields` are mutually exclusive within a single operation document. Setting both is a parse-time error.

The conditional-source rule (§6.2.4) applies identically. A structural pseudo-field appearing under `addFields` on `find` implies the corresponding compute, just as it would under `project`.

**Frontmatter precedence at evaluation.** `addFields` flat-merges user frontmatter into the result (like the default projection — see §13.2.3), so the §6.2.2 frontmatter-precedence rule applies: for the output names `key` and `title`, a colliding user frontmatter value wins over the projected structural source.

This means an `addFields` override of `key` or `title` only takes effect on documents whose frontmatter does *not* already define that name. With `addFields: { title: $key }` against a doc whose frontmatter has `title: "User Title"`, the result's `title` is `"User Title"` (frontmatter wins). Against a doc with no `title` in frontmatter, the result's `title` is the doc's key (the override applies). For other output names — including the four default-projection edge fields (`references`, `includes`, `referencedBy`, `includedBy`) — the projected value wins on collision.

This rule does **not** apply under explicit `project`: that mode emits exactly the listed fields with their resolved sources and does not flat-merge user frontmatter, so there is no collision to resolve.

**Tree carve-out.** On `iwe tree`, the output names `key`, `title`, and `children` are reserved by the recursive renderer and cannot be overwritten via `addFields` (or `project`). An `addFields` entry whose output name collides with one of those three is silently ignored on `tree` — the structural value is emitted instead. Other commands (`find`, `retrieve`) follow the unrestricted overwrite rule above.

Example — `find` with the body added:

```yaml
addFields:
  body: $content
```

Result document carries the full `find` default projection (§6.2.2), the document's user frontmatter flat-merged at the top level (per §13.2.3), plus the `body` field:

```yaml
key: doc-1
title: Doc One
references: []
includes: []
referencedBy: []
includedBy: []
status: draft        # user frontmatter, flat-merged
priority: 5          # user frontmatter, flat-merged
body: "# Doc One\n\n..."
```

### 6.4 Output shape under projection

Projection shapes the per-document fields. The wire shape is **a flat array of projected documents** — no envelope. This matches §13.4.2 (`find`) and §13.4.3 (`retrieve`).

```yaml
[<projected-doc>]
```

Each element is a projected document per §6.2.1. The shapes `FindResult` and `DocumentOutput` are the **default-projection** results for `find` and `retrieve` respectively; explicit projection produces whatever shape the projection document specifies.

#### 6.4.1 Cross-command convergence

Once projection is unified, `find` and `retrieve` differ only in **selection vocabulary**: `find` accepts text queries (`--fuzzy`, `--lexical`); `retrieve` accepts `-k KEY` (and graph-walk flags like `--expand-includes`, `--expand-included-by`, `--expand-references`). Default projection is the same on both (§6.2.2), and the wire shape is the same flat array.

`retrieve` deliberately exposes no projection flags — every shaped read is `find`'s (`find -k KEY --project ...`). Under the shared default projection, `find` and `retrieve` produce the same per-document shape — and the same outer shape — for the same key set.

### 6.5 Markdown rendering under projection

With explicit projection:

- **`find` markdown:** emits a compact index — one `- [title](key)` line per result — unless the projection includes a `$content`-shaped field. When `$content` is projected, the whole invocation switches to one four-backtick fenced `markdown #<key>` block per result: the frontmatter inside the block contains the projected fields under their **output names**, with two omissions — `key` is hoisted to the fence info string (never duplicated inside frontmatter), and the `$content` field is rendered as the body rather than inside frontmatter. In index mode, edge fields render as `<-`/`->` arrows and other non-edge fields as ` · name: value` annotations.
- **`retrieve` markdown:** the frontmatter block contains only the fields the projection requested, under their **output names**. Omitting a `$content` projection emits the frontmatter block with no body.

The cross-format invariant in §13.1.1 still applies — markdown MUST NOT contradict JSON/YAML, but MAY abbreviate or omit fields for readability.

## 7. Sort

```yaml
sort:
  modified_at: -1
```

| Value | Meaning |
|---|---|
| `1` | Ascending |
| `-1` | Descending |

The sort direction is type-strict: integer `1` (ascending) or integer `-1` (descending). Floats (`1.0`), strings (`"1"`), booleans, and null are parse-time errors. (YAML `+1` resolves to the same integer as `1` and is accepted.)

A `sort` mapping with two or more entries is a parse-time error.

Documents missing the sort key sort as if the value were null. Null sorts before all other values ascending, last descending. Sort applies to all four operations (on `update` / `delete` it bounds the iteration order before mutation).

Ties — including the no-`sort` case — are broken by document key in ascending lexicographic order. The engine sorts the matched set by key first, then applies the user-provided sort with a stable algorithm; the result is deterministic given the same corpus and operation.

## 8. Limit

A non-negative integer cap.

```yaml
limit: 20
```

`limit: 0` means no limit. Negative values are an error. Limit applies to all four operations; on `update` / `delete` it bounds the number of mutated / removed documents.

## 9. Update operators

The `update` field of a mutation operation document specifies the mutations to apply to each matched document. It must contain at least one update operator at the top level. All operators in one update document apply atomically per matched document (§10).

### 9.1 Frontmatter operators

Two frontmatter operators:

| Operator | Effect |
|---|---|
| `$set` | Set fields to values |
| `$unset` | Remove fields |

#### `$set`

```yaml
update:
  $set:
    reviewed: true
    audited_at: 2026-04-26
    author:
      email: alice@example.com
    "review.reviewer": alice
```

Adds the field if absent, replaces it otherwise. Nested paths can be expressed via nested mappings or dotted-key shorthand (matching §4.4).

Intermediate mappings are auto-created when a dotted path writes through a missing parent: `$set: { "a.b.c": 1 }` on a doc without `a` produces `a: { b: { c: 1 } }`. A dotted path that traverses a present-but-non-mapping intermediate **coerces the intermediate to a fresh mapping** holding the new leaf — `$set: { "a.b": 1 }` against `{ a: "scalar" }` produces `{ a: { b: 1 } }`. The previous scalar value is discarded; the user took explicit action by writing through that path. This is the symmetric write-side rule to §4.4, which on read treats a non-mapping intermediate as a missing leaf.

Mapping values **replace wholesale**. `$set: { author: { name: alice } }` overwrites the existing `author` field with the literal mapping `{ name: alice }`; any pre-existing keys under `author` (e.g. `email`) are dropped. To merge into an existing mapping, address the inner fields with dotted shorthand (`$set: { "author.name": alice }`); per §4.4, that is equivalent in *path* to the nested-mapping form, but the dotted form only writes the named leaves and leaves siblings intact.

`$set` requires at least one entry. Empty `$set: {}` is a parse-time error.

#### `$unset`

```yaml
update:
  $unset:
    draft_notes: ""
    temporary: ""
```

Values are ignored. Absent field → no-op. `$unset` requires at least one entry; empty `$unset: {}` is a parse-time error (same reason as `$set`).

### 9.2 Reserved-prefix protection

Reserved-prefix names (`_`, `$`, `.`, `#`, `@`) are invisible to query operations and are dropped on update writeback — see §2.3. On the mutation side, **operators that target a reserved-prefix segment in any path are parse-time errors**. The check applies to every segment — top-level keys, dotted-shorthand segments, and nested-mapping keys at every depth — not only the leaf or the top-level segment.

```yaml
# ERROR — top-level reserved-prefix names
update:
  $set:
    _hidden: 1
    .secret: 2
    "#tag": foo
    "@user": bar
```

```yaml
# ERROR — dotted segment with reserved prefix
update:
  $set:
    "author._hidden": 1
```

```yaml
# ERROR — reserved prefix on a nested-mapping key (any depth)
update:
  $set:
    author:
      _hidden: 1
```

```yaml
# ERROR — reserved prefix on a leaf segment
update:
  $set:
    "review.@user": alice
```

The error is detected during update-document validation. Without it, a top-level `$set: { _hidden: 1 }` would be silently lost when writeback strips reserved-prefix entries from the rendered frontmatter — the parse-time error makes the failure loud instead. Extending the check to every segment keeps the reserved namespace consistent across the language.

### 9.3 Combining operators

Multiple operators in one update document apply atomically per matched document. `$set` and `$unset` paths are checked for **prefix overlap**: two paths conflict when, after canonicalizing nested-mapping form into dotted form per §4.4, one path is equal to or a prefix of the other.

Conflicts are parse-time errors. The rule applies both across operators (`$set` vs `$unset`) and within a single operator (e.g. two `$set` entries that overlap after canonicalization).

| Update document | Result |
|---|---|
| `$set: { "a.b": 1 }, $unset: { a: "" }` | error — `a` is a prefix of `a.b` |
| `$set: { a: 1 }, $unset: { "a.b": "" }` | error — same prefix relation, opposite direction |
| `$set: { author: { name: alice } }, $set: { "author.name": bob }` | error — both canonicalize to writes overlapping `author.name` |
| `$set: { "a.b": 1 }, $unset: { "a.c": "" }` | OK — sibling paths, no overlap |
| `$set: { a: 1 }, $unset: { b: "" }` | OK — disjoint top-level fields |

### 9.4 Update requirements (use-case checklist)

The language MUST express the following mutations directly:

| Operation | Update document |
|---|---|
| Mark all drafts reviewed | `$set: {reviewed: true}` |
| Promote drafts to published | `$set: {status: published, published_at: 2026-04-26}, $unset: {draft_notes: ""}` |

### 9.5 Block operators

The update document also accepts six **block operators** — `$replace`, `$replaceText`, `$insertBefore`, `$insertAfter`, `$append`, `$delete` — as siblings of `$set` / `$unset`. Each addresses blocks inside the matched documents through a block predicate carried in its argument: the `$`-prefixed keys select, the bare keys are the payload and the optional `expect` guard. Block operators are valid only in `update` operations, combine freely with the frontmatter operators, apply atomically with them per matched document (§10.1), and never conflict with `$set` / `$unset` paths — they mutate different data. Grammar: §A.7 (operators) and §A.9 (block predicates). Semantics — targets and coalescing, header splice behavior, `$replaceText` anchor rules, extent disjointness, guards — are specified in [Block update operators](query-language.md#block-update-operators).

## 10. Atomicity

### 10.1 Per-document

All operators in one update document apply atomically per matched document: either every operator succeeds and the engine emits a single rewritten document — frontmatter and body — for that document, or no replacement is emitted for it. There is no half-applied rewrite. `$set` and `$unset` have no evaluation-time failure modes — their invalid forms are rejected at parse time (`$set` / `$unset` conflict, reserved-prefix paths, etc.) before any matching runs. The block operators (§9.5) do have evaluation-time failure modes — type compatibility, `$replaceText` anchors, extent disjointness, `expect` guards; the operation validates fully against the original documents before anything is emitted, so a failure in any matched document means no document is rewritten and the frontmatter operators write nothing (semantics: [Validation and atomicity](query-language.md#validation-and-atomicity)).

### 10.2 Across-document

Across-document atomicity is **not** provided. The engine itself is a pure function: given an `update` operation it returns `changes` — a list of `(key, new markdown)` pairs the host should write. A `delete` operation returns the list of keys to remove. The host applies these effects to its storage; how it sequences writes, recovers from partial application, or surfaces partial success is host-defined.

Because the engine never writes itself, a "preview-only mode" requires no special flag: the host simply consumes the outcome without applying it. Engine output contains everything needed to render the post-operation state in memory.

## 11. Composition order

Within one operation, predicates compose in this order — each step intersects with the previous:

1. **Filter** (`filter`) — narrows by per-document predicate. Includes both frontmatter predicates (§4) and graph operators (§5). *(all four operations)*
2. **Search** (`search`, §5A) — on `find` only: intersects the filter survivors with the search matches and supplies the default ordering. Selection is joint with step 1 (defined set-theoretically, no order implied). Skipped when `search` is absent.

After selection:

3. **Sort** (§7) orders the matched set. On `find` with `search` but no `sort`, the relevance order from step 2 is the ordering; an explicit `sort` overrides it.
4. **Limit** (§8) caps the matched set.
5. **Action**: `find` projects (§6) and returns matches; `count` returns the integer; `update` applies the update operators (§9) atomically per document and returns the rendered patch (§10); `delete` returns the keys to remove. For mutating actions the host applies the returned effects to its storage.

## 12. CLI surface

This section specifies the `iwe` CLI surface for the four query operations. It covers the flag set that maps each spec operator to a CLI flag, the `--filter` inline expression form, legacy aliases, and how each command lowers its flags into a spec operation document.

### 12.1 Subcommands

| Subcommand | Spec operation | Notes |
|---|---|---|
| `iwe find [QUERY]` | `find` | Combines a text query — `--fuzzy` (title/key) or `--lexical` (BM25 on title and body); the bare positional `QUERY` is a deprecated alias of `--fuzzy` (§12.6) — with filter flags via AND. Supports `--project`, `--sort`, `--limit`, `--blocks`, `--matches`. |
| `iwe count` | `count` | Prints integer matches to stdout. Supports `--limit`. |
| `iwe update` | `update` (mutation mode) | Two modes: body overwrite (`-k -c`) or mutation (`--filter`/`-k` + `--set`/`--unset` and the block-operator flags, §12.4). Modes are mutually exclusive. |
| `iwe delete [KEY]` | `delete` | Positional `KEY` is sugar for `$key: K`. Combine with `--filter` to widen. Either `KEY` or `--filter` is required. |
| `iwe tree`, `retrieve`, `export` | (selection only) | Reuse the same filter flag set to narrow what they operate on. They are not spec operations. |

### 12.2 Filter flags

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

#### 12.2.1 `--filter` lowering

The argument to `--filter` is parsed as a YAML value. If the parsed value is a mapping, it is used directly as a filter document. Otherwise the engine wraps the input in `{` and `}` and re-parses. This lets users write either form:

```
--filter 'status: draft'                # block-style mapping (preferred)
--filter '{status: draft, priority: 5}' # flow-style mapping
--filter '$key: notes/foo'              # graph operator at top level
```

The resulting filter document is parsed by the same builder that handles full operation documents, so all errors defined in §4 (mixed `$`/bare keys in field-value mappings, top-level `$not`, etc.) are surfaced verbatim.

#### 12.2.2 Anchor depth syntax

Inclusion anchors (`--includes`, `--included-by`) accept `KEY[:DEPTH]` where DEPTH is a non-negative integer that becomes `maxDepth`. Reference anchors (`--references`, `--referenced-by`) use the same `KEY[:DIST]` syntax, lowered to `maxDistance`. **DEPTH / DIST `0` is the unbounded sentinel** — see below.

**Default values.** The CLI carries two session-level defaults, both starting at 1:

- `--max-depth N` — applied to inclusion anchor flags (`--includes`, `--included-by`) that omit a per-flag value.
- `--max-distance N` — applied to reference anchor flags (`--references`, `--referenced-by`) that omit a per-flag value.

`0` is the **unbounded sentinel** for both flags and the colon-suffix: passing `--max-depth 0`, `--max-distance 0`, or `KEY:0` lowers to the full form with `maxDepth` / `maxDistance` omitted (the language's "unbounded" form, §5.2.3). This mirrors `limit: 0` in the language (§8). Positive integers behave as today.

A colon-suffix on a single anchor (`--includes KEY:5`) overrides the session default for that anchor only. The lowered shape depends on the effective depth:

- When the effective depth is 1 (the default, with no `--max-depth` / `--max-distance` and no colon-suffix, or an explicit `:1`), a bare `--includes KEY` lowers to **scalar shorthand** `$includes: KEY` — the language defines this as `{ match: { $key: KEY }, maxDepth: 1 }` (§5.2.1).
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

### 12.3 Shape flags

#### 12.3.1 Format flags matrix

| Subcommand | `-f` / `--format` accepted values | Default |
|---|---|---|
| `iwe find` | `markdown`, `keys`, `json`, `yaml` | `markdown` |
| `iwe retrieve` | `markdown`, `keys`, `json`, `yaml` | `markdown` |
| `iwe tree` | `markdown`, `keys`, `json`, `yaml` | `markdown` |
| `iwe export` | `dot` | `dot` |
| `iwe count` | (no format flag — output is always a single integer) | n/a |
| `iwe delete` | `markdown`, `keys` | `markdown` |
| `iwe rename`, `extract`, `inline` | `markdown`, `keys` | `markdown` |

Read-side commands (`find`, `retrieve`, `tree`) share one format set so a query written for one renders the same way under another; `iwe export` emits DOT only (§13.5.2). Mutation commands return a status report and only need `markdown` (human) or `keys` (machine) modes. `count`'s output is the integer match count and admits no format choice.

#### 12.3.2 Projection and sort flags

| Flag | Lowers to | Operations |
|---|---|---|
| `--project f1,f2[,f3]` | `project: { f1: 1, f2: 1, f3: 1 }` | `find`, `tree` |
| `--add-fields f1,f2[,f3]` | `addFields: { f1: 1, f2: 1, f3: 1 }` | `find`, `tree` |
| `--sort field:1`, `--sort field:-1` | `sort: { field: 1 }` / `sort: { field: -1 }` | `find` only |
| `-l, --limit N` | `limit: N` (0 = unlimited, matching §8) | `find`, `count` |
| `--blocks "PRED"` | `addFields: { blocks: { $blocks: PRED } }` | `find` only |
| `--matches PATTERN` | `filter: { $content: { $matches: PATTERN } }` **and** `addFields: { matches: { $matches: PATTERN } }` — a composite lowering: one-flag grep | `find` only |

`--project` and `--add-fields` accept two argument forms:

- **Comma list:** `--add-fields body=$content,parents=$includedBy`
- **Inline YAML mapping:** `--add-fields 'body: $content'` or `--add-fields '{body: $content, parents: $includedBy}'`

The argument is parsed as YAML first; if it is a mapping, it is used as the projection document directly — this form carries the full §6 grammar, including the block sources and, on `--project`, the top-level block-predicate form (§A.4). Otherwise it is treated as a comma list. This mirrors the `--filter` lowering in §12.2.1.

**Comma-list form.** Each `ITEM` lowers to a single `<outputName>: <source>` entry:

| ITEM form | Lowered entry | Notes |
|---|---|---|
| `name` | `name: 1` | Frontmatter field, output as `name`. |
| `name=path.to.fm` | `name: path.to.fm` | Frontmatter at dotted path, output as `name`. |
| `name=$selector` | `name: $selector` | Pseudo-field source, output as `name`. |
| `$selector` | `selector: $selector` | Pseudo-field, output name = selector minus `$`. Convenience form. |
| `$blocks` / `name=$blocks` | `blocks: { $blocks: {} }` / `name: { $blocks: {} }` | Block source with the empty predicate — every block. A parameterized predicate (or the `$content` / `$matches` block sources) requires the mapping form. |

`--project` and `--add-fields` are mutually exclusive on a single invocation. Passing both is a CLI parse error, mirroring the document-level rule in §6.3.

`--sort` accepts exactly one `field:DIR` pair, matching the single sort key rule (§7).

**Shell quoting.** The `$`-prefix in source selectors triggers shell variable expansion in unquoted form. Quote `--project` and `--add-fields` arguments with single quotes: `--add-fields 'body=$content,parents=$includedBy'` or `--add-fields 'body: $content'`. Bash, zsh, fish, and PowerShell all preserve `$` inside single quotes.

#### 12.3.3 `iwe retrieve` flags

`retrieve` assembles reading context around a set of documents. It runs as **two internal queries** when a search flag is present: a **seed query** (a `find` with `search` and `limit` over the resolved candidate set) followed by the **expansion read** over the ordered seeds. Without a search flag, the seed set is `-k` / `--filter` / anchors directly.

**Expansion (`--expand-*`).** Expansion directions use the standard edge vocabulary — `includes`, `includedBy`, `references`, `referencedBy` — one direct flag each: `--expand-includes`, `--expand-included-by`, `--expand-references`, `--expand-referenced-by`. Each takes an optional depth:

```
--expand-references 1 --expand-included-by 1
--expand-includes 2
--expand-includes            # bare flag = 1 level
```

A bare flag means depth `1`; an explicit value is an integer depth with **`0` = unbounded**; an omitted flag is not followed. A negative or non-integer value is an error. Each direction pulls documents into the result set: `includes` = inclusion descendants, `includedBy` = inclusion ancestors, `references` = outbound reference walk, `referencedBy` = inbound reference walk. **Expansion is doc-only by default** — with no `--expand-*` flag and no legacy alias, `retrieve` returns the requested document(s) with no expansion.

**Edge-list output toggles.** Separate from expansion; they populate the output edge arrays and never add documents. `includedBy` is always populated; the others follow their flag (§6.2.4). When a flag is omitted the field is still emitted with its empty value (`[]`).

| Flag | Effect |
|---|---|
| `-k, --key KEY` | Repeatable. The set of root keys to retrieve (candidate restriction when a search flag is present). |
| `--expand-includes [N]` / `--expand-included-by [N]` / `--expand-references [N]` / `--expand-referenced-by [N]` | Expansion depth per direction (bare = `1`, `0` = unbounded, omitted = not followed). |
| `--lexical Q` | Seed search: BM25 full-text query. Switches `retrieve` to the search-indexed loader and runs the seed query. |
| `--fuzzy Q` | Seed search: fuzzy query on title + key. `--lexical` + `--fuzzy` fuse with RRF. |
| `--limit N` | Cap the seed count **before** expansion — top-N by relevance when searching, the first N of the selection otherwise (`0` = unlimited). |
| `--max-documents N` | Cap the document count **after** expansion, trimming periphery documents first (`0` = unlimited). |
| `-b, --backlinks` | Populate `referencedBy` on each `DocumentOutput` (default true). Edge-list toggle, not expansion. |
| `--children` | Populate `includes` on each `DocumentOutput`. Edge-list toggle, not expansion. |
| `-e, --exclude KEY` | Repeatable. Skip these keys when assembling the result. |
| `--filter`, `--includes`, `--included-by`, `--references`, `--referenced-by`, `--max-depth`, `--max-distance` | Same selection-side filter flags documented in §12.2; restrict the candidate set. With a search flag, search runs **within** these candidates, so listed `-k` keys that do not match the query drop from the output. |

**Output order.** Seeds first, in relevance order, then expansion; seeds survive token-budget trimming first (§12.3.4). Because search restricts, `-k a -k b -k c --lexical Q` searches within those keys — `-k` is candidate restriction, not a guaranteed read.

#### 12.3.4 Token budgets (`iwe find`, `iwe retrieve`)

Two output-budget flags cap how much rendered content the command emits. They are host-side output shaping, not part of the operation document: they do not affect which documents match, only how much of each body is printed.

| Flag | Effect |
|---|---|
| `--max-tokens N` | Cap total content tokens across all results (`0` = unlimited). |
| `--max-document-tokens N` | Cap content tokens per document (`0` = unlimited). |

When a budget clips output, the command prints a truncation warning to stderr naming the limits that apply. `$blocks` / `$matches` entries are exempt from both budgets; a narrowed `$content` field is counted and capped like a full body (§13.6.1).

### 12.4 Update flags (`iwe update` mutation mode)

| Flag | Lowers to |
|---|---|
| `--set FIELD=VALUE` | `$set: { FIELD: VALUE }` (repeatable) |
| `--unset FIELD` | `$unset: { FIELD: "" }` (repeatable) |
| `--replace "ARG"`, `--replace-text "ARG"`, `--insert-before "ARG"`, `--insert-after "ARG"`, `--append "ARG"`, `--delete "ARG"` | one block-operator entry (`$replace`, `$replaceText`, `$insertBefore`, `$insertAfter`, `$append`, `$delete`) in the update document; `ARG` is the operator's `{ <selector>, <payload> }` mapping (§A.7), the operator-name wrapper dropped. Each flag appears at most once. |
| `--expect VAL` | the document-level `expect` clause (§3.2) |
| `--strict` | surface policy, not grammar: refuses to run unless every mutating application carries its `expect` guard (document-level `--expect` plus each block operator's `expect`); exempt under `--dry-run` |
| `--filter "EXPR"` | required if `-k` is omitted |
| `--dry-run` | preview only; print the would-be changes per doc and exit |
| `--quiet` | suppress progress output |

`--set FIELD=VALUE` parses VALUE as a YAML scalar. `5` is an integer, `true` is a bool, `draft` is a string, `[a, b]` is a list. To force a string, quote it as YAML: `--set 'count="5"'`.

`iwe update` does not prompt for confirmation. The caller is responsible for passing the right `--filter` / `-k` selector; use `--dry-run` to inspect the matched set before applying. This matches the engine contract in §10.2 — the engine returns the patch and the host writes it without further interaction.

Body-overwrite mode (`-k KEY -c CONTENT`) is the existing single-doc body rewrite. It does not touch frontmatter and is not a spec `update` operation. Body and mutation flags cannot be combined in one invocation. `--dry-run` applies to both modes.

### 12.5 Delete flags (`iwe delete`)

| Flag | Lowers to |
|---|---|
| Positional `KEY` | `$key: K` (sugar) |
| `--filter "EXPR"` | inline filter |
| `--expect VAL` | the document-level `expect` clause (§3.2) |
| `--strict` | requires `--expect`; refuses to run without it; exempt under `--dry-run` |
| `--dry-run` | preview |
| `-f, --format markdown\|keys` | output format (default `markdown`); `keys` prints affected document keys, suppresses progress |
| `--quiet` | suppress progress |

The deprecated alias `--keys` (also accepted on `rename`, `extract`, `inline`) lowers to `-f keys`; see §12.6.

Either `KEY` or `--filter` (or both) must be present, matching the spec's required-filter rule (§3.2). When both are given, the union is deleted. `iwe delete` does not prompt; use `--dry-run` to preview before applying. Reference cleanup runs once over the whole matched set.

`-f keys` returns *affected* keys (the deleted target plus every doc whose references were rewritten) rather than *matched* keys. The same `-f markdown|keys` selector is also available on `iwe rename`, `iwe extract`, and `iwe inline` with identical semantics.

### 12.6 Deprecated aliases

These flags predate the language and remain accepted on the commands they originally appeared on. Selector aliases (`--in`, `--refs-to`, etc.) print a one-line `warning: --X is deprecated; use --Y` to stderr **each time the deprecated flag appears in a parsed command**. For one-shot CLI invocations this is one warning per run. For long-running hosts (LSP, MCP), the warning fires on every operation that uses the alias — making the deprecation visible across many requests instead of being suppressed after the first. Mutation-output aliases (`--keys`) are silent — `--keys` and `-f keys` behave identically and produce the same output.

| Deprecated | Lowers to |
|---|---|
| bare positional `QUERY` (on `find`) | `--fuzzy QUERY` (prints a deprecation warning; use `--lexical QUERY` for BM25 full-text) |
| `--in KEY[:N]` | `--included-by KEY[:N]` |
| `--in-any K1 --in-any K2` | `$or: [{ $includedBy: K1 }, { $includedBy: K2 }]` (scalar shorthand for each) |
| `--not-in KEY` | `$nor: [{ $includedBy: KEY }]` (scalar shorthand) |
| `--refs-to KEY` | `$or: [{ $includes: KEY }, { $references: KEY }]` (scalar shorthand; legacy mixed-edge) |
| `--refs-from KEY` | `$or: [{ $includedBy: KEY }, { $referencedBy: KEY }]` (scalar shorthand; legacy mixed-edge) |
| `--keys` (on `delete`, `rename`, `extract`, `inline`) | `-f keys` |
| `-d, --depth N` (on `retrieve`) | `--expand-includes N` (legacy `0` = off, not unbounded) |
| `-c, --context N` (on `retrieve`) | `--expand-included-by N` (legacy `0` = off) |
| `-l, --links` (on `retrieve`) | `--expand-references 1` |

The mixed-edge lowering of `--refs-to` / `--refs-from` preserves their pre-spec semantics (matching either inclusion or reference edges to the target). New code should pick the spec operators directly.

On `retrieve`, `-d` / `-c` / `-l` keep their legacy **`0` = off** meaning (they never expressed unbounded), whereas the `--expand-*` flags treat `0` as the unbounded sentinel. Passing a legacy alias together with the `--expand-*` flag for the same direction is an error.

`--in`, `--in-any`, and `--not-in` defaulted to depth 1 before the spec, matching the new `--included-by` default (§12.2.2); the lowering above is behavior-preserving. Use `--max-depth N` or a per-flag colon-suffix to widen.

### 12.7 Composition rules

Within a single command:

1. All filter flags at the top level are AND-composed. The text query (`--fuzzy` / `--lexical` on `iwe find`, or the deprecated bare positional) is also ANDed: the result is the **set intersection** of the text-match set and the filter-match set. Order of evaluation is implementation-defined (typically the more selective predicate is applied first for performance), but the result set is order-independent.
2. `--filter "EXPR"` contributes its top-level filter document to the same AND.
3. `-k` / positional `KEY` participate in the AND like any other clause.
4. `--sort`, `--limit`, `--project` apply after filtering, in the order defined by §11.

**`-k` / `$key` collision.** Combining `-k KEY` with a `--filter` whose top level contains a `$key` predicate is a CLI parse-time error: both clauses contribute to the document's key predicate, and silently AND-composing them would either produce a YAML mapping with two `$key` keys or quietly match the empty set. The error message points users at OR-composition (`--filter '$or: [{$key: a}, {$key: b}]'`) when they wanted a multi-key match, or at picking one source when they didn't. Multi-key match via `-k a -k b` (which lowers to `$key: { $in: [a, b] }`) remains valid.

For OR or NOR compositions, write the filter inside `--filter`:

```
--filter '$or: [{ status: draft }, { status: review }]'
--filter '$nor: [{ status: archived }]'
```

### 12.8 CLI examples

#### 12.8.1 Find

```
iwe find --fuzzy rust                               # fuzzy on "rust" (title/key)
iwe find --lexical "borrow checker"                 # BM25 full-text on title and body
iwe find --filter 'status: draft'                   # all drafts
iwe find --fuzzy rust --filter 'status: draft'      # fuzzy AND status==draft
iwe find --included-by projects/alpha:5             # descendants within 5 levels
iwe find --included-by projects/alpha:0             # all descendants of alpha (unbounded)
iwe find --references people/alice                  # docs that reference alice
iwe find --filter 'priority: { $gt: 3 }' --sort modified_at:-1
iwe find --project title,modified_at -f json        # only project two fields
iwe find --project title,modified_at -f yaml        # same, as YAML
iwe find --filter 'status: draft' --add-fields body=$content
iwe find --add-fields 'body=$content,parents=$includedBy' -f json
iwe find --filter 'status: draft' --project '$content,$includedBy'
iwe find --filter 'status: draft' --project 'body=$content,parents=$includedBy,status'
iwe find --filter 'status: draft' --project 'key=$key,title=$title,body=$content'
iwe find --filter 'status: draft' --add-fields 'body: $content'
iwe find --add-fields '{body: $content, parents: $includedBy}' -f json
iwe find --project '{key: $key, title: $title, body: $content}'
```

#### 12.8.2 Count

```
iwe count                                           # total documents
iwe count --filter 'status: draft'                  # count drafts
iwe count --included-by projects/alpha:10           # count descendants of alpha
```

#### 12.8.3 Update

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

# Block mutation — retitle one section, guarded to a single target
iwe update -k projects/roadmap --replace '{ $header: Goals, content: "## Goals 2026", expect: 1 }'

# Strict bulk edit: every mutating application must carry its guard
iwe update --filter 'status: draft' --strict --expect 3 --set status=published
```

#### 12.8.4 Delete

```
iwe delete document-key                             # single doc
iwe delete --filter 'status: archived'              # bulk delete by filter
iwe delete --filter 'status: draft' --strict --expect '{ max: 5 }'   # guarded bulk delete
iwe delete --filter '$key: drafts/scratch' --dry-run # preview a deletion by filter
```

## 13. Output formats

### 13.1 Format invariants

#### 13.1.1 Cross-format invariant

A given query MUST encode the same logical result data in JSON and YAML. JSON and YAML are isomorphic at the field level. Markdown is a human projection of the same data and may omit detail for readability, but MUST NOT contradict JSON/YAML. `keys` is a strict projection: one key per line, no header.

#### 13.1.2 Field-name convention

| Surface | Convention |
|---|---|
| JSON keys | `camelCase` |
| YAML keys (top-level output and frontmatter inside markdown) | `camelCase` (same as JSON) |
| Markdown body text (headers, prose) | sentence case as authored |

A single name per concept across all surfaces avoids triple-naming. Pick one external name per field and use it everywhere.

#### 13.1.3 Stable-schema rule

Every field this spec lists for JSON/YAML output MUST be present in every emission of that command, with its declared type. Empty values are encoded explicitly:

| Type | Empty encoding |
|---|---|
| array | `[]` |
| mapping | `{}` |
| string | `""` |
| nullable scalar | `null` |

A consumer MUST NOT need to test for key presence to handle the empty case. This applies to `find` (`includedBy: []`), `retrieve` (`includedBy: []`, `includes: []`, `referencedBy: []`), `tree` (`children: []`), and all structured output. Note that with `--project`, only the listed fields are emitted — the stable-schema rule applies to fields the spec declares for the command, not to fields the user excluded by projection.

Markdown is a human surface and is exempt — see §13.3.8.

### 13.2 Common shapes

#### 13.2.1 `KeyTitleRef`

```yaml
{ key: string, title: string }
```

Used wherever one document references another *by identity* without carrying any positional context.

#### 13.2.2 `EdgeRef`

```yaml
{ key: string, title: string, sectionPath: [string] }
```

`KeyTitleRef` extended with `sectionPath`: the chain of section header texts (root-to-leaf) under which the edge appears in the source document. Empty array when the edge sits at the document root.

Used for every inclusion or reference edge surfaced in structured output: `includedBy`, `includes`, `referencedBy`.

#### 13.2.3 User frontmatter merging

`find` and `tree` flatten user frontmatter into each result/node alongside system fields (`key`, `title`, etc.) — there is no nested `frontmatter` object. Reserved-prefix entries (`_`, `$`, `.`, `#`, `@` per §2.3) are stripped before merging and MUST NOT appear in any output. On collision between a user frontmatter field named `key` or `title` and the corresponding system field, the user frontmatter value wins (§6.2.2). For the four edge fields (`references`, `includes`, `referencedBy`, `includedBy`), the structural value is authoritative even on name collision.

`--project f1,f2,...` restricts the result to the listed fields, in the listed order; system fields and user frontmatter fields are projectable interchangeably. Without `--project`, every system field plus every user frontmatter field is flat-merged into the result. With `--project`, no flat-merge happens — only the listed fields appear, each resolved from the source on its right-hand side. `--add-fields` flat-merges like the default; the `key`/`title` precedence rule above applies to it as well.

### 13.3 Markdown and keys

#### 13.3.1 Format set

Commands that accept `-f markdown` or `-f keys`:

| Command | `markdown` | `keys` | Default |
|---|---|---|---|
| `iwe find` | ✓ | ✓ | `markdown` |
| `iwe retrieve` | ✓ | ✓ | `markdown` |
| `iwe tree` | ✓ | ✓ | `markdown` |
| `iwe stats` (aggregate) | ✓ | — | `markdown` |
| `iwe delete`, `rename`, `extract`, `inline` | ✓ | ✓ | `markdown` |

Markdown is a human surface and may omit detail (e.g. counts, edge metadata) for readability — but it MUST NOT contradict the structured form for the same query. `keys` is a strict projection: one key per line, no header, no trailing blank.

#### 13.3.2 Common markdown frontmatter shapes

`EdgeRef` inside markdown frontmatter:

- Always emits `key`, `title`.
- Emits `sectionPath` only when non-empty (the human surface tolerates omission; JSON/YAML always include it).

User frontmatter inside markdown documents has reserved-prefix entries already stripped — they MUST NOT appear in any output.

#### 13.3.3 `iwe find` (markdown / keys)

**Markdown:** `find` markdown is a compact index — one `- [title](key)` line per result, meant to be scanned. No header line. No `(showing M)` / `for "Q"` annotation — counts and query metadata live in `-f json|yaml`. Result order matches the find result order (filter / sort / limit applied upstream).

Each line is `- [title](key)`, where the link text falls back to the key string when `title` is not projected. Non-empty edge fields render after the link as arrows keyed on direction — incoming (`includedBy`, `referencedBy`) as `<- [Title](key)`, outgoing (`includes`, `references`) as `-> [Title](key)`, multiple targets comma-joined after one arrow, empty edges omitted. Other non-edge fields render as ` · name: value`; arrays of scalars join with `, `, and richer values fall back to compact inline YAML.

Projecting a `$content`-shaped field switches the whole invocation to the fenced-block form that `retrieve` emits (byte-identical for the matched key set): each result is a four-backtick fenced block with info string `markdown #<key>`, body and frontmatter as defined in §13.3.4. This is the only way `find` markdown emits document bodies — see §6.5.

When the result set is empty, stdout is empty.

**Keys:** One key per line, no header, no blank lines, no trailing blank.

#### 13.3.4 `iwe retrieve` (markdown / keys)

**Markdown:** Each returned document is wrapped in a **four-backtick fenced code block** with the info string `markdown #<key>`. Inside the fence:

1. A YAML frontmatter block — flat, no wrapper key. The structure mirrors the JSON `DocumentOutput` shape (§13.4.3), minus the fields lifted out: `key` lives in the fence info string, and the body lives below the frontmatter, not as a `content:` field.
2. A blank line.
3. The rendered markdown content.

The fence info string carries the key prefixed with `#` (e.g. `markdown #child`). If the embedded content contains a four-or-more-backtick fence, use one more backtick for the outer fence so the inner fence cannot terminate it.

Frontmatter fields (rendered in this order):

| Location | Field | Required? | Source |
|---|---|---|---|
| fence info | `key` | always | `DocumentOutput.key` |
| frontmatter | `title` | always | `DocumentOutput.title` |
| frontmatter | `references` | omit when empty | `EdgeRef` list |
| frontmatter | `includes` | omit when empty | `EdgeRef` list |
| frontmatter | `referencedBy` | omit when empty | `EdgeRef` list |
| frontmatter | `includedBy` | omit when empty | `EdgeRef` list |

The order matches the unified default projection (§6.2.2). When `find` markdown is in fenced-block form (a `$content`-shaped field is projected), it produces byte-identical frontmatter to `retrieve -f markdown` for the same key; otherwise `find` renders the compact index (§13.3.3).

`EdgeRef` inside frontmatter omits `sectionPath` when empty. There is no `document:` wrapper map; the frontmatter is always flat. Field naming matches JSON (no `parents` / `back-links` aliases).

`````
````markdown #child
---
title: Child Document
includedBy:
  - key: parent
    title: Parent Document
    sectionPath:
      - Overview
---

# Child Document

Child content.
````
`````

A multi-document stream concatenates blocks separated by **exactly one blank line** between the closing fence of one block and the opening fence of the next; no trailing blank line after the final block:

`````
````markdown #doc-a
---
title: Doc A
---

# Doc A

Body.
````

````markdown #doc-b
---
title: Doc B
---

# Doc B

Body.
````
`````

**Keys:** One key per line, in the order documents appear in the envelope. No header.

#### 13.3.5 `iwe tree` (markdown / keys)

**Markdown:** Nested unordered list, two-space indent per depth level, each entry as a markdown link `[<title>](<key>)`:

```
- [AI Agent Memory](ai-memory)
  - [Post One](post-1)
  - [Post Two](post-2)
```

**Keys:** One key per line, **tab-indented** by depth (root has zero tabs). Order matches a depth-first walk of the tree.

```
ai-memory
    post-1
    post-2
```

#### 13.3.6 `iwe stats` (markdown)

Aggregate `markdown` is a human-readable report of corpus-level statistics. Per-document stats (`-k KEY`) does not produce markdown — see §13.4.5 for the per-doc shape.

#### 13.3.7 Mutation commands (markdown / keys)

`delete`, `rename`, `extract`, `inline` produce a status report and accept `-f markdown|keys`.

**Markdown — status report:** A human-prose report of what changed:

```
<verb-ing> '<source-key>'[ to '<target-key>']
Updated N document(s)
```

Or, with `--dry-run`:

```
Would <verb> '<source-key>'[ to '<target-key>']
Would update N document(s)
  <key>
  <key>
```

`--quiet` suppresses both forms.

**Keys — affected-keys list:** One key per line, no header. The list contains every key that was modified — the operation's primary target plus every document whose references were rewritten. With `--dry-run`, the list is the keys that *would* be modified.

#### 13.3.8 Stable-schema exemption for markdown

The stable-schema rule in §13.1.3 requires every declared field to appear in every emission. Markdown is a human surface and is exempt:

- Renderer MAY omit `includedBy` / `includes` / `referencedBy` from frontmatter when empty.
- `EdgeRef` inside frontmatter MAY omit `sectionPath` when empty.
- `key` is always carried in the fence info string; `title` is always in frontmatter.

`keys` output is a strict projection — one key per line, no envelope — so the rule does not apply.

### 13.4 JSON and YAML

JSON and YAML are isomorphic — same keys, same values, same nesting; only the surface syntax differs. They are the authoritative wire shape: when markdown disagrees, JSON/YAML wins.

#### 13.4.1 Per-command format matrix

| Command | `json` | `yaml` | Default |
|---|---|---|---|
| `iwe find` | ✓ | ✓ | `markdown` |
| `iwe retrieve` | ✓ | ✓ | `markdown` |
| `iwe tree` | ✓ | ✓ | `markdown` |
| `iwe stats` | ✓ | ✓ | `markdown` |

`iwe count` and the mutation commands have no `-f json|yaml` form (see §13.5).

#### 13.4.2 `iwe find` (JSON / YAML)

The top-level value is an **array** of `FindResult` — no envelope.

```yaml
[FindResult]
```

A `FindResult` is a flat mapping. There is no nested `frontmatter` object: user frontmatter fields are siblings of `key`, `title`, and the system-derived edge arrays. Without `--project` the result carries the full set of system fields plus all user frontmatter fields. With `--project` the result carries only the listed fields, in the listed order.

System fields (the full set emitted without `--project`):

```yaml
FindResult:
  key:           string
  title:         string
  references:    [EdgeRef]   # always present; [] when none
  includes:      [EdgeRef]   # always present; [] when none
  referencedBy:  [EdgeRef]   # always present; [] when none
  includedBy:    [EdgeRef]   # always present; [] when none
```

Reserved-prefix entries are stripped from user frontmatter before merging and MUST NOT appear in output. On collision between a system field and a user frontmatter field of the same name, the user frontmatter value wins.

**JSON** — without `--project`, system fields and the document's user frontmatter are flat-merged at the top level (`status`, `priority` below come from frontmatter):

```json
[
  {
    "key": "doc1",
    "title": "Document One",
    "references": [],
    "includes": [],
    "referencedBy": [],
    "includedBy": [],
    "status": "draft",
    "priority": 5
  }
]
```

Trailing newline after the closing `]`.

**YAML:**

```yaml
- key: doc1
  title: Document One
  references: []
  includes: []
  referencedBy: []
  includedBy: []
  status: draft
  priority: 5
```

#### 13.4.3 `iwe retrieve` (JSON / YAML)

The top-level value is an **array** of `DocumentOutput` — no envelope.

```yaml
DocumentOutput:
  key:           string
  title:         string
  content:       string         # rendered markdown body only — source-file YAML frontmatter is always stripped
  references:    [EdgeRef]      # always present; populated when --links; [] otherwise
  includes:      [EdgeRef]      # always present; populated when --children; [] otherwise
  referencedBy:  [EdgeRef]      # always present; populated when --backlinks; [] otherwise
  includedBy:    [EdgeRef]      # always present; [] when none
```

**JSON:**

```json
[
  {
    "key": "test-doc",
    "title": "Test Document",
    "content": "# Test Document\n\nContent here.\n",
    "references": [],
    "includes": [],
    "referencedBy": [],
    "includedBy": []
  }
]
```

**YAML:**

```yaml
- key: test-doc
  title: Test Document
  content: |
    # Test Document

    Content here.
  references: []
  includes: []
  referencedBy: []
  includedBy: []
```

#### 13.4.4 `iwe tree` (JSON / YAML)

The top-level value is an **array** of root nodes (not wrapped in an object):

```yaml
TreeNode:
  key:      string
  title:    string
  children: [TreeNode]   # always present in spec; [] when leaf
```

Each `TreeNode` is a flat mapping with the same projection semantics as a `FindResult` (§13.4.2). Without `--project`, only the system fields (`key`, `title`, `children`) are emitted. With `--project`, the listed user frontmatter fields are added as siblings of `key` and `title`, in projection order; `children` is always present regardless of projection so the tree shape remains traversable.

**JSON:**

```json
[
  {
    "key": "ai-memory",
    "title": "AI Agent Memory",
    "children": [
      {
        "key": "post-1",
        "title": "Post One",
        "children": []
      }
    ]
  }
]
```

**YAML:**

```yaml
- key: ai-memory
  title: AI Agent Memory
  children:
    - key: post-1
      title: Post One
      children: []
```

#### 13.4.5 `iwe stats` (JSON / YAML)

Two modes: aggregate (no `-k`) and per-document (`-k KEY`).

**Aggregate** — `json` and `yaml` serialize the `GraphStatistics` struct directly. Aggregate `markdown` is in §13.3.6; aggregate `csv` is in §13.5.3.

**Per-document** — `-k KEY` emits a single object/document. With `-f yaml`, YAML; with `-f json`, JSON. Per-doc format is restricted to `json|yaml`.

### 13.5 Other formats

#### 13.5.1 `iwe count` — bare integer

```
25
```

A single integer followed by a newline. No format flag, no envelope. Empty corpus returns `0`. Stderr carries errors as usual; stdout is exactly the integer plus newline.

#### 13.5.2 `iwe export` — `dot`

`-f dot` only. The output is a graphviz DOT document; its internal grammar is owned by graphviz. The envelope is whatever `dot_exporter::export_dot` produces. `--include-headers` switches to a denser variant (`dot_details_exporter::export_dot_with_headers`) but stays valid DOT.

This spec does not pin the internal DOT grammar. Consumers should pass the output to a DOT-aware tool unmodified.

#### 13.5.3 `iwe stats` — `csv`

Aggregate mode (no `-k`) emits one row per document with `GraphStatistics::export_csv` headers.

Per-document mode (`-k KEY`) does not produce csv — it falls through to JSON in the current implementation; §13.4.5 proposes restricting per-doc output to `json|yaml` at parse time.

#### 13.5.4 Mutation commands — prose status

`update`, `attach`, `new`, `init`, `normalize`, `squash` (and the rest of the create-family) emit a fixed prose status line. They have no format flag because the operation has nothing structured to report.

```
Updated '<key>'
Updated N document(s)
Created '<key>'
Renamed '<old>' to '<new>'
```

`--quiet` suppresses the status line. `--dry-run` (where supported) prefixes with `Would `.

### 13.6 Flag effects on output shape

Flags that change the *shape* (not just the selection) of output. Selection-only flags (filter, sort, limit, anchors) are in §12.2.

#### 13.6.1 `iwe find`

| Flag | Effect on shape |
|---|---|
| `--project f1,f2,...` | `markdown` renders the compact index (one `- [title](key)` line per result) unless a `$content`-shaped field is projected, in which case it switches to one fenced block per result — the frontmatter carries the projected fields under their output names, with `key` lifted to the fence info string and the `$content` field rendered as the body. `keys` is unaffected. JSON/YAML: each `FindResult` carries only the listed fields, in the listed order. |
| `--add-fields f1,f2,...` | Same as `--project` — additive over the default projection in structured output (§6.3); `keys` ignores it. JSON/YAML: each `FindResult` carries the default projection plus the listed fields. |
| `--blocks "PRED"` / a projected `$blocks` field | `markdown`: one grep line per entry — `key › section path › text` — under the result's index line; never a ` · name: value` annotation. JSON/YAML: an array of entry mappings — `type`, `path`, and `text` on every entry (`""` when the block has no own text), plus `target` on every `ref` entry — per the stable-schema rule (§13.1.3). Entries are exempt from `--max-tokens` / `--max-document-tokens`. |
| `--matches PATTERN` / a projected `$matches` field | Same rendering, one entry per matching line (`path`, `text`). |

A narrowed `{ $content: PREDICATE }` field is `$content`-shaped for the fenced-block rule in the `--project` row: it switches the invocation to the fenced-block form with the narrowed rendering as the body, and it is counted and capped by the token budgets like a full body.

#### 13.6.2 `iwe retrieve`

The flags below gate which structural sources are populated (per the conditional-source rule in §6.2.4). When a backing flag is omitted, the corresponding field is still emitted with its empty value.

| Flag | Effect on shape |
|---|---|
| `--children` | `includes` populated with `EdgeRef` entries for child documents. |
| `-b`, `--backlinks` | `referencedBy` populated with `EdgeRef` entries for inbound reference edges (default on). |
| `--expand-includes N` | Adds N levels of inclusion descendants to the top-level array (selection, not shape). |
| `--expand-included-by N` | Same, for inclusion ancestors. |
| `--expand-references N` | Adds outbound reference targets within N hops, and populates `references` with their `EdgeRef` entries. |
| `--expand-referenced-by N` | Adds inbound reference sources within N hops (selection, not shape). |

#### 13.6.3 `iwe tree`

| Flag | Effect on shape |
|---|---|
| `--project EXPR` | JSON/YAML only: each `TreeNode` carries the projected fields alongside the system fields (`key`, `title`, `children`). `EXPR` takes the full `--project` grammar (§12.3.2) — frontmatter fields, `$`-selectors, block sources. `children` is always present regardless of projection. With no `--project`: only system fields. `markdown` and `keys` ignore projection. |
| `--add-fields EXPR` | Same grammar; extends each node's default fields instead of replacing. Mutually exclusive with `--project`. |

#### 13.6.4 Mutation commands

| Flag | Effect on shape |
|---|---|
| `-f keys` | Switches from prose status to one-key-per-line. |
| `--dry-run` | Prefixes prose with `Would …`; for `keys`, lists the keys that *would* be affected. Suppresses writeback. |
| `--quiet` | Suppresses prose-form output. Has no effect on `-f keys`. |

### 13.7 Error output

All commands write errors and progress to stderr. Stdout carries only the format-determined payload. An error path MUST NOT print partial output to stdout — if the command fails before producing a complete result, stdout is empty and the process exits non-zero.

Error message form is free-text, prefixed `Error: ` (or `error: ` for parse-stage failures).

## Appendix A. Formal grammar (BNF)

This appendix collects the full BNF grammar for the IWE query language. The semantic rules — type coercion, missing-field behavior, equality, edge model, walk semantics — live in §§2–11. This appendix is the syntactic source of truth.

### Notation

- `::=` defines a production.
- `|` separates alternatives.
- `[X, ...]` is a YAML sequence of `X`. Empty sequences are noted as parse-time errors where they apply.
- `{ K: V, ... }` is a YAML mapping. Required vs optional entries are annotated inline.
- All literal `$`-prefixed names are operator keywords; user frontmatter field names cannot begin with `$` (§2.3).

### A.1 Operation documents

```
operation ::= find_op | count_op | update_op | delete_op

find_op ::= {
    filter:    filter                               (optional, default {})
    search:    search_spec                          (optional; §A.4A)
    project:   projection                           (optional, mutually exclusive with addFields)
    addFields: projection                           (optional, mutually exclusive with project)
    sort:      sort                                 (optional)
    limit:     limit                                (optional)
}

count_op ::= {
    filter: filter                                  (optional, default {})
    sort:   sort                                    (optional)
    limit:  limit                                   (optional)
}

update_op ::= {
    filter: filter                                  (required)
    sort:   sort                                    (optional)
    limit:  limit                                   (optional)
    update: update_doc                              (required)
    expect: expect_val                              (optional; matched-document guard, §3.2; production in §A.7)
}

delete_op ::= {
    filter: filter                                  (required)
    sort:   sort                                    (optional)
    limit:  limit                                   (optional)
    expect: expect_val                              (optional; matched-document guard, §3.2; production in §A.7)
}
```

Operation-inappropriate fields are parse-time errors (e.g. `project` outside `find`, `update` outside `update`, `expect` outside `update` / `delete`). `project` and `addFields` cannot both be set in a single `find_op` (§6.3).

### A.2 Filter

```
filter ::= { (filter_entry)* }                     # entries AND-composed at top level

filter_entry ::=
    field_path : field_predicate
  | logical_op
  | graph_op

field_predicate ::=
    value                                          # implicit $eq (§4.1)
  | operator_expr
  | nested_filter

operator_expr ::= { ($_field_op : V)+ }            # all keys $-prefixed; multiple keys ANDed

nested_filter ::= { (sub_field_entry)+ }           # all keys non-$-prefixed

sub_field_entry ::= field_path : field_predicate

# A mapping that mixes $-prefixed and non-$-prefixed keys at the same level is a parse-time error.
```

#### A.2.1 Logical operators

```
logical_op ::=
    $and : [filter, ...]                           # non-empty
  | $or  : [filter, ...]                           # non-empty
  | $nor : [filter, ...]                           # non-empty
```

`$not` is field-level only; see `$_field_op` below.

#### A.2.2 Field operators

```
$_field_op ::=
    comparison_op
  | element_op
  | array_op
  | $not : operator_expr                           # per-field negation

comparison_op ::=
    $eq:  value
  | $ne:  value
  | $gt:  value
  | $gte: value
  | $lt:  value
  | $lte: value
  | $in:  [value, ...]                             # non-empty
  | $nin: [value, ...]                             # non-empty

element_op ::=
    $exists: bool
  | $type:   type_name | [type_name, ...]          # non-empty list

array_op ::=
    $all:  [value, ...]                            # non-empty
  | $size: non_neg_int

type_name ::=
    "string" | "number" | "boolean" | "null"
  | "array"  | "object" | "date"    | "datetime"

# Type names are YAML strings only. The bare YAML null literal ($type: null) is a
# parse-time error — write $type: "null" to test for the null type.
```

#### A.2.3 Field paths

```
field_path ::= segment ("." segment)*              # dotted shorthand
segment    ::= identifier                          # §2.3
                                                   # non-empty; no whitespace; no control chars; no `.`;
                                                   # first char not in $, _, ., #, @
```

A nested mapping (`author: { name: ... }`) is equivalent to the dotted form (`author.name: ...`). Field names containing a literal `.` are not addressable.

### A.3 Graph operators

```
graph_op ::=
    $key          : key_op
  | $includes     : relational_arg
  | $includedBy   : relational_arg
  | $references   : relational_arg
  | $referencedBy : relational_arg
  | $content      : block_predicate                 # block membership: at least one matching block (§A.9)
```

The `filter` production used inside relational operators (`match` field, §A.3.2) is the same `filter` production from §A.2 — the grammar is mutually recursive.

#### A.3.1 Identity

```
key_op ::= key | key_expr

key_expr ::=
    { $eq:  key }
  | { $ne:  key }
  | { $in:  [key, ...] }                           # non-empty
  | { $nin: [key, ...] }                           # non-empty

# $gt / $gte / $lt / $lte on $key are parse-time errors.
```

#### A.3.2 Relational operators

```
relational_arg ::= key | relational_obj

relational_obj ::= {
    match:       filter                            (required)
    maxDepth:    pos_int                           (inclusion ops, optional; absent = unbounded)
    minDepth:    pos_int                           (inclusion ops, optional; absent = 1)
    maxDistance: pos_int                           (reference ops, optional; absent = unbounded)
    minDistance: pos_int                           (reference ops, optional; absent = 1)
}

# Scalar `key` shorthand expands to:
#   - inclusion ops:  { match: { $key: KEY }, maxDepth: 1 }
#   - reference ops:  { match: { $key: KEY }, maxDistance: 1 }
# Inclusion-edge ops accept maxDepth / minDepth only;
#   maxDistance / minDistance are parse-time errors.
# Reference-edge ops accept maxDistance / minDistance only;
#   maxDepth / minDepth are parse-time errors.
# match is required; an object without match is a parse-time error.
# Empty mapping {} is a parse-time error. The array form [...] is a parse-time error.
# All walk-parameter values are positive integers (>= 1).
# No -1 sentinel; absence is the unbounded signal in the full relational_obj form.
# minDepth > maxDepth (and minDistance > maxDistance) is a parse-time error (§5.2.3).
# Field names inside relational_obj are bare — $-prefix is reserved for evaluating operators.
# The recognized key set is closed: any key other than match / maxDepth / minDepth /
#   maxDistance / minDistance is a parse-time error (unknown keys are not silently ignored).
# The filter inside `match` is the §A.2 filter production — the grammar is mutually recursive.
# Because that filter accepts any §A.2 / §A.3 production, $key is allowed inside `match`:
#   { match: { $key: K },                       maxDepth: 5 }
#   { match: { $key: { $in: [a, b] } },         maxDepth: 5 }
#   { match: { $or: [{ $key: a }, { tag: x }] } }
```

### A.4 Projection

```
projection ::= { (project_entry)* }               # empty mapping = explicit empty projection:
                                                   #   each result carries no fields

project_entry ::= field_path : source

source ::=
    include_marker                                 # include frontmatter[outputName]
  | "$" pseudo_field                               # include the named structural pseudo-field source
  | dotted_path                                    # include frontmatter at the dotted path
  | block_source                                   # block surface (§A.9 for block_predicate)

include_marker  ::= 1 | true | null                # all three mean "include frontmatter[outputName]";
                                                   # type-strict: integer 1, bool true, or YAML null
                                                   # 0, false, "1", "true", "null", 1.0 → parse-time error

pseudo_field    ::= "key" | "title" | "titleSlug" | "content" | "frontmatter"
                  | "includedBy" | "includes" | "referencedBy" | "references"
                                                   # closed set; see §6.1

block_source ::=
    { $content: block_predicate }                  # body narrowed to matching blocks — a string
  | $blocks | { $blocks: block_predicate }         # located blocks as entries; bare form = { $blocks: {} }
  | { $matches: regex }                            # grep — one entry per matching line
  | { $matches: { pattern: regex, (block_pred_entry)* } }
                                                   # scoped grep: bare `pattern` is the payload, $-keys select

dotted_path     ::= segment ("." segment)*         # §A.2.3 segment rules
```

The `project` clause of a `find_op` (not `addFields`) additionally accepts a **block predicate** in place of the field map — any `$`-key at the top level of `project` selects this reading:

```
project_clause ::= projection                      # v1 field map (bare output names)
                 | block_predicate                 # §A.9; lowers to
                                                   #   { key: $key, content: { $content: P } };
                                                   #   mixing bare and $ keys = parse-time error;
                                                   #   {} keeps its v1 empty-projection meaning
```

### A.4A Search (find only)

```
search_spec ::= {
    lexical: string                                (optional)
    fuzzy:   string                                (optional)
}

# At least one of lexical / fuzzy is required; the empty mapping {} is a parse-time error.
# Both present → RRF fusion of the two rankings.
# The recognized key set is closed: any key other than lexical / fuzzy is a parse-time error.
# search is valid only in a find operation; appearing in count / update / delete is a parse-time error.
# search selects and orders (§5A): membership = filter ∩ search-matches, default order = relevance.
# A lexical query with no searchable terms (stop words only) matches nothing and carries a warning.
```

### A.5 Sort

```
sort     ::= { field_path : sort_dir }             # exactly one entry
sort_dir ::= 1 | -1                                # type-strict integer; YAML +1 normalizes to 1 and is accepted;
                                                   # 1.0, "1", true, null → parse-time error
```

### A.6 Limit

```
limit ::= non_neg_int                              # 0 = no limit
```

### A.7 Update document

```
update_doc ::= { (update_op_entry)+ }              # at least one operator

update_op_entry ::=
    $set:   { (field_path : value)+ }              # body must be non-empty
  | $unset: { (field_path : any_value)+ }          # body must be non-empty; values ignored
  | $replace:      replace_arg                     # block operators (§9.5); block_pred_entry in §A.9
  | $replaceText:  replace_text_arg
  | $insertBefore: insert_arg
  | $insertAfter:  insert_arg
  | $append:       insert_arg
  | $delete:       delete_arg

replace_arg      ::= { (block_pred_entry)*, content: markdown, expect?: expect_val }
replace_text_arg ::= { (block_pred_entry)*, from?: string, to: string, expect?: expect_val }
                                                   # from omitted: to replaces the entire own text
insert_arg       ::= { (block_pred_entry)*, content: markdown, expect?: expect_val }
delete_arg       ::= { (block_pred_entry)*, expect?: expect_val }

expect_val       ::= non_neg_int | { min?: non_neg_int, max?: non_neg_int }
                                                   # also the document-level expect of update_op / delete_op (§A.1)

# Within a block operator's argument, $-prefixed keys form the block predicate
#   and bare keys are the payload. The bare-key set per operator is closed;
#   unknown bare keys are parse-time errors.
# An argument with no $-keys carries the empty predicate and selects all blocks.
# Block operators are valid only in update documents; appearing in a find /
#   count / delete operation is a parse-time error.
# Unit operators act on the selection's coalesced roots; their expect counts
#   targets. $replaceText applies per selected block, un-coalesced; its expect
#   counts blocks. Semantics: query-language.md#block-update-operators.
# Empty $set: {} / $unset: {} is a parse-time error (grammar requires +).
# Targeting a reserved-prefix segment (_, $, ., #, @ as first character of any segment in
#   any path — top-level, dotted, or nested mapping key, recursively) is a parse-time error.
# Two paths in $set / $unset conflict when, after canonicalizing nested-mapping form
#   to dotted form per §4.4, one path equals or is a prefix of the other. Conflicts are
#   parse-time errors. The check applies across operators ($set vs $unset) and within
#   a single operator (two $set entries).
# A dotted $set path that traverses a present-but-non-mapping intermediate coerces the
#   intermediate to a fresh mapping holding the new leaf (per §9.1, $set).
#   Not a parse-time error and not a runtime failure.
# Mapping values in $set replace wholesale; use dotted shorthand to write subset leaves.
```

### A.8 Primitives

```
key         ::= string                             # document key (relative path without .md)
identifier  ::= YAML name; non-empty; no whitespace; no control chars; no `.`;
                first char not in $, _, ., #, @
value       ::= scalar | array | mapping | null
scalar      ::= string | number | boolean | date | datetime
array       ::= [value, ...]
mapping     ::= { (string : value)+ }
bool        ::= true | false
non_neg_int ::= integer ≥ 0
pos_int     ::= integer ≥ 1
any_value   ::= value                              # placeholder; ignored by $unset
regex       ::= string                             # Rust regex syntax; compile failure = parse-time error
markdown    ::= string                             # parsed and normalized on write
```

### A.9 Block predicates

One grammar, consumed at three sites: `$content` in filter (§A.3), the block projection sources and the top-level `project` form (§A.4), and the block operators' selectors (§A.7). Semantics — selection forests, own text, normalized-form matching — are specified in the [Query Language reference](query-language.md#block-predicates).

```
block_predicate ::= { (block_pred_entry)* }        # empty mapping matches every block

block_pred_entry ::=
    $text:       string | { $eq: string }          # case-insensitive; bare = substring, $eq = whole own text
  | $matches:    regex                             # case-sensitive; (?i) for case-insensitive
  | $within:     string | block_predicate          # interior of the argument's forest;
                                                   # scalar T = { $section: { $text: { $eq: T } } };
                                                   # {} = the interior of the document;
                                                   # a mapping argument must select content:
                                                   #   hold a tree_selector ($and: some branch,
                                                   #   $or: every branch) — else parse-time error
  | $contains:   block_predicate                   # descendant axis: has such a block below it
  | tree_selector                                  # in block position: all blocks of the trees
  | type_op                                        # per-type operators
  | $references: key                               # own content links to the key
  | $and:        [block_predicate, ...]            # non-empty
  | $or:         [block_predicate, ...]            # non-empty
  | $nor:        [block_predicate, ...]            # non-empty

type_op ::=
    $header: type_arg | $paragraph: type_arg | $item: type_arg
  | $code:   type_arg | $table:     type_arg
  | $ref:    block_predicate                       # scalar = parse-time error; own text = authored (piped) link text
  | $hr:     block_predicate                       # no own text: scalar or direct $text / $matches = parse-time error

type_arg ::= string | block_predicate              # scalar T = { $text: { $eq: T } }; {} = any block of the type

tree_selector ::= { $section: string | block_predicate }
                                                   # trees rooted at headers matching the argument
                | { $quote: block_predicate }      # empty head; scalar or direct $text / $matches = parse-time error
                | { $list:  block_predicate }      # empty head; scalar or direct $text / $matches = parse-time error
```

Every key in a block predicate is a `$`-prefixed operator; unknown `$`-names and bare keys are parse-time errors. Top-level keys AND together, as everywhere in the language.
