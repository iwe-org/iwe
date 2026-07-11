# Document Schema

A document schema declares the required shape of a page: which frontmatter
fields it carries, which sections it contains and in what order, how headers
are written, how deep the heading tree may nest, and how large each part may
grow. Schemas are checked by [`iwe schema validate`](cli-schema.md), so a
store's conventions become machine-checked policy in the loop
*write → validate → fix*.

The language is JSON-Schema-aligned. Frontmatter is validated by literal
JSON Schema (draft 2020-12). The body schema mirrors the document's own
structure — a document has `sections`, a section has a `header` and its own
nested `sections` — and keyword names and semantics come from JSON Schema
wherever the concept maps: `pattern`, `const`, `enum`, `minLength`,
`maxLength`, `minContains`, `maxContains`, `additionalSections` (after
`additionalProperties`), `description`.

A complete schema:

``` yaml
$schema: https://iwe.md/document-schema/v1
frontmatter:
  type: object
  required: [status]
  properties:
    status: { enum: [draft, published] }
maxTokens: 1200
sections:
  - header: { pattern: "^[A-Z]", maxTokens: 12 }
    maxContains: 1
    sections:
      - header: { const: Summary }
        maxContains: 1
        description: every note opens with a summary
      - header: { const: Tasks }
        maxContains: 1
additionalSections: false
```

This reads: the frontmatter declares a `status`; the page body stays under
1200 tokens; there is exactly one top-level section, its header capitalized
and at most 12 tokens; it contains a `Summary` section and then a `Tasks`
section, in that order; no other top-level sections are allowed.

## 1. Schema documents

Schemas live in `.iwe/schemas/<name>.yaml`, one schema per file. Files are
YAML 1.2, so JSON content is equally valid. The optional `$schema` key names
the dialect; only `https://iwe.md/document-schema/v1` is accepted.

A schema binds to documents through the `[schemas]` section of
`.iwe/config.toml` (see [Configuration](configuration.md)):

``` toml
[schemas.note]
match = "notes/**"

[schemas.session]
match = ["journal/*", "meetings/**"]
```

The entry name is the schema name — `[schemas.note]` resolves to
`.iwe/schemas/note.yaml`. `match` is a glob, or a list of globs, matched
against the document key: `*` stays within a path segment, `**` crosses
segments, and a leading `/` is optional. Binding is order-free — a document
is validated against **every** schema whose `match` hits, so overlapping
bindings compose (as JSON Schema `allOf` does). A document matching no entry
is unvalidated.

Every keyword in a schema is optional, and an absent keyword constrains
nothing. An empty schema (`{}`) passes every document.

## 2. What is validated

- **The section tree.** Sections at their structural depth after
  normalization (`1` for `#`, `2` for `##`). A section's subsections are its
  `sections`, in document order. Non-section content — paragraphs, lists,
  code blocks, tables — is invisible to the schema: it is never matched and
  never violates.
- **Header text.** The rendered plain text of a heading, inline markup
  stripped.
- **Token counts.** The same counting as the retrieve budgets, over rendered
  text, frontmatter excluded.
- **The frontmatter mapping.** `{}` when the page has no frontmatter.
  Reserved-prefix fields (`_`, `$`, `.`, `#`, `@` — see the
  [query language](spec.md)) are invisible to the schema, mirroring the
  query engine. YAML dates and datetimes are presented to the validator as
  ISO-8601 / RFC 3339 strings.

## 3. Document schema

The top level of a schema file:

| Keyword              | Value                          | Meaning                                                                |
| -------------------- | ------------------------------ | ---------------------------------------------------------------------- |
| `$schema`            | string                         | optional dialect id                                                     |
| `description`        | string                         | default hint for document-level violations                              |
| `frontmatter`        | JSON Schema                    | validates the frontmatter mapping                                       |
| `maxTokens`          | integer                        | budget for the whole rendered body                                      |
| `maxDepth`           | integer                        | maximum heading nesting (`3` allows `###`, forbids `####`)              |
| `allSections`        | reduced section schema (§6)    | applies to every section at every depth                                 |
| `sections`           | array of section schemas       | ordered shapes for the top-level sections                               |
| `additionalSections` | bool or reduced section schema | policy for top-level sections matching no entry; default `true` (open)  |

The `frontmatter` value is standard JSON Schema, draft 2020-12, with
`format` assertions enabled. Only in-document references (`#/...`) are
allowed; external and remote `$ref` are rejected.

