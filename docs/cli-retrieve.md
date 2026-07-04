# IWE Retrieve

Retrieves document content with graph expansion and relationship context. Designed for AI agents to navigate and understand the knowledge graph.

## Usage

``` bash
iwe retrieve [OPTIONS]
```

Without `-k`, reads the document key from stdin (for piping).

## Options

| Flag                  | Description                                                            | Default  |
| --------------------- | ---------------------------------------------------------------------- | -------- |
| `-k, --key <KEY>`              | Document key(s) to retrieve (repeatable). 1 key = `$eq`, 2+ = `$in`.                  | stdin      |
| `-e, --exclude <KEY>`          | Exclude document key(s) from results (repeatable).                                    | none       |
| `-d, --depth <N>`              | Follow children down N levels.                                                        | 1          |
| `-c, --context <N>`            | Include N levels of parent context.                                                   | 1          |
| `-l, --links`                  | Include inline referenced documents.                                                  | false      |
| `-b, --backlinks`              | Include incoming references in output.                                                | true       |
| `-f, --format <FMT>`           | Output format: `markdown`, `keys`, `json`, `yaml`.                                    | `markdown` |
| `--filter <EXPR>`              | Inline YAML filter expression. See [Query Language](query-language.md).               | none       |
| `--includes <KEY[:DEPTH]>`     | `$includes` anchor. Repeatable; anchors are ANDed.                                    | none       |
| `--included-by <KEY[:DEPTH]>`  | `$includedBy` anchor. Repeatable; anchors are ANDed.                                  | none       |
| `--references <KEY[:DIST]>`    | `$references` anchor. Repeatable; anchors are ANDed.                                  | none       |
| `--referenced-by <KEY[:DIST]>` | `$referencedBy` anchor. Repeatable; anchors are ANDed.                                | none       |
| `--max-depth <N>`              | Session default for inclusion anchor flags without a colon-suffix. `0` = unbounded.   | 1          |
| `--max-distance <N>`           | Session default for reference anchor flags without a colon-suffix. `0` = unbounded.   | 1          |
| `--limit <N>`                  | Maximum number of documents to return. `0` = unlimited.                               | unlimited  |
| `--max-tokens <N>`             | Cap total content tokens across all documents (whole documents are dropped). `0` = unlimited. | unlimited  |
| `--max-document-tokens <N>`         | Cap content tokens per document (body is head-truncated with a marker). `0` = unlimited. | unlimited  |

### Output limits

`--limit`, `--max-tokens`, and `--max-document-tokens` bound output for context-limited callers and are **off by default**.

- **What is counted.** Each document's body plus its serialized edge lists. JSON/YAML structural overhead (field names, escaping) is not counted, and the counts are an approximate (OpenAI-BPE) proxy — so treat the budget as a soft bound with headroom, not an exact byte cap. The `⋯ truncated` marker appended to a clipped body is itself uncounted, so a clipped document lands slightly over `--max-document-tokens`.
- **What survives.** The requested document(s) are collected first, so they always survive; `--max-tokens` trims from the periphery (deepest expansion and parent context) inward. A clipped result set is therefore **not relationally closed** — a kept child may lose the parent-context document it appeared under.
- **Signal.** When any limit trims the output, a `warning:` line is printed to **stderr** while stdout stays a clean array/markdown. Page through a large result set by combining `--exclude` with the truncation signal: fetch, note the returned keys, re-run excluding them.

### Filter and structural anchors

When filter or anchor flags are provided, `retrieve` reads the documents that match. With both `-k` and a filter expression, the result is the **intersection** — `-k` clauses AND with the filter at the top level.

``` bash
# Retrieve every doc under projects/alpha (unbounded)
iwe retrieve --included-by projects/alpha:0 --depth 0

# Sub-documents of two anchors (intersection)
iwe retrieve --included-by projects/alpha --included-by people/dmytro --depth 0

# Restrict explicit keys to those inside an anchor's subtree
iwe retrieve -k x -k y -k z --included-by projects/alpha

# Filter by frontmatter
iwe retrieve --filter 'status: draft' --depth 0
```

See [Query Language](query-language.md) for the full filter syntax.


## How It Works

The `retrieve` command collects documents in a specific order:

1.  **Main document** - The document specified by `-k`
2.  **Children** (`-d N`) - Documents included via [Inclusion Links](inclusion-links.md), expanded N levels deep
3.  **Context** (`-c N`) - Parent documents of the main document, up N levels
4.  **Sub-document parents** - Parents of direct sub-documents (only when both `-d > 0` and `-c > 0`)
5.  **Links** (`-l`) - Documents referenced inline within the main document

