# Query Language

IWE has a YAML-based, MongoDB-style query language for selecting, shaping,
and mutating documents in a workspace by their frontmatter, graph
relationships, and content. It is reachable through the CLI subcommands
`iwe find`, `iwe count`, `iwe update`, and `iwe delete`, plus the read-only
selector flags on `iwe retrieve`, `iwe tree`, and `iwe export`.

Every YAML example below is a complete, valid input: operation documents run
as given, filter documents fit `--filter`, block predicates fit the block
sites described in the Blocks section.

## Operations

| Operation | CLI subcommand | What it does |
| --- | --- | --- |
| `find` | `iwe find` | Returns matched documents (subject to projection). |
| `count` | `iwe count` | Returns the integer count of matched documents. |
| `update` | `iwe update` | Mutates frontmatter and blocks on each matched document. |
| `delete` | `iwe delete` | Removes each matched document and cleans up references. |

`update` and `delete` require an explicit filter — passing `{}` on purpose is
the only way to operate on the whole corpus.

## Filter syntax

A filter document is YAML. A document matches when every top-level key
matches; multiple top-level keys are AND-composed.

### Bare equality

```yaml
status: draft
tags: rust
```

Matches documents whose `status` equals `draft`. For arrays, a bare scalar
tests membership: `tags: rust` matches when `rust` is in the `tags` array.
Cross-type comparisons are always false (no implicit coercion: `priority: "3"`
does not match an integer field).

### Operator expressions

A mapping with `$`-prefixed keys is an operator expression. Operators in one
expression are ANDed together:

```yaml
priority: { $gt: 3 }
score:    { $gte: 3, $lte: 7 }        # closed range [3, 7]
status:   { $in: [draft, review] }
stage:    { $nin: [archived, deleted] }
reviewed: { $exists: true }
tags:     { $all: [rust, async] }
labels:   { $size: 0 }                # exact array length
topics:   { $size: { $gte: 3 } }      # $size also takes count comparisons
```

User frontmatter fields cannot start with `$`, so an operator and a field
name never collide.

### Logical composition

```yaml
$and:
  - status: draft
  - priority: { $gt: 3 }
$or:
  - type: note
  - type: journal
$nor:
  - stage: archived
  - stage: deleted
```

Top-level AND is implicit. Use explicit `$and` when you need the same field
name on multiple sub-clauses (a YAML mapping cannot have duplicate keys).

### Nested fields

Nested fields can be addressed via nested mapping or dotted shorthand; the
two forms are equivalent:

```yaml
author.name: alice
review:
  status: done
```

Field names that themselves contain a literal `.` are not addressable — the
engine always splits paths on `.`.

## Graph operators

Graph operators live alongside frontmatter predicates inside the same filter.
They walk inclusion edges (block-reference inclusion links) or reference
edges (inline links).

### `$key` — identity

```yaml
$key: notes/foo
# $key: { $in: [a, b, c] }                       # any of these
# $key: { $nin: [drafts/scratch, drafts/temp] }  # none of these
```

### Relational operators

| Operator | Reads as | Edge type | Walk parameters |
| --- | --- | --- | --- |
| `$includes` | this doc includes an anchor | inclusion | `maxDepth`, `minDepth` |
| `$includedBy` | this doc is included by an anchor | inclusion | `maxDepth`, `minDepth` |
| `$references` | this doc references an anchor | reference | `maxDistance`, `minDistance` |
| `$referencedBy` | this doc is referenced by an anchor | reference | `maxDistance`, `minDistance` |

Each takes either a scalar key (shorthand for direct edges) or a mapping with
an optional `match`, walk parameters, and an optional `$size`:

```yaml
# Direct edges only — scalar shorthand for { match: { $key: projects/alpha } }
$includedBy: projects/alpha
```

```yaml
# Walk inclusion edges from a single anchor, bounded
$includedBy: { match: { $key: projects/alpha }, maxDepth: 5 }
```

```yaml
# Anchor by frontmatter predicate (every active project)
$includedBy:
  match:
    type: project
    status: active
  maxDepth: 5
```

```yaml
# Range bounds — documents 2 to 3 hops from archive/index
$referencedBy: { match: { $key: archive/index }, minDistance: 2, maxDistance: 3 }
```

In the full mapping form, **omitting `maxDepth` / `maxDistance` means direct
edges** (depth 1). Set `maxDepth: 0` / `maxDistance: 0` for an unbounded walk
that reaches every transitively-related document. Walks are BFS and
de-duplicate via a visited set, so cycles terminate.