## 4. Section schema

An item in a `sections` array is always a section.

| Keyword              | Value                          | Meaning                                                                  |
| -------------------- | ------------------------------ | ------------------------------------------------------------------------ |
| `header`             | header schema (§5)             | constrains the header text; also decides binding (§7)                     |
| `maxTokens`          | integer                        | budget for this section's subtree, header included                        |
| `maxDepth`           | integer                        | maximum nesting below this section (`1` allows children, forbids deeper)  |
| `minContains`        | integer, default `1`           | minimum occurrences of this shape; `0` makes it optional                  |
| `maxContains`        | integer, default unbounded     | maximum occurrences                                                       |
| `description`        | string                         | the violation hint for anything failing in this entry                     |
| `allSections`        | reduced section schema         | applies to every section below this one                                   |
| `sections`           | array of section schemas       | ordered shapes for the subsections                                        |
| `additionalSections` | bool or reduced section schema | policy for subsections matching no entry; default `true`                  |

The occurrence defaults are JSON Schema's `contains` defaults: a listed
shape is required at least once and unbounded above. The recipes:

| Intent                  | Spelling                         |
| ----------------------- | -------------------------------- |
| one or more (default)   | nothing                          |
| optional                | `minContains: 0`                 |
| exactly one             | `maxContains: 1`                 |
| at most one             | `minContains: 0, maxContains: 1` |
| n or more               | `minContains: n`                 |
| exactly n               | `minContains: n, maxContains: n` |

## 5. Header schema

Applies to the header's plain text. The string keywords carry JSON Schema
semantics exactly; `maxTokens` is the one extension.

| Keyword       | Meaning                                                                                    |
| ------------- | ------------------------------------------------------------------------------------------ |
| `pattern`     | regex the text must match; unanchored, as in JSON Schema — write `^...$` for a full match  |
| `const`       | the text equals this string                                                                 |
| `enum`        | the text equals one of these strings                                                        |
| `minLength`   | minimum length in characters                                                                |
| `maxLength`   | maximum length in characters                                                                |
| `maxTokens`   | maximum tokens in the header text                                                           |
| `description` | hint override for header violations                                                         |

`const` is in principle an anchored, regex-escaped `pattern`
(`const: Tasks` ≡ `pattern: "^Tasks$"` with metacharacters escaped), but it
is a distinct keyword for three reasons: JSON Schema alignment; escaping
safety for literal headers containing regex metacharacters (a header like
`C++ (Draft)` needs no escaping under `const`); and error naming — a
missing-section message takes the section's name from `const` directly.
Mind the asymmetry: `pattern` is unanchored, so `pattern: Tasks` matches
any header *containing* "Tasks", while `const: Tasks` matches exactly.
`enum` is a disjunction of consts. `enum` and `const` cannot be combined.

## 6. Reduced section schemas

`allSections` and a schema-valued `additionalSections` take a **reduced**
section schema: `header`, `maxTokens`, `maxDepth`, and `description` only.
Occurrence keywords are meaningless there — `allSections` applies to every
section, `additionalSections` applies per leftover section — and structural
keywords (`sections`, `additionalSections`, `allSections`) are not allowed
inside them. When several `allSections` are in scope (the document's plus
enclosing sections'), all of them apply.

`additionalSections` is **boolean or schema**, matching JSON Schema's own
value form for array extras (`items` after `prefixItems`, draft-07's
`additionalItems`, `unevaluatedItems` — all schema-or-boolean, all open by
default). `true` allows leftover sections unconstrained, a schema validates
each leftover against it, `false` makes each leftover a violation.
Semantically it is closest to `unevaluatedItems`: it governs the sections
no listed shape claimed.

## 7. Matching semantics

For each node — the document, then each bound section, recursively — the
node's sections are matched against its `sections` entries. Matching is
**ordered, sequential, and greedy, without backtracking**:

1. Walk the instance sections in document order, holding a pointer into the
   entry list, starting at the first entry.
2. For each section, find the first entry — at the pointer or later — whose
   **`header` schema** the section's header text satisfies (an entry with no
   `header` matches any section). Bind the section to that entry and advance
   the pointer to it. Entries before the pointer are closed and never bind
   again.
3. A section that satisfies no entry at or after the pointer — including one
   that would only match an already-closed entry, i.e. out of order — is
   **additional** and is handled by `additionalSections`.
