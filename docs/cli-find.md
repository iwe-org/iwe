# IWE Find

Search and discover documents in your knowledge base. Combines a text query — `--fuzzy` (title and key) or `--lexical` (BM25 full-text) — with a YAML-based filter language.

## Usage

``` bash
iwe find [OPTIONS]
iwe find --fuzzy <QUERY> [OPTIONS]
iwe find --lexical <QUERY> [OPTIONS]
```

## Options

| Flag                            | Description                                                                                  | Default    |
| ------------------------------- | -------------------------------------------------------------------------------------------- | ---------- |
| `--fuzzy <QUERY>`               | Fuzzy match on document title and key.                                                       | none       |
| `--lexical <QUERY>`             | Lexical (BM25) full-text match on title and body.                                            | none       |
| `[QUERY]`                       | Deprecated: bare positional query. Behaves as `--fuzzy` and prints a warning.                | none       |
| `--filter <EXPR>`               | Inline YAML filter expression. See [Query Language](query-language.md).                      | none       |
| `-k, --key <KEY>`               | Match by document key. Repeatable: 1 key uses `$eq`, 2+ uses `$in`.                          | none       |
| `--includes <KEY[:DEPTH]>`      | `$includes` anchor. Repeatable; anchors are ANDed.                                           | none       |
| `--included-by <KEY[:DEPTH]>`   | `$includedBy` anchor. Repeatable; anchors are ANDed.                                         | none       |
| `--references <KEY[:DIST]>`     | `$references` anchor. Repeatable; anchors are ANDed.                                         | none       |
| `--referenced-by <KEY[:DIST]>`  | `$referencedBy` anchor. Repeatable; anchors are ANDed.                                       | none       |
| `--max-depth <N>`               | Session default for inclusion anchor flags without a colon-suffix. `0` = unbounded.          | 1          |
| `--max-distance <N>`            | Session default for reference anchor flags without a colon-suffix. `0` = unbounded.          | 1          |
| `--project <EXPR>`              | Projection: comma-list (`name`, `name=path`, `name=$selector`, `$selector`) or inline YAML mapping. Replaces the default fields. | none       |
| `--add-fields <EXPR>`           | Additive projection: same grammar as `--project`, extends the defaults instead of replacing. | none       |
| `--blocks <PRED>`               | Locate blocks: adds a `blocks` field listing each block matching the inline block predicate. | none       |
| `--matches <PATTERN>`           | Grep over blocks: restricts results to documents whose content matches the Rust regex `PATTERN` and adds a `matches` field with the matching lines. | none       |
| `--sort <field:DIR>`            | Sort by frontmatter field. `DIR` is `1` (asc) or `-1` (desc).                                | none       |
| `-l, --limit <N>`               | Maximum number of results (`0` = unlimited).                                                 | unlimited  |
| `--max-tokens <N>`              | Cap total projected `$content` tokens across all results (`0` = unlimited).                  | unlimited  |
| `--max-document-tokens <N>`          | Cap projected `$content` tokens per result, head-truncating with a marker (`0` = unlimited). | unlimited  |
| `-f, --format <FMT>`            | Output format: `markdown`, `keys`, `json`, `yaml`.                                           | `markdown` |

All filter clauses (the text query plus every flag above) are AND-composed at the top level. For OR or NOT, write it inside `--filter`. See [Query Language](query-language.md).

`--lexical` stems its terms and drops stop words; a query with no searchable terms left matches nothing (a warning suggests `--fuzzy` for common or partial words).

`--max-tokens` and `--max-document-tokens` only act when the projection includes a `$content`-shaped field — bare `$content` or a narrowed `{ $content: PREDICATE }`; a narrowed body is counted and capped like a full one, while `$blocks` / `$matches` entries cost nothing. A metadata index carries no content tokens, so bound it with `--limit`. Token budgets are off by default, count body text only, and print a `warning:` to stderr when they trim the output.

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