```yaml
$includedBy: { match: { $key: projects/alpha }, maxDepth: 0 }  # whole tree
```

`match` is optional and defaults to `{}` (any document); the empty mapping
`$includedBy: {}` is still a parse error.

A relational operator never matches a document in its own anchor set. To
include the anchor, OR it in:

```yaml
$or:
  - $key: projects/alpha
  - $includedBy: { match: { $key: projects/alpha }, maxDepth: 0 }
```

#### Cardinality — `$size`

By default a relational operator holds when at least one document stands in
the relation. Add `$size` to test the *count* of related documents instead.
`$size` takes a non-negative integer (an `$eq` shorthand) or a mapping of
count comparisons (`$eq`, `$ne`, `$gt`, `$gte`, `$lt`, `$lte`); multiple
comparisons AND together. The count is over the distinct documents in the
walk's `[min, max]` band, matching `match`, always excluding the document
itself.

```yaml
$includedBy:   { $size: 0 }              # roots — nothing includes them
$referencedBy: { $size: { $gte: 5 } }    # hubs — 5+ direct referrers
```

```yaml
$includes: { $size: 0 }                  # leaves — include nothing
```

```yaml
$includedBy: { match: { type: project, status: active },
               $size: { $gte: 2 }, maxDepth: 3 }
```

Because the default bounds count direct edges, `$size: 0` is the same whether
bounded or unbounded: zero direct edges means zero at every depth.

### `$content` — content membership

`$content` lifts a block predicate (see Blocks below) into the filter: it
matches documents containing **at least one** block satisfying the predicate.
`$content: {}` matches any document with at least one block.

```yaml
# Drafts, or documents whose content mentions TODO
filter:
  $or:
    - status: draft
    - $content: { $text: "TODO" }
```

```yaml
# Documents under a project that LACK a Status section — absence is
# a quantifier over the whole block set, expressible only in filter
filter:
  $includedBy: { match: { $key: projects/alpha }, maxDepth: 3 }
  $nor:
    - $content: { $header: Status }
```

Like `$key`, it is a noun operator testing the document itself, so it
composes with every other filter clause under `$and` / `$or` / `$nor` and
inside the `match` of any relational operator. `$content` decides membership
only — it selects nothing; which blocks an operation reads or mutates is
designated at the projection or update site.

## Search (`find` only)

`search` is a top-level clause of a `find` operation, beside `filter`. It
selects documents by relevance to a full-text query and supplies the default
ordering. Search leads, filter refines:

```yaml
search:
  lexical: "broken links cleanup"
filter: { type: note }
limit: 5
```

- `lexical: <string>` — BM25 full-text over title + body. `fuzzy: <string>`
  — skim subsequence match over title + key. Both present → RRF fusion,
  exactly as `iwe find --fuzzy --lexical`. At least one is required;
  `search: {}` is a parse-time error, as is any key other than `lexical` /
  `fuzzy`.
- **Search selects and orders.** A document is in the result iff it matches
  the search **and** passes the `filter`, ordered by relevance, ties by key
  ascending. Candidates with no BM25 hit / no skim score are dropped — a
  text query is a filter here, not a sort variant. `limit: 5` always means
  the 5 best documents that pass the filter.
- **`search` + `sort` is legal.** Search contributes membership and the
  default ordering; an explicit `sort` overrides the ordering only. "The 5
  newest documents matching Q" is `search` + `sort: { modified_at: -1 }` +
  `limit: 5`.
- **Corpus-global scores.** The BM25 index is fit to the whole workspace;
  IDF is not re-fit per filter.
- A `lexical` query that stems away to nothing (stop words only) matches
  nothing; the result carries a structured warning so callers see why it is
  empty.
- `search` is `find` only — not `count` / `update` / `delete`.

## Projection (`find` only)

`project` shapes each result to exactly the listed fields; `addFields` keeps
the default fields and adds to them. The two are mutually exclusive. Each
entry maps an output name to a source: `1`, `true`, and YAML null project the
frontmatter field of the same name, a dotted path projects a nested
frontmatter field, and a `$`-selector projects a system field.

```yaml
project:
  title: 1
  priority: meta.priority
  key: $key
  body: $content
  parents: $includedBy
```

The `$`-selectors: `$key`, `$title`, `$titleSlug`, `$content`,
`$frontmatter`, `$includes`, `$includedBy`, `$references`, `$referencedBy`.