4. After the walk, every entry's bound count is checked against
   `minContains` and `maxContains`. An entry bound fewer than `minContains`
   times reports a missing required section, named by its `const`, else
   `enum`, else `pattern`, else its position.
5. Each bound section is then validated against the rest of its entry:
   `maxTokens`, `maxDepth`, `allSections`, and the nested `sections`
   matching, recursively.

Consequences:

- **Binding is decided by `header` alone.** A `Tasks` section missing its
  required subsections still binds to the `Tasks` entry and reports the
  missing pieces — it does not fall through to `additionalSections`.
- Repeated shapes bind as **consecutive runs**: `minContains: 3` means three
  such sections in a row; an interloper ends the run (under the default
  `additionalSections: true` the interloper passes and a new run cannot
  rejoin the closed entry).
- A headerless (wildcard) entry with the default unbounded `maxContains`
  greedily absorbs every remaining section; give it `maxContains` or make it
  the last entry.
- There is no backtracking: matching is deterministic and errors are
  explainable. Order entries specific-first.

## 8. Violations

`iwe schema validate` reports one line per violation, `<key> › <breadcrumb>:
<message>` (or `<key>: <message>` when the breadcrumb is empty). The
breadcrumb is built from the matched header texts — a position like
`sections[2]` where no header text is available, a frontmatter path like
`frontmatter › status`. A `hint:` line follows when a hint is present:

``` text
journal/2026-01-05 › Tasks: header is 18 tokens (limit 12)
  hint: keep section headers short
notes/intro: required section 'Summary' missing
  hint: every note opens with a summary
notes/intro › frontmatter › status: not one of 'draft', 'published'
```

The hint is the nearest `description` walking outward from the failing
keyword — header schema, then entry, then enclosing entries, then the
document schema. Without one, no hint line is shown.

`-f json` output is an array of `{ key, schema, violations }` objects; each
violation additionally carries the machine paths `schemaPath` (a JSON Pointer
into the schema file, e.g. `/sections/0/sections/1/header`) and the failing
`keyword`. A document bound to several schemas yields one report per schema.
The command exits `1` when any document has a violation, `0` when the store is
clean.

## 9. Schema errors

These are configuration errors — `iwe schema validate` prints them to stderr
and exits `2` before validating any document, rather than reporting them as
violations:

- a `[schemas]` entry naming a schema file that does not exist;
- an invalid glob in a `[schemas]` entry's `match`;
- a `frontmatter` subschema that fails the 2020-12 meta-schema, or contains
  an external or remote `$ref`;
- an unknown keyword anywhere outside `frontmatter` — unlike JSON Schema,
  unknown keywords are rejected, so a typo cannot silently validate nothing;
- a reserved keyword (§10);
- an invalid `pattern` regex, a negative count, `minContains` greater than
  `maxContains`, or `enum` and `const` together;
- occurrence or structural keywords inside a reduced section schema.

## 10. Reserved keywords

Block-level validation is a planned extension. Its keywords are reserved
and rejected at load with a message saying so: `blocks`,
`additionalBlocks`, `type`, `items`, `minItems`, `maxItems`, `ordered`,
`lang`, `text`, `target`.

## 11. Examples

Header discipline for a whole store — every header capitalized and short,
every section within budget, nothing deeper than `###`:

``` yaml
maxDepth: 3
allSections:
  header: { pattern: "^[A-Z]", maxLength: 60 }
  maxTokens: 400
```

A log page — at least three dated entries, each small, extra sections
allowed but budgeted:

``` yaml
sections:
  - header: { pattern: '^\d{4}-\d{2}-\d{2}$' }
    minContains: 3
    maxTokens: 150
additionalSections:
  maxTokens: 300
```

A docs page — Installation and Usage required in order, Configuration
optional:

``` yaml
sections:
  - header: { pattern: ".+" }
    maxContains: 1
    sections:
      - header: { const: Installation }
      - header: { const: Usage }
      - header: { const: Configuration }
        minContains: 0
```

Frontmatter only — the body left free:

``` yaml
frontmatter:
  type: object
  required: [type, date]
  properties:
    type: { const: post }
    date: { type: string, format: date }
    tags:
      type: array
      items: { type: string, pattern: "^[a-z][a-z0-9-]*$" }
```

## See also

- [`iwe schema validate`](cli-schema.md) — the command that checks schemas,
  its output and exit codes.
- [Configuration](configuration.md) — the `[schemas]` section and glob
  binding.
- [Query Language](spec.md) — the corpus model and reserved frontmatter
  prefixes.