The same selector flags are accepted by [`iwe count`](cli-count.md), [`iwe retrieve`](cli-retrieve.md), [`iwe tree`](cli-tree.md), [`iwe export`](cli-export.md), [`iwe schema`](cli-schema.md), [`iwe update`](cli-update.md), and [`iwe delete`](cli-delete.md).

## Block projection

`--project` and `--add-fields` can address blocks — the structural nodes inside each matched document — through three sources, each taking a block predicate. See [Query Language](query-language.md#block-projection) for the predicate grammar. A block predicate is structured, so these forms are written as an inline YAML mapping (the comma list reaches only bare `$blocks` and `$content`):

``` bash
# Body narrowed to one section, header included
iwe find -k projects/roadmap --project 'notes: { $content: { $section: Unreleased } }'

# Table of contents: the whole argument is a block predicate (--project only)
iwe find -k guides/handbook --project '{ $header: {} }'

# Located blocks as data: type, section path, own text
iwe find --add-fields 'hits: { $blocks: { $within: Goals, $text: "Q3" } }'

# Grep: matching lines with their locations
iwe find --add-fields 'found: { $matches: "(?i)todo|fixme" }'
```

Two dedicated flags shortcut the common reads:

``` bash
# --blocks PRED lowers to: addFields: { blocks: { $blocks: PRED } }
iwe find --blocks '{ $within: Goals, $text: "Q3" }'

# --matches PATTERN lowers to BOTH a membership clause and a projection entry:
#   filter: { $content: { $matches: PATTERN } }
#   addFields: { matches: { $matches: PATTERN } }
# — one-flag grep: only matching documents return, each carrying its lines
iwe find --matches '(?i)todo|fixme'
```

`--blocks` adds the located blocks without restricting membership; combine it with `--filter '$content: …'` (usually the same predicate) to drop non-matching documents. `--matches` restricts membership on its own.

In markdown output, `$blocks` and `$matches` entries print one grep line each — `key › section path › text` — under the result's index line:

```
- [Roadmap](projects/roadmap)
projects/roadmap › Goals › Q3 Milestones › Ship the editor integration TODO confirm date
```

In JSON / YAML, a narrowed `$content` field is a string (empty when no block matches) and `$blocks` / `$matches` fields are arrays of entries carrying `path` (enclosing section titles), `text` (own text), and — for `$blocks` — `type`.

## How it works

1. **Text matching** — `--fuzzy` matches the key and the title using SkimMatcherV2; `--lexical` runs a BM25 full-text query over title and body.
2. **Filter** — `--filter` and the structural-anchor flags evaluate per document; results are intersected.
3. **Sort** — `--sort field:DIR` orders the matched set; ties are broken by document key.
4. **Limit** — applied last.
5. **Project** — `--project` / `--add-fields` shape each result: frontmatter fields, system fields (`$key`, `$content`, edge selectors), or block-addressed sources.

Without a text query, results are sorted by incoming-reference popularity. With `--fuzzy`, they are sorted by fuzzy match score; with `--lexical`, by BM25 relevance.

## Output formats

### Markdown (default)

A compact index — one line per document, meant to be scanned:

``` markdown
- [User Authentication](authentication)
- [Login Flow](login-flow) <- [User Authentication](authentication)
- [Session Management](session-management) <- [User Authentication](authentication)
```

Each line is `- [title](key)`, followed by any projected edge and scalar fields:

- **Link text** is the `title`. When `title` is not projected (e.g. `--project key`), the key string is used instead.
- **Edge fields** render as arrows after the link, by direction: incoming (`includedBy`, `referencedBy`) as `<- [Title](key)`, outgoing (`includes`, `references`) as `-> [Title](key)`. Multiple targets in one field are comma-joined after a single arrow. Empty edges are omitted.
- **Scalar fields** render as ` · name: value`. Arrays of scalars join with `, `; richer values fall back to compact inline YAML (use `-f json`/`yaml` for those).

The index never prints document bodies. Projecting a `$content`-shaped field (via `--project` or `--add-fields`, bare or narrowed) switches the whole invocation to the fenced document block that [`iwe retrieve`](cli-retrieve.md) emits, with content as the body and other projected fields as frontmatter. A narrowed `{ $content: PREDICATE }` puts the narrowed rendering in the body; several `$content`-shaped fields concatenate in projection order. `$blocks` / `$matches` fields render as grep lines instead (see [Block projection](#block-projection)).

``` markdown
$ iwe find --fuzzy auth --add-fields '$content'
````markdown #authentication
---
title: User Authentication
---

# User Authentication

…body…
````
```

To read full documents, use [`iwe retrieve`](cli-retrieve.md).

### Keys (`-f keys`)

```
authentication
login-flow
session-management
```

One key per line; suitable for piping.

### JSON (`-f json`)

``` json
[
  {
    "key": "authentication",
    "title": "User Authentication",
    "includedBy": []
  }
]
```

The top-level value is a bare array of result objects. Each object carries the system fields `key`, `title`, `includedBy`, and every user-frontmatter field merged at the top level. `includedBy` entries are `EdgeRef { key, title, sectionPath }`.

`--project title,status` projects only those fields into each result, in the listed order. System fields and user frontmatter fields are projectable interchangeably.

### YAML (`-f yaml`)

Same shape as JSON, rendered as YAML.

## Examples

``` bash
# All documents, default markdown
iwe find

# Fuzzy search on title and key
iwe find --fuzzy authentication

# Full-text search on title and body
iwe find --lexical "session token"

# Text query AND a frontmatter filter
iwe find --fuzzy auth --filter 'status: draft'

# Roots — documents with no incoming inclusion edges
iwe find --filter '$nor: [{ $includedBy: { match: {} } }]'

# Limit
iwe find --limit 10

# JSON for programmatic use, project two fields
iwe find --project title,modified_at -f json

# Corpus-wide grep with locations
iwe find --matches '(?i)todo|fixme'

# Locate blocks before mutating them with iwe update
iwe find --blocks '{ $within: Goals, $text: "Q3" }'

# One section of one document
iwe find -k projects/roadmap --project 'notes: { $content: { $section: Goals } }'

# Pipe keys to retrieve
iwe find --filter 'status: draft' -f keys | xargs -I {} iwe retrieve -k {}
```

## Deprecated aliases

The following flags pre-date the query language and remain accepted for backward compatibility. Each invocation prints a one-line `warning: ... is deprecated` to stderr.

| Deprecated         | Use instead                                                                 |
| ------------------ | --------------------------------------------------------------------------- |
| bare positional `QUERY` | `--fuzzy QUERY` (or `--lexical QUERY` for full-text)                   |
| `--in KEY[:N]`     | `--included-by KEY[:N]`                                                     |
| `--in-any K1 K2`   | `--filter '$or: [{ $includedBy: K1 }, { $includedBy: K2 }]'`                |
| `--not-in KEY`     | `--filter '$nor: [{ $includedBy: KEY }]'`                                   |
| `--refs-to KEY`    | `--references KEY` (legacy semantics: ORs `$includes` and `$references`)    |
| `--refs-from KEY`  | `--referenced-by KEY` (legacy semantics: ORs `$includedBy` and `$referencedBy`) |
| `--roots`          | `--filter '$nor: [{ $includedBy: { match: {} } }]'`                         |

## Technical notes

- All filter flags AND together at the top level. To compose with OR or NOR, wrap inside `--filter`.
- The colon-suffix on an anchor flag (`KEY:N`) overrides `--max-depth` / `--max-distance` for that anchor only. `0` is the unbounded sentinel.
- Combining `-k KEY` with a `--filter` whose top level also contains `$key` is a parse-time error. Use `-k a -k b` for multi-key match (lowers to `$key: { $in: [a, b] }`), or write the OR inside `--filter`.
- Both [Inclusion Links](inclusion-links.md) and inline references count toward `incoming_refs`.