Projection can also address blocks inside each matched document — narrowed
bodies, located blocks, grep lines. See Block projection below.

## Sort and limit

```yaml
sort:  { modified_at: -1 }   # 1 = ascending, -1 = descending
limit: 100                   # 0 = no limit
```

Exactly one sort key is accepted. Ties (and the no-sort case) are broken by
document key in ascending lexicographic order.

## Update operators

```yaml
filter: { type: note }
update:
  $set:
    reviewed: true
    audited_at: 2026-04-26
    "review.reviewer": alice
  $unset:
    draft_notes: ""
```

`$set` adds the field if absent, replaces it otherwise. Mapping values
replace wholesale; use dotted shorthand to write subset leaves without
dropping siblings. `$unset` removes fields; values are ignored.

Block update operators — `$replace`, `$replaceText`, `$insertBefore`,
`$insertAfter`, `$append`, `$delete` — live in the same `update` document as
siblings of `$set` / `$unset` and combine freely with them. See Block update
operators below.

### Reserved-prefix protection

Frontmatter field names whose first character is `_`, `$`, `.`, `#`, or `@`
are reserved by the engine. They are invisible to filters, projections, and
sort, and `update` strips them on writeback. Targeting a reserved-prefix
segment in a `$set` or `$unset` path — at any depth — is a parse-time error.

## Blocks

The language also addresses **blocks** — the structural nodes inside a
document: headers, paragraphs, lists, list items, code blocks, quotes,
tables, references, horizontal rules. There is no separate block-selection
clause; blocks enter through three sites the operation document already
owns:

| Site | Addition | Purpose |
| --- | --- | --- |
| `filter` | `$content` operator | Document membership by content |
| `project` / `addFields` | `{ $content: P }` narrowing, `$blocks` and `$matches` sources | Read a slice of a document: narrowed body, located blocks, grep lines |
| `update` | Block operators, each carrying its own selection | Mutate blocks |

One grammar — the **block predicate** — is consumed at all three sites, and
one convention governs every argument shape: **`$`-prefixed keys select,
bare keys configure.** Inside an update operator's argument, the `$`-keys
form the block predicate and the bare keys (`from`, `to`, `content`,
`expect`) are the payload.

Membership stays `filter`'s alone: nothing outside `filter` adds or removes
documents from the result. Deliberately out of scope: computed replacements
(renumbering, case transforms), stateful walks, inline-element rewriting —
the driving agent reads the selection, computes, and applies literal edits.
The typical workflow is two operations sharing one predicate: locate with
`project: { hits: { $blocks: P } }`, then mutate with an update operator
carrying the same `P`.

### Block model

Each block type has its own predicate operator. **Node** operators select the
matching block alone; **tree** operators select the block together with
everything below it. The **own text** column is what text predicates match
against:

| Operator | Sort | Markdown construct | Own text |
| --- | --- | --- | --- |
| `$header` | node | heading (`#`, `##`, …); the blocks of its section are its children | the heading text |
| `$paragraph` | node | paragraph | the paragraph text |
| `$list` | tree | bullet or ordered list — an empty head whose children are the items | none |
| `$quote` | tree | block quote — an empty head wrapping the quoted blocks | none |
| `$item` | node | list item | the item's own line — a loose item's leading paragraph folds into it — excluding nested children |
| `$code` | node | fenced code block | the code text, excluding the fence lines |
| `$table` | node | table | the rows as rendered, one line per row |
| `$ref` | node | block reference (inclusion link) | the authored (piped wikilink) text, when present |
| `$hr` | node | horizontal rule | none |

The tree rooted at a header — the header together with every block below it —
is a **section**: `$header` matches the heading line alone, `$section`
selects the whole tree. Quotes and lists are containers with **empty heads**:
the root carries no own text, so their operators select the container with
its contents, and `$within` peels the wrapper off. A **container** is a block
that can carry children — a header, an item, a list, a quote; containers are
the legal `$append` targets. Raw HTML blocks are outside the model — the
parser drops them.

**Own text.** Text predicates evaluate against a block's own text only,
never descendant text. A header whose section contains `TODO` does not match
`$text: TODO` — the paragraph carrying it does. To select a block *because
of* its contents, use `$contains`. Own texts are pairwise disjoint, which
the update semantics rely on.

