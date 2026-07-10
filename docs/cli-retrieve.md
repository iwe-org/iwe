# IWE Retrieve

Retrieves document content with graph expansion and relationship context. Designed for AI agents to navigate and understand the knowledge graph.

`retrieve` assembles reading context around a set of documents in a single call. It has two moving parts: **seeds** — the documents to read, given directly by `-k` / `--filter` / anchors, or found by a `--lexical` / `--fuzzy` search — and **expansion** — the graph directions to follow out from each seed, given by the `--expand-*` flags.

## Usage

``` bash
iwe retrieve [OPTIONS]
iwe retrieve -k KEY --expand-includes 1 --expand-included-by 1
iwe retrieve --lexical "QUESTION" --limit 5 --expand-references 1 --expand-included-by 1
```

Without `-k` (and without a search flag), reads the document key from stdin (for piping).

## Options

| Flag                  | Description                                                            | Default  |
| --------------------- | ---------------------------------------------------------------------- | -------- |
| `-k, --key <KEY>`              | Document key(s) to retrieve, or the candidate set searched within when a search flag is present (repeatable). 1 key = `$eq`, 2+ = `$in`. | stdin      |
| `--expand-includes [N]`        | Follow inclusion edges downward, pulling child (sub-)documents to depth `N`. Bare = `1`; `0` = unbounded; omitted = not followed. | not followed |
| `--expand-included-by [N]`     | Follow inclusion edges upward, pulling parent documents to depth `N`. Bare = `1`; `0` = unbounded; omitted = not followed. | not followed |
| `--expand-references [N]`      | Follow outbound reference links, pulling documents this seed links to within `N` hops. Bare = `1`; `0` = unbounded; omitted = not followed. | not followed |
| `--expand-referenced-by [N]`   | Follow inbound reference links, pulling documents that link to this seed within `N` hops. Bare = `1`; `0` = unbounded; omitted = not followed. | not followed |
| `--lexical <Q>`                | Seed search: BM25 full-text query on title and body. Runs a seed query and switches to the search-indexed loader.                     | none       |
| `--fuzzy <Q>`                  | Seed search: fuzzy query on title and key. Combine with `--lexical` to fuse (RRF).                                                    | none       |
| `-e, --exclude <KEY>`          | Exclude document key(s) from results (repeatable).                                    | none       |
| `-b, --backlinks`              | Populate the `referencedBy` edge list in output (does not add documents).             | true       |
| `--children`                   | Populate the `includes` edge list in output (does not add documents).                 | false      |
| `-f, --format <FMT>`           | Output format: `markdown`, `keys`, `json`, `yaml`.                                    | `markdown` |
| `--filter <EXPR>`              | Inline YAML filter expression restricting the candidate set. See [Query Language](query-language.md). | none       |
| `--includes <KEY[:DEPTH]>`     | `$includes` anchor restricting the candidate set. Repeatable; anchors are ANDed.      | none       |
| `--included-by <KEY[:DEPTH]>`  | `$includedBy` anchor restricting the candidate set. Repeatable; anchors are ANDed.    | none       |
| `--references <KEY[:DIST]>`    | `$references` anchor restricting the candidate set. Repeatable; anchors are ANDed.    | none       |
| `--referenced-by <KEY[:DIST]>` | `$referencedBy` anchor restricting the candidate set. Repeatable; anchors are ANDed.  | none       |
| `--max-depth <N>`              | Session default for inclusion anchor flags without a colon-suffix. `0` = unbounded.   | 1          |
| `--max-distance <N>`           | Session default for reference anchor flags without a colon-suffix. `0` = unbounded.   | 1          |
| `--limit <N>`                  | Cap the number of seed documents kept **before** expansion — top-N by relevance when searching, the first N of the selection otherwise. `0` = unlimited. | unlimited  |
| `--max-documents <N>`          | Cap the number of documents returned **after** expansion, trimming periphery documents first. `0` = unlimited. | unlimited  |
| `--max-tokens <N>`             | Cap total content tokens across all documents (whole documents are dropped). `0` = unlimited. | unlimited  |
| `--max-document-tokens <N>`         | Cap content tokens per document (body is head-truncated with a marker). `0` = unlimited. | unlimited  |

> **Note.** The colon-suffix (`KEY:DEPTH`) on the anchor *selection* flags above and the depth values on the `--expand-*` flags are unrelated: the anchor flags **restrict which documents are seeds**, the `--expand-*` flags **follow edges out from the seeds**.

