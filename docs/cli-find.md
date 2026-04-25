# IWE Find

Search and discover documents in your knowledge base with fuzzy matching and relationship filtering.

## Usage

``` bash
iwe find [QUERY] [OPTIONS]
```

## Options

| Flag                 | Description                                                      | Default          |
| -------------------- | ---------------------------------------------------------------- | ---------------- |
| `[QUERY]`            | Fuzzy search on document title and key                           | none (lists all) |
| `--roots`            | Only show root documents (no parents)                            | false            |
| `--refs-to <KEY>`    | Documents that reference this key (direct, includes inline)      | none             |
| `--refs-from <KEY>`  | Documents referenced by this key (direct, includes inline)       | none             |
| `--in <KEY[:DEPTH]>` | Sub-documents of EVERY listed key (AND). Repeatable.             | none             |
| `--in-any <KEY...>`  | Sub-documents of AT LEAST ONE listed key (OR). Repeatable.       | none             |
| `--not-in <KEY...>`  | Exclude sub-documents of any listed key (NOT). Repeatable.       | none             |
| `--max-depth <N>`    | Default depth for `--in` / `--in-any` / `--not-in`. Unbounded if omitted. | none      |
| `-l, --limit <N>`    | Maximum number of results                                        | 50               |
| `-f, --format <FMT>` | Output format: `markdown`, `keys`, `json`                        | markdown         |

## Structural set selector (`--in` family)

The `--in`, `--in-any`, `--not-in`, and `--max-depth` flags together form a **set selector** over block-reference relationships. They answer questions like "documents that are sub-documents of A and B within depth N":

``` bash
# Sub-documents of A AND B (intersection, unbounded depth)
iwe find --in projects/alpha --in people/dmytro

# Same, bounded to 2 hops from each parent
iwe find --in projects/alpha --in people/dmytro --max-depth 2

# Per-parent depth: A within 3 hops, Dmytro within 1
iwe find --in projects/alpha:3 --in people/dmytro:1

# Sub-documents of A OR B (union)
iwe find --in-any projects/alpha --in-any projects/beta

# Sub-documents of A but NOT B
iwe find --in projects/alpha --not-in archive
```

The same selector flags are also accepted by `iwe retrieve`, `iwe tree`, and `iwe export`. See those commands for set-aware reads.


## How It Works

The `find` command searches and filters documents in your knowledge base:

1.  **Fuzzy matching** - Uses the same fuzzy search algorithm as the LSP server (SkimMatcherV2)
2.  **Ranking** - Without a query, documents are sorted by popularity (incoming references count)
3.  **Filtering** - Apply filters for root documents or reference relationships
4.  **Parent context** - Results include parent document information

### Fuzzy Search

The fuzzy matcher searches across both the document key and title:

``` bash
# Finds "authentication.md" with title "User Authentication"
iwe find auth

# Finds documents with "api" in key or title
iwe find api
```

### Root Documents

Root documents are entry points - documents with no parents:

``` bash
# List only root documents (no parents)
iwe find --roots
```

### Reference Filters

Find documents based on their relationships:

``` bash
# Documents that reference "authentication"
iwe find --refs-to authentication

# Documents referenced by "index"
iwe find --refs-from index
```

## Output Formats

### Markdown Format (default)

``` markdown
## Documents

Found 3 results:

- [User Authentication](authentication) (root)
- [Login Flow](login-flow) <- [User Authentication](authentication)
- [Session Management](session-management) <- [User Authentication](authentication)
```

Each result shows:

- Document title and key as a markdown link
- `(root)` indicator if no parents
- Parent documents shown with `<-` arrow

### Keys Format (`-f keys`)

```
authentication
login-flow
session-management
```

One key per line, suitable for piping to other commands.

### JSON Format (`-f json`)

``` json
{
  "query": "auth",
  "total": 3,
  "results": [
    {
      "key": "authentication",
      "title": "User Authentication",
      "is_root": true,
      "incoming_refs": 5,
      "outgoing_refs": 2,
      "parent_documents": []
    },
    {
      "key": "login-flow",
      "title": "Login Flow",
      "is_root": false,
      "incoming_refs": 2,
      "outgoing_refs": 0,
      "parent_documents": [
        {
          "key": "authentication",
          "title": "User Authentication",
          "section_path": ["Implementation"]
        }
      ]
    }
  ]
}
```

Fields:

- `query` - The search query (null if no query provided)
- `total` - Total matching documents (before limit applied)
- `results` - Array of matching documents
  - `is_root` - True if no parents
  - `incoming_refs` - Count of parents + inline references to this document
  - `outgoing_refs` - Count of children in this document
  - `parent_documents` - Documents that include this one

## Examples

``` bash
# List all documents (sorted by popularity)
iwe find

# Fuzzy search for documents
iwe find authentication
iwe find "api endpoint"

# List only root documents (entry points)
iwe find --roots

# Find what references a specific document
iwe find --refs-to my-document

# Find what a document references
iwe find --refs-from index

# Combine search with filters
iwe find api --roots

# Limit results
iwe find --limit 10

# Get JSON for programmatic use
iwe find -f json

# Get keys for piping
iwe find --roots -f keys

# Pipe keys to retrieve command
iwe find --roots -f keys | head -5 | xargs -I {} iwe retrieve -k {}
```

## Use Cases

### Discover Entry Points

Find root documents that serve as entry points to different topics:

``` bash
iwe find --roots
```

### Explore Document Relationships

See what documents reference or are referenced by a specific document:

``` bash
# What uses this document?
iwe find --refs-to authentication

# What does this document use?
iwe find --refs-from index
```

### Quick Document Lookup

Fuzzy search when you remember part of a document name:

``` bash
iwe find deploy    # Finds "deployment", "deploy-script", etc.
iwe find config    # Finds "configuration", "config-options", etc.
```

### Pipeline Integration

Use keys format for scripting:

``` bash
# Retrieve content for top 5 root documents
iwe find --roots -l 5 -f keys | while read key; do
  iwe retrieve -k "$key" -d 0
done

# Export all root documents to separate files
iwe find --roots -f keys | xargs -I {} sh -c 'iwe retrieve -k {} > {}.out.md'
```

### Analyze Knowledge Base Structure

Use JSON output for analysis:

``` bash
# Find most referenced documents
iwe find -f json | jq '.results | sort_by(-.incoming_refs) | .[0:5]'

# Find orphan documents (roots with no outgoing refs)
iwe find --roots -f json | jq '.results | map(select(.outgoing_refs == 0))'
```

## Technical Notes

- Without a query, documents are sorted by incoming reference count (popularity)
- With a query, results are sorted by fuzzy match score
- The limit is applied after sorting, so you get the top N results
- Parent documents show section path breadcrumbs when applicable
- Both [Inclusion Links](inclusion-links.md) and inline references count toward `incoming_refs`