**Normalized form.** Matching operates on the normalized rendering of the
document — the same form `iwe retrieve` and every other read emits — at the
byte level: inline markup keeps its markdown syntax (`**bold**` stays
`**bold**`, never `bold`), and inline link URLs are part of own text,
rendered relative to the containing document. Text copied from a read
matches byte-for-byte in a later predicate or `$replaceText` anchor. In
normalized form every block's own text is a single line, except code blocks
and tables — one line per code line or table row, with table cells padded to
their column width.

**Section path.** Every block has a section path: the titles of its
enclosing sections, ordered from the document root down, with top-level
headers omitted — the value block reads emit as `path`. Path elements are
plain text (inline markup stripped); a heading's own text keeps its markdown
syntax — echo own text, not path elements, into text predicates.

**No block identity.** No IDs are minted or written to files; addressing is
by predicate against normalized text, guarded by `expect`. Two byte-identical
sibling blocks under the same section path are indistinguishable — every
mutation touches both or fails its `expect`. The escape hatch is the
whole-body rewrite (`iwe update -k KEY -c CONTENT`).

**Readable but not editable.** The task-checkbox marker (`[ ]` / `[x]`) sits
outside an item's own text: text predicates never see it, no predicate
selects items by state, and `$replaceText` cannot toggle it. Tables select,
render, and travel as units — membership, reads, `$delete`, `$replace`, and
the insertion anchors all work — but `$replaceText` selecting a table is a
validation error; editing inside a table means replacing the table.

### Block predicates

A block predicate is a YAML mapping of `$`-prefixed operators. The empty
predicate `{}` matches every block; top-level keys AND together; unknown
`$`-names and bare keys are parse-time errors.

| Operator | Matches |
| --- | --- |
| `$text: S` | own text contains `S`, case-insensitively; `$text: { $eq: S }` matches the whole own text, also case-insensitively |
| `$matches: REGEX` | own text matches the pattern (Rust regex, case-sensitive — use an inline `(?i)` flag; no backreferences or lookaround) |
| `$header`, `$paragraph`, `$item`, `$code`, `$table` | blocks of one type — the matching block alone, no implicit subtree; the argument is a nested predicate, and a scalar is exact-text shorthand (`$header: Status`) |
| `$ref` | block references, selected by target (`$ref: { $references: KEY }`); the scalar shorthand is a parse-time error, and text predicates match the authored (piped) link text alone |
| `$hr` | horizontal rules; no own text — the scalar shorthand and direct text predicates in the argument are parse-time errors |
| `$section: T` | a header matching `T` together with everything below it; scalar `T` is exact-text shorthand for the root header |
| `$quote: P`, `$list: P` | a quote / a list together with its contents; the root has no own text, so scalars and direct text predicates in the argument are parse-time errors — scope with `$within` or `$contains` |
| `$within: T` | blocks inside the selection — a section's body, a quote's content, at any depth; scalar `T` names a section; a mapping argument must select content: `{}`, or a predicate containing `$section` / `$quote` / `$list` |
| `$contains: P` | blocks with a descendant matching `P`, at any depth |
| `$references: KEY` | blocks whose own content links to `KEY` — a ref targeting `KEY`, or inline text linking to it |
| `$and`, `$or`, `$nor` | logical composition, as in filters |

```yaml
$header: {}                                     # every header: the outline
```

```yaml
$section: Unreleased                            # one section, header included
```

```yaml
$within: Goals                                  # inside Goals, header excluded
```

```yaml
$within: { $section: { $matches: "^Q[0-9]" } }  # inside any quarterly section
```

```yaml
$paragraph: { $references: archive/old-plan }   # paragraphs linking to a document
```

```yaml
{ $header: {}, $contains: { $matches: "(?i)todo" } }  # headers whose section holds a TODO
```

```yaml
$within: { $quote: {} }                         # quoted content, wrapper peeled off
```

Deeper scoping is recursion, not extra syntax — each element of a section
path becomes one `$section` predicate scoped by `$within` to the elements
before it:

```yaml
# Inside Q3, itself inside Goals (matches Goals > 2026 > Q3, not Q3 > Goals)
$within: { $section: { $text: { $eq: Q3 }, $within: Goals } }
```

Conditions inside a type operator's argument and conditions beside it
conjoin identically: `{ $header: { $matches: "^Q" } }` and
`{ $header: {}, $matches: "^Q" }` match the same blocks. Type follows the
block model, not the visual layout — a loose item's leading paragraph folds
into the item, so its text matches `$item`, not `$paragraph`. When unsure of
a block's type, locate with an untyped predicate (`$blocks: { $text: alpha }`)
and read the entry's `type`.

