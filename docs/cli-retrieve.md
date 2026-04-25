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
| `-k, --key <KEY>`     | Document key(s) to retrieve (can be specified multiple times)          | stdin    |
| `-e, --exclude <KEY>` | Exclude document key(s) from results (can be specified multiple times) | none     |
| `-d, --depth <N>`     | Follow children down N levels                                          | 1        |
| `-c, --context <N>`   | Include N levels of parent context                                     | 1        |
| `-l, --links`         | Include inline referenced documents                                    | false    |
| `-b, --backlinks`     | Include incoming references in output                                  | true     |
| `-f, --format <FMT>`  | Output format: `markdown`, `keys`, `json`                              | markdown |
| `--no-content`        | Exclude content, show child documents instead (metadata only)          | false    |
| `--dry-run`           | Show document count and total lines without content                    | false    |
| `--in <KEY[:DEPTH]>`  | Sub-documents of EVERY listed key (AND). Repeatable.                   | none     |
| `--in-any <KEY...>`   | Sub-documents of AT LEAST ONE listed key (OR). Repeatable.             | none     |
| `--not-in <KEY...>`   | Exclude sub-documents of any listed key (NOT). Repeatable.             | none     |
| `--max-depth <N>`     | Default depth for `--in` family. Unbounded if omitted.                 | none     |

### Structural set selector

When `--in` / `--in-any` / `--not-in` flags are provided, `retrieve` reads the documents that match the selector. With both `-k` and `--in`, the result is the **intersection** — explicit keys filtered through the selector. Empty intersection produces an empty result.

``` bash
# Read content of all docs that are sub-documents of A AND B
iwe retrieve --in projects/alpha --in people/dmytro --depth 0

# Read explicit keys but only those inside projects/alpha's subtree
iwe retrieve -k x -k y -k z --in projects/alpha
```


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
  parents:
  - key: index
    title: Index
  back-links:
  - key: related-doc
    title: Related Doc
---

# My Document

Original document content preserved exactly as written.
```

Each document in the result set includes:

- YAML frontmatter with `key`, `title`, `parents`, and `back-links`
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
{
  "documents": [
    {
      "key": "my-document",
      "title": "My Document",
      "content": "# My Document\n\nContent here...",
      "parent_documents": [
        {
          "key": "index",
          "title": "Index",
          "section_path": ["Topics", "My Topic"]
        }
      ],
      "backlinks": [
        {
          "key": "related-doc",
          "title": "Related Doc",
          "section_path": []
        }
      ]
    }
  ]
}
```

### Dry Run (`--dry-run`)

Shows statistics without outputting content:

``` bash
$ iwe retrieve -k my-document --dry-run
documents: 5
lines: 234
```

Useful for checking how much content would be retrieved before fetching it.

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

# Get metadata only, no content
iwe retrieve -k my-document --no-content

# Check size before retrieving
iwe retrieve -k my-document -d 2 -c 1 --dry-run

# Get JSON output for programmatic processing
iwe retrieve -k my-document -f json

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

# Check context size first
iwe retrieve -k authentication -d 2 --dry-run

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

## Technical Notes

- Documents use YAML frontmatter for metadata, content follows with original formatting
- Empty `parents` or `back-links` fields are omitted from frontmatter
- Original document headers are preserved (no level shifting)
- Two empty lines separate documents for easier parsing
- Duplicate documents are automatically filtered out
- Sub-document parent context only includes parents of direct (first-level) sub-documents, not nested ones