Documents are deduplicated - each document appears only once in the output.

### Depth Expansion (-d)

Controls how deep to follow children (sub-documents):

- **`-d 0`**: Only the main document, no expansion
- **`-d 1`** (default): Main document + direct sub-documents
- **`-d 2`**: Main + sub-documents + their sub-documents
- **`-d N`**: Follow children up to N levels deep

### Context Expansion (-c)

Controls how many levels of parent documents to include:

- **`-c 0`**: No parent context documents included
- **`-c 1`** (default): Include direct parents of main document and direct parents of immediate sub-documents
- **`-c N`**: Include parent documents up to N levels up

### Sub-document Parent Context

When you retrieve a document with both depth (`-d`) and context (`-c`) enabled, the command also fetches parent documents of any sub-documents. This provides important context: the parent document often defines what *type* of thing the sub-document is.

**Example: A component used in multiple projects**

Consider a `button` component embedded in two different projects. When you retrieve `mobile-app`:

```
iwe retrieve -k mobile-app -d 1 -c 1
```

The output for the `button` sub-document shows its other parents:

``` markdown
# Button

[Button](button) <- [Web Dashboard](web-dashboard)

A reusable button component.
```

The `<-` notation lists other documents where `button` is embedded. Since we're already viewing `button` from within `mobile-app`, only `web-dashboard` is shown — revealing that this component is shared across projects.

**Result includes:**

1.  `mobile-app` — the main document
2.  `button` — sub-document (via `-d 1`)
3.  `web-dashboard` — another parent of `button` (via `-c 1`)

The `web-dashboard` document is fetched because it's a parent of `button`. This context helps understand what the component is and where else it's used.

**Note:** Only parents of *direct* sub-documents are included.

### Links (-l)

When enabled, includes documents that are referenced inline (not as inclusion links):

``` markdown
# Main Document

See [Related Topic](related-topic) for more details.
```

With `-l`, `related-topic` would be included in the output.

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
# Retrieve single document with defaults (depth=1, context=1)
iwe retrieve -k my-document

# Retrieve multiple documents at once
iwe retrieve -k doc1 -k doc2 -k doc3

# Retrieve only the main document, no expansion
iwe retrieve -k my-document -d 0 -c 0

# Retrieve with deep expansion (3 levels of sub-documents)
iwe retrieve -k my-document -d 3

# Include more parent context
iwe retrieve -k my-document -c 2

# Include inline referenced documents
iwe retrieve -k my-document -l

# Exclude documents which you don't need
iwe retrieve -k my-document -e already-loaded -e another-loaded

# Populate the `includes` edge array with child document edges
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
# Get document with immediate context
iwe retrieve -k authentication

# Get broader context if needed
iwe retrieve -k authentication -d 2 -c 2
```

### Understanding Document Relationships

``` bash
# See where a document is used (parent documents in output)
iwe retrieve -k my-topic -d 0 -c 0

# See the full context including inline references
iwe retrieve -k my-topic -l
```

### Exploring Knowledge Base Structure

``` bash
# Start from an entry point and expand
iwe retrieve -k index -d 2

# Get JSON for analysis
iwe retrieve -k project-overview -d 1 -f json
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

The following flags pre-date the query language and remain accepted for backward compatibility. Each invocation prints a one-line `warning: ... is deprecated` to stderr.

| Deprecated         | Use instead                                                                 |
| ------------------ | --------------------------------------------------------------------------- |
| `--in KEY[:N]`     | `--included-by KEY[:N]`                                                     |
| `--in-any K1 K2`   | `--filter '$or: [{ $includedBy: K1 }, { $includedBy: K2 }]'`                |
| `--not-in KEY`     | `--filter '$nor: [{ $includedBy: KEY }]'`                                   |
| `--refs-to KEY`    | `--references KEY` (legacy semantics: ORs `$includes` and `$references`)    |
| `--refs-from KEY`  | `--referenced-by KEY` (legacy semantics: ORs `$includedBy` and `$referencedBy`) |

## Technical Notes

- Documents use YAML frontmatter for metadata, content follows with original formatting
- Empty `includedBy` or `referencedBy` fields are omitted from markdown frontmatter (JSON/YAML output always emits them as `[]`)
- Original document headers are preserved (no level shifting)
- Two empty lines separate documents for easier parsing
- Duplicate documents are automatically filtered out
- Sub-document parent context only includes parents of direct (first-level) sub-documents, not nested ones