`$within: {}` is the interior of the document itself — every block below the
top level; a top-level block is therefore `$nor: [{ $within: {} }]`.

#### Selection semantics

A block predicate is evaluated against every block in the document, at every
depth, always. Node operators select each matching block alone; tree
operators (`$section`, `$quote`, `$list`) select the matching root together
with every block below it. The result is a **forest**: the selected blocks
arranged by ancestry, in document order. Union over multiple matches is
automatic: when a selector matches several headers, `$within` is the union
of all their interiors.

For sections, three selections line up on one predicate:

| Selection | Predicate | Denotes |
| --- | --- | --- |
| the header alone | `$header: Goals` | one node — the heading line |
| the section — header and contents | `$section: Goals` | the full tree |
| the contents only | `$within: Goals` | the tree minus its root |

Each consumer takes from the forest what it needs: membership (`$content` in
filter) tests that it is non-empty; `$content` in projection renders it;
`$blocks` flattens it to data; the unit update operators act on its roots;
`$within` drops them.

### Block projection

Three projection sources consume a block predicate. Each takes the predicate
in an options mapping; the bare form is the empty-predicate shorthand.

| Source | Projects |
| --- | --- |
| `{ $content: P }` | the document body narrowed to the selected blocks, rendered at their original depth — a string |
| `$blocks` / `{ $blocks: P }` | one entry per selected block: `type`, `path` (enclosing section titles), `text` (own text) |
| `{ $matches: REGEX }` | grep — one entry per matching line: `path` plus the full line as `text` |

```yaml
project:
  key: $key
  toc: { $content: { $header: {} } }                  # headers-only rendering
  notes: { $content: { $section: Unreleased } }       # one section, as it appears
  hits: { $blocks: { $within: Goals, $text: "Q3" } }  # located blocks as data
  found: { $matches: "(?i)todo|fixme" }               # grep lines
```

Bare `$content` is `{ $content: {} }` — every block, the full body. A
matched document in which no block matches projects `""`. Multiple narrowed
entries may coexist in one projection, each with its own predicate.

**`$content` narrowing** renders each selected block's own content in
document order, at its original depth, normalized. Nothing renders that was
not selected; subtrees appear when the predicate selects them (`$section`,
`$within`), never by implication.

**`$blocks`** entries carry `type` (the operator name without the `$`),
`path`, and `text` (the own text). A header's entry carries its heading
text, not its section — use `$content` narrowing to read bodies. Every entry
carries `text` — `""` for blocks without own text; a ref entry additionally
carries `target`. The entry count is the number `$replaceText`'s `expect`
pins; a tree selection lists every block of its trees flattened, so it
over-counts unit-operator targets — list those by selecting the roots as
nodes (a `$section: P` previews as `$header: P`).

**`$matches`** applies the pattern to the own text of every block in scope
and reports one entry per matching line, in document order — the full line,
not the matched substring, and never the whole block. The argument is a
regex scalar, or a mapping combining the pattern with a scope — bare
`pattern` is the payload, `$`-keys select:

```yaml
project:
  found: { $matches: { pattern: "(?i)todo", $within: Goals } }
```

#### Top-level form

`project` (not `addFields`) also accepts a block predicate directly in place
of the field map — any `$`-key at the top level selects this reading, and
the result carries the document's `key` plus a `content` field with the
narrowed body:

```yaml
project: { $header: {} }        # the headers-only form of each document
```

```yaml
# exactly the same as
project:
  key: $key
  content: { $content: { $header: {} } }
```

Top-level keys conjoin as in any block predicate —
`project: { $header: {}, $within: Usage }` is the table of contents of one
section. Mixing bare and `$`-keys is a parse-time error, `project: {}` keeps
its meaning of an explicit empty projection, and the projection sources are
not predicate operators — `project: { $blocks: P }` is an unknown-operator
error, and a top-level `$matches` reads as the block-predicate operator,
never the grep source.

### Block update operators

Block operators live directly in the `update` document, as siblings of
`$set` and `$unset`. Every update operator is an *(address, payload)* pair:
`$set` addresses by field path, the block operators address by block
predicate. An operator's argument is one flat mapping — the `$`-keys form
the predicate, the bare keys are the payload and the optional `expect`
guard. The bare-key set per operator is closed; unknown bare keys are
parse-time errors. Each operator appears at most once per update document;
applying one operator to several independent selections requires several
operations.