## Expansion (`--expand-*`)

Expansion pulls related documents into the result set. Each direction is its own flag; the value is a depth (bare flag = `1`, `0` = **unbounded**, omitted = **not followed**):

| Flag | Follows | Adds |
| --- | --- | --- |
| `--expand-includes [N]` | inclusion edges downward | child (sub-)documents |
| `--expand-included-by [N]` | inclusion edges upward | parent documents |
| `--expand-references [N]` | outbound reference (inline) links | documents this seed links to |
| `--expand-referenced-by [N]` | inbound reference links | documents that link to this seed |

``` bash
--expand-references 1 --expand-included-by 1   # one hop of outbound refs + one level of parents
--expand-includes 2                            # children two levels deep
--expand-included-by 0                          # every ancestor (0 = unbounded)
--expand-includes                               # bare flag = one level
```

A non-integer or negative value is an error.

**Expansion is doc-only by default.** With no `--expand-*` flag (and no deprecated `-d` / `-c` / `-l`), `retrieve` returns exactly the requested document(s) with no expansion. The old implicit default is now written explicitly as `--expand-includes 1 --expand-included-by 1`.

`includedBy` is always listed in the output frontmatter regardless of expansion; `-b` and `--children` toggle the `referencedBy` and `includes` edge lists; `references` is listed when a `--expand-references` expansion is requested. Edge-list toggles never add documents.

## Search seeds

When `--lexical` or `--fuzzy` is present, `retrieve` first runs a **seed query** — a `find` with that search over the candidate set (`-k` / `--filter` / anchors) — then reads and expands the ordered seeds. Seeds come out first, in relevance order, followed by their expansion; seeds survive token-budget trimming first. `--limit` caps the seeds kept (top-N by relevance) before expansion.

Because search **restricts**, `-k a -k b -k c --lexical Q` searches *within* those three keys: any that do not match the query drop from the output. `-k` is candidate restriction here, not a guaranteed read.

The standing/hub pages an agent needs are reached structurally rather than pinned: search finds the episode pages, and one expansion direction brings the hubs in — `includedBy` when hubs include their episodes, `referencedBy` when a hub timeline links to them, `references` when episodes link back to the hub.

### Output limits

`--max-documents`, `--max-tokens`, and `--max-document-tokens` bound output for context-limited callers and are **off by default**. (`--limit` is not an output budget — it caps the *seeds* before expansion.)

- **What is counted.** Each document's body plus its serialized edge lists. JSON/YAML structural overhead (field names, escaping) is not counted, and the counts are an approximate (OpenAI-BPE) proxy — so treat the budget as a soft bound with headroom, not an exact byte cap. The `⋯ truncated` marker appended to a clipped body is itself uncounted, so a clipped document lands slightly over `--max-document-tokens`.
- **What survives.** The requested document(s) are collected first, so they always survive; `--max-documents` and `--max-tokens` trim from the periphery (deepest expansion and parent context) inward. A clipped result set is therefore **not relationally closed** — a kept child may lose the parent-context document it appeared under.
- **Signal.** When any limit trims the output, a `warning:` line is printed to **stderr** while stdout stays a clean array/markdown. Page through a large result set by combining `--exclude` with the truncation signal: fetch, note the returned keys, re-run excluding them.

### Filter and structural anchors

When filter or anchor flags are provided, `retrieve` reads the documents that match. With both `-k` and a filter expression, the result is the **intersection** — `-k` clauses AND with the filter at the top level.

``` bash
# Retrieve every doc under projects/alpha (unbounded), no expansion
iwe retrieve --included-by projects/alpha:0

# Sub-documents of two anchors (intersection)
iwe retrieve --included-by projects/alpha --included-by people/dmytro

# Restrict explicit keys to those inside an anchor's subtree
iwe retrieve -k x -k y -k z --included-by projects/alpha

# Filter by frontmatter
iwe retrieve --filter 'status: draft'
```

See [Query Language](query-language.md) for the full filter syntax.


## How It Works

The `retrieve` command collects documents in a specific order:

1.  **Seeds** - the requested documents (`-k` / `--filter` / anchors, or the ordered results of a `--lexical` / `--fuzzy` seed query)
2.  **Children** (`--expand-includes N`) - Documents included via [Inclusion Links](inclusion-links.md), expanded N levels deep
3.  **Context** (`--expand-included-by N`) - Parent documents of each seed, up N levels
4.  **Sub-document parents** - Parents of direct sub-documents (only when both `--expand-includes` and `--expand-included-by` are set)
5.  **References** (`--expand-references N`) - Documents referenced inline, within N hops
6.  **Referenced by** (`--expand-referenced-by N`) - Documents that reference the seed, within N hops