| Operator | Payload | Effect per target |
| --- | --- | --- |
| `$replace` | `content` (markdown) | Replace the target as selected — a tree target wholesale, a node target as its own line(s); on a header node, a heading payload retitles the section |
| `$replaceText` | `from` (optional), `to` | With `from`: replace that substring of the block's own text with `to`. Without `from`: `to` replaces the entire own text |
| `$insertBefore` | `content` (markdown) | Insert sibling content before the target |
| `$insertAfter` | `content` (markdown) | Insert sibling content after the target — after a header node: directly below the heading line; after a `$section`: below the whole tree |
| `$append` | `content` (markdown) | Append child content at the end of the block (containers only — a header: at the end of its section; an item: after its nested blocks; a list: after its last item; a quote: after its last block) |
| `$delete` | — | Remove the target as selected — a `$section` with its contents; a bare `$header` dissolves into its enclosing section |

```yaml
filter:
  $content: { $text: "Q3" }
update:
  $set: { reviewed: true }
  $replaceText:
    $within: Goals
    $text: "Q3 Milestones"
    from: "Q3 Milestones"
    to: "Q3 2026 Milestones"
    expect: 1
  $delete:
    $paragraph: { $references: archive/old-plan }
```

An argument with no `$`-keys carries the empty predicate and selects **all
blocks**: `$delete: {}` clears the document body — title header included —
bounded by the required `filter` and guardable with `expect`.

#### Targets and coalescing

The unit operators (`$replace`, `$insertBefore`, `$insertAfter`, `$append`,
`$delete`) act on **targets**: the selection's forest roots, each taken *as
selected* — a tree root (`$section`, `$quote`, `$list`) is the whole tree, a
node root is the block alone, never its children. This is the same rule
projection follows: `$content: { $header: Goals }` renders one heading line,
and `$delete: { $header: Goals }` removes one heading line — the predicate
means the same thing at every site.

Targets **coalesce by extent**: a target lying inside another target's
extent is absorbed by the outer one — selecting a section and a paragraph
within it yields one target, the section, so `$delete: { $section: {} }`
deletes each outermost section once. Node targets never absorb one another:
own texts are pairwise disjoint, so two node targets always name disjoint
lines. Operators apply once per target, in document order.

Insertions are **anchored**: inserted content stays adjacent to its target,
so when `$insertAfter` on a block and `$insertBefore` on its following
sibling name the same gap, each lands hugging its anchor.

**`$replaceText`** operates on own text and applies to **every selected
block, un-coalesced**. A selected header's `$replaceText` edits the heading
text and nothing below it; a selected ref's edits its authored link text,
and the target key never changes. `from` is optional. When present it must
occur **exactly once** in each selected block's own text, matched
**byte-exact** against the normalized form — case-sensitive, unlike `$text`;
the workflow is to echo text read from `$blocks` output. When omitted, `to`
replaces the block's **entire own text** — renaming a header is
`{ $header: Goals, to: Aims }`; pair the `from`-less form with a
text-bearing predicate or an `expect` guard. A selected block with no own
text (a quote or list root, a horizontal rule, a ref without authored text)
or a table is a validation error.

**Payload markdown** (`content`) is parsed and normalized on write. Heading
levels inside a supplied fragment define nesting within the fragment only;
the fragment's root level is set by its attachment point — child of the
target for `$append`, sibling of the target for the insertions, the target's
own position for `$replace`. At a list attachment point — sibling of an
item, child of a list — the fragment must parse as a single list; its items
attach, and the target keeps its own list type.

#### Headers: retitle, dissolve, remove, clear

Every unit edit splices the target's lines in the normalized rendering; the
document re-parses, and normalization re-derives nesting and header levels
from the result. Headers are where the node/tree distinction matters — the
three selections become three different operations:

| Selection | `$delete` | `$replace: { content: C }` |
| --- | --- | --- |
| `$header: Goals` — the heading line | **Dissolve**: the heading line is removed; the former contents re-attach to the enclosing section and re-level | C splices at the heading line. A heading payload **retitles** — the former contents re-nest beneath it; any other payload dissolves the section around it |
| `$section: Goals` — header and contents | **Remove**: the header and every block below it | C replaces the whole section |
| `$within: Goals` — contents only | **Clear**: the header stays, its blocks go | C replaces each content target |

Against

```markdown
# Roadmap

## Goals

Ship the editor integration

### Q3 Milestones

Deliver block operations spec
```

`$delete: { $header: Goals }` yields

```markdown
# Roadmap

Ship the editor integration

## Q3 Milestones

Deliver block operations spec
```

— the paragraph re-attaches to `Roadmap`, and `Q3 Milestones` re-levels from
`###` to `##`. Re-leveling ripples through the whole former subtree: after a
header edit, expect level changes below the splice point.
`$replace: { $header: Goals, content: "## Aims" }` retitles — the same
document with `## Aims` in place of `## Goals`, contents intact; the
payload's own level is advisory, since normalization re-levels to the
attachment point. The destructive whole-section forms require naming the
tree: `$delete: { $section: Goals }`.

The asymmetry of mistakes is deliberate: deleting a header node when the
section was meant leaves visible, recoverable content behind; the reverse
mistake under subtree semantics would silently destroy a section. The
intuition is the text editor's — deleting a heading line does not delete the
text under it.

`$replace` is per-target, never "set the body to C": against a body with
three top-level blocks, `$replace: { content: X }` yields `X` three times.
Whole-body replacement is the CLI body-overwrite mode; a single-target
`$replace` is guarded with `expect: 1`.

#### Combining operators

`$set`, `$unset`, and block operators combine freely in one update document
and apply atomically per matched document. Block operator applications must
be **pairwise disjoint by extent** after coalescing: no two applications may
touch the same block, or a tree target and anything within it — a `$delete`
taking the Goals *section* while a `$replaceText` edits a paragraph inside
it is a validation error listing the pair, nothing written. A `$delete` of
the Goals *header node* beside the same `$replaceText` is legal: the heading
line and the paragraph are disjoint extents.

Frontmatter operators apply to **every** filter-matched document regardless
of block selections — membership is `filter`'s alone. To restrict a combined
operation to documents where the block edit lands, conjoin `$content` into
the filter with the same predicate:

```yaml
filter:
  status: active
  $content: { $text: "Q3" }
update:
  $set: { reviewed: true }
  $replaceText: { $text: "Q3", from: "Q3", to: "Q3 2026" }
```

Drop the `$content` line and `reviewed` is stamped on every `active`
document while the text edit lands only where `Q3` occurs.

### `expect` guards

Any block operator application may carry `expect: N` or
`expect: { min: M, max: N }` as a bare key, asserting the number of
applications it will make, counted across all mutated documents
(post-`limit`):

- The **unit operators** count **targets** — the coalesced roots: one
  section, however many blocks it holds, is one target, so
  `$delete: { $section: Goals, expect: 1 }` passes.
- **`$replaceText`** counts selected **blocks**, un-coalesced, matching its
  per-block application.

`update` and `delete` operation documents additionally take a
**document-level** `expect` — a top-level clause, sibling of `filter` /
`sort` / `limit` — asserting the number of matched documents the operation
will write. In a `find` or `count` operation a top-level `expect` is a
parse-time error. The two levels are independent quantities.

On violation the whole operation fails before writing anything; the error
reports the actual count plus each target as
`key › section path › first line of own text` (documents as `key › title`).
`expect: 1` is the single-target precision edit; `expect: 0` is legal and
asserts an empty selection.

```yaml
# Bounded cleanup: delete completed task paragraphs, refusing a runaway match
filter: { type: meeting }
update:
  $delete:
    $paragraph: { $matches: "^DONE " }
    expect: { max: 20 }
```

### Validation and atomicity

The operation validates fully before writing anything, anywhere.

**Parse time:** unknown operators; unknown bare keys in an operator
argument; invalid payload types; regex that fails to compile; a `$within`
argument that cannot select content; block operators in a `find` / `count` /
`delete` operation.

**Evaluation time**, against the original normalized documents — every
selection and anchor position is resolved before any edit applies — in
order:

1. **Type compatibility** — `$append` on a non-container block;
   `$replaceText` selecting a block with no own text or a table; a `content`
   payload that cannot attach at its site. The error lists every offending
   block.
2. **`$replaceText` anchors** — a given `from` absent or occurring more than
   once in any selected block's own text.
3. **Disjointness** — block-operator applications with overlapping extents.
   The error lists the pairs.
4. **`expect`** — any violated guard, block-level or document-level.

Any failure aborts the whole operation: frontmatter operators write nothing
if a block operator fails validation, and no document is written if any
document fails. Per document, frontmatter and block edits commit as one
atomic rewrite.