Documents are deduplicated - each document appears only once in the output.

### Depth Expansion (`--expand-includes`)

Controls how deep to follow children (sub-documents):

- **omitted**: Only the seed, no downward expansion (the default)
- **`--expand-includes 1`**: Seed + direct sub-documents
- **`--expand-includes 2`**: Seed + sub-documents + their sub-documents
- **`--expand-includes 0`**: Every descendant (unbounded)

### Context Expansion (`--expand-included-by`)

Controls how many levels of parent documents to include:

- **omitted**: No parent context documents included (the default)
- **`--expand-included-by 1`**: Direct parents of the seed and of its immediate sub-documents
- **`--expand-included-by 0`**: Every ancestor (unbounded)

### Sub-document Parent Context

When you retrieve a document with both `--expand-includes` and `--expand-included-by` set, the command also fetches parent documents of any sub-documents. This provides important context: the parent document often defines what *type* of thing the sub-document is.

**Example: A component used in multiple projects**

Consider a `button` component embedded in two different projects. When you retrieve `mobile-app`:

```
iwe retrieve -k mobile-app --expand-includes 1 --expand-included-by 1
```

The output for the `button` sub-document shows its other parents:

``` markdown
# Button

[Button](button) <- [Web Dashboard](web-dashboard)

A reusable button component.
```

The `<-` notation lists other documents where `button` is embedded. Since we're already viewing `button` from within `mobile-app`, only `web-dashboard` is shown — revealing that this component is shared across projects.

**Result includes:**

1.  `mobile-app` — the seed document
2.  `button` — sub-document (via `includes: 1`)
3.  `web-dashboard` — another parent of `button` (via `includedBy: 1`)

The `web-dashboard` document is fetched because it's a parent of `button`. This context helps understand what the component is and where else it's used.

**Note:** Only parents of *direct* sub-documents are included.

### References (`references`)

When expanded, includes documents that are referenced inline (not as inclusion links):

``` markdown
# Main Document

See [Related Topic](related-topic) for more details.
```

With `--expand-references 1`, `related-topic` would be included in the output. Distances greater than 1 follow the reference chain transitively.

## Document Relationships

### Parent Documents vs Back Links

**Parent Documents** - Inclusion links (document embedded in another):

``` markdown
# Parent Document

## Overview

[child](child)   <- Inclusion link creates parent-child relationship
```

The child document shows: `**Parent documents:** [Parent Document](parent) > Overview`

**Back Links** - Inline references (links within text):

``` markdown
# Some Document

See [target](target) for more details.   <- Inline reference creates backlink
```

The target document shows: `**Back links:** [Some Document](some-document)`

## Output Format

### Markdown Format (default)

Documents are rendered with YAML frontmatter containing metadata, followed by the document content:

``` markdown
---
document:
  key: my-document
  title: My Document
  includedBy:
  - key: index
    title: Index
  referencedBy:
  - key: related-doc
    title: Related Doc
---

# My Document

Original document content preserved exactly as written.
```

Each document in the result set includes:

- YAML frontmatter with `key`, `title`, `includedBy`, and `referencedBy`
- Document content with original headers preserved
- Two empty lines after each document for easy parsing

Multiple documents are separated by their frontmatter delimiters (`---`), no horizontal rules.

### Keys Format (`-f keys`)

```
my-document
child-document
parent-document
```

One key per line, suitable for piping to other commands or building exclude lists.

### JSON Format (`-f json`)

``` json
[
  {
    "key": "my-document",
    "title": "My Document",
    "content": "# My Document\n\nContent here...",
    "includedBy": [
      {
        "key": "index",
        "title": "Index",
        "sectionPath": ["Topics", "My Topic"]
      }
    ],
    "includes": [],
    "referencedBy": [
      {
        "key": "related-doc",
        "title": "Related Doc",
        "sectionPath": []
      }
    ]
  }
]
```

The top-level value is a bare array of document objects. Each carries `key`, `title`, `content`, and three edge arrays — `includedBy`, `includes`, `referencedBy` — using the same `EdgeRef { key, title, sectionPath }` shape. Empty arrays are emitted explicitly.

`includes` is populated only when `--children` is passed.

## Examples

``` bash
# Retrieve a single document (doc only, no expansion)
iwe retrieve -k my-document

# Retrieve multiple documents at once
iwe retrieve -k doc1 -k doc2 -k doc3

# Immediate context: direct children + direct parents
iwe retrieve -k my-document --expand-includes 1 --expand-included-by 1

# Deep expansion (3 levels of sub-documents)
iwe retrieve -k my-document --expand-includes 3

# More parent context
iwe retrieve -k my-document --expand-included-by 2

# Include inline referenced documents
iwe retrieve -k my-document --expand-references 1

# One-shot search + expand: find seeds, then pull structure
iwe retrieve --lexical "auth token rotation" --limit 5 \
  --expand-references 1 --expand-included-by 1 -f json

# Exclude documents which you don't need
iwe retrieve -k my-document -e already-loaded -e another-loaded

# Populate the `includes` edge list with child document edges
iwe retrieve -k my-document --children

# Get JSON output for programmatic processing
iwe retrieve -k my-document -f json

# Or YAML
iwe retrieve -k my-document -f yaml

# Pipe keys from stdin (one per line)
echo -e "doc1\ndoc2\ndoc3" | iwe retrieve

# Without backlinks (cleaner output)
iwe retrieve -k my-document -b false
```

## Use Cases

### AI Agent Context Building

Build rich context for AI agents navigating the knowledge base:

``` bash
# Get a document with immediate context
iwe retrieve -k authentication --expand-includes 1 --expand-included-by 1

# Assemble a dossier from a question in one call
iwe retrieve --lexical "how does session expiry work" --limit 5 \
  --expand-references 1 --expand-included-by 1
```

### Understanding Document Relationships

``` bash
# See where a document is used (parent documents in output)
iwe retrieve -k my-topic

# See the full context including inline references
iwe retrieve -k my-topic --expand-references 1
```

### Exploring Knowledge Base Structure

``` bash
# Start from an entry point and expand
iwe retrieve -k index --expand-includes 2

# Get JSON for analysis
iwe retrieve -k project-overview --expand-includes 1 -f json
```

### Chaining Retrievals

Use the `keys` format to chain retrieval operations and exclude already-fetched documents:

``` bash
# Get keys from first retrieval
KEYS_A=$(iwe retrieve -k topic-a -f keys)

# Retrieve for topic-b, excluding keys from topic-a
iwe retrieve -k topic-b -e $KEYS_A

# Or using command substitution with xargs
iwe retrieve -k topic-b $(iwe retrieve -k topic-a -f keys | xargs -I {} echo "-e {}")
```

## Deprecated aliases

The following flags pre-date the query language and remain accepted as hidden aliases for backward compatibility.

| Deprecated         | Use instead                                                                 |
| ------------------ | --------------------------------------------------------------------------- |
| `-d, --depth N`    | `--expand-includes N` (legacy `0` = off, not unbounded)                     |
| `-c, --context N`  | `--expand-included-by N` (legacy `0` = off)                                 |
| `-l, --links`      | `--expand-references 1`                                                     |
| `--in KEY[:N]`     | `--included-by KEY[:N]`                                                     |
| `--in-any K1 K2`   | `--filter '$or: [{ $includedBy: K1 }, { $includedBy: K2 }]'`                |
| `--not-in KEY`     | `--filter '$nor: [{ $includedBy: KEY }]'`                                   |
| `--refs-to KEY`    | `--references KEY` (legacy semantics: ORs `$includes` and `$references`)    |
| `--refs-from KEY`  | `--referenced-by KEY` (legacy semantics: ORs `$includedBy` and `$referencedBy`) |

`-d` / `-c` / `-l` keep their legacy **`0` = off** meaning — they never expressed unbounded, whereas the `--expand-*` flags treat `0` as the unbounded sentinel. Passing a legacy alias together with the `--expand-*` flag for the same direction (e.g. `-d` and `--expand-includes`) is an error.

## Technical Notes

- Documents use YAML frontmatter for metadata, content follows with original formatting
- Empty `includedBy` or `referencedBy` fields are omitted from markdown frontmatter (JSON/YAML output always emits them as `[]`)
- Original document headers are preserved (no level shifting)
- Two empty lines separate documents for easier parsing
- Duplicate documents are automatically filtered out
- Sub-document parent context only includes parents of direct (first-level) sub-documents, not nested ones