### Strict mode

The `expect` guards are optional in the language; strictness is a property
of the **surface**, not the grammar — the same operation document is valid
everywhere; a strict surface refuses to *run* an unguarded mutation.

- **CLI**: `iwe update` and `iwe delete` take `--strict`. Under the flag,
  every mutating application must carry its guard — the operation's
  document-level `--expect` and each block operator's `expect`. A missing
  guard is an error before anything runs.
- **MCP**: the `iwe_query` tool is always strict, no opt-out.

`--dry-run` writes nothing and is exempt from strict mode — it is how the
counts are learned: dry-run the mutation, read the matched documents, pin
`expect` to what it shows, re-run. An agent that just located its targets
with `$blocks` already knows the counts.

### Locate, then mutate

The agent workflow is two operations sharing one predicate:

```yaml
# 1. Locate (read)
filter:
  $includedBy: { match: { $key: projects/roadmap }, maxDepth: 3 }
  $content: { $within: Goals, $text: "Q3" }
project:
  key: $key
  hits: { $blocks: { $within: Goals, $text: "Q3" } }
```

```yaml
# 2. Mutate (same predicate, now inside the operator)
filter:
  $includedBy: { match: { $key: projects/roadmap }, maxDepth: 3 }
update:
  $replaceText:
    $within: Goals
    $text: "Q3 Milestones"
    from: "Q3 Milestones"
    to: "Q3 2026 Milestones"
    expect: 1
```

## CLI flags

On the CLI, structural anchor flags lower to graph operators. A
`KEY[:DEPTH]` suffix sets `maxDepth` (or `maxDistance`) for that anchor;
depth `0` is the unbounded sentinel.

| CLI flag | Lowers to |
| --- | --- |
| `-k KEY` | `$key: KEY` (1 key = `$eq`; 2+ = `$in`) |
| `--includes KEY` | `$includes: KEY` (scalar shorthand, depth 1) |
| `--included-by KEY:5` | `$includedBy: { match: { $key: KEY }, maxDepth: 5 }` |
| `--references KEY:0` | `$references: { match: { $key: KEY }, maxDistance: 0 }` (unbounded) |
| `--referenced-by KEY` | `$referencedBy: KEY` |
| `--max-depth N` | session default for `--includes` / `--included-by` (default 1) |
| `--max-distance N` | session default for `--references` / `--referenced-by` (default 1) |
| `--filter "EXPR"` | inline YAML filter document |
| `--project "EXPR"` | `project: EXPR` — comma list (`title,author`, `body=$content`, bare `$blocks`) or inline YAML mapping (find, tree) |
| `--add-fields "EXPR"` | `addFields: EXPR` — same grammar as `--project`, extends the defaults (find, tree) |
| `--sort field:1` / `--sort field:-1` | `sort: { field: 1 / -1 }` (find only) |
| `-l, --limit N` | `limit: N` (find, count) |
| `--blocks "PRED"` | `addFields: { blocks: { $blocks: PRED } }` (find only) |
| `--matches PATTERN` | `filter: { $content: { $matches: PATTERN } }` **and** `addFields: { matches: { $matches: PATTERN } }` (find only) |
| `--set FIELD=VALUE` | `$set: { FIELD: VALUE }` (update only; repeatable) |
| `--unset FIELD` | `$unset: { FIELD: "" }` (update only; repeatable) |
| `--replace`, `--replace-text`, `--insert-before`, `--insert-after`, `--append`, `--delete` `"ARG"` | one block-operator entry (`$replace`, `$replaceText`, …) in the `update` document (update only) |
| `--expect VAL` | the document-level `expect` clause (update, delete) |
| `--strict` | surface policy, not grammar: requires the `expect` guards on every mutating application (update, delete) |

All filter flags AND together. For OR or NOR, write the composition inside
`--filter`:

```bash
iwe find --filter '$or: [{ status: draft }, { status: review }]'
iwe find --filter '$nor: [{ status: archived }]'
```

`--matches` is a composite flag — it lowers to both a membership clause and
a projection entry, because one-flag grep is the point. Each block-operator
flag's argument is that operator's `{ $selector…, payload… }` mapping — the
operator name is the flag, so the `$replaceText:` wrapper is dropped.

Combining `-k KEY` with a `--filter` whose top level also contains `$key` is
a parse-time error — pick one source, or use `-k a -k b` for multi-key
match.
