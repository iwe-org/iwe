# IWE Contents

Lists root documents (notes without parent references) in your knowledge graph.

## Usage

``` bash
iwe contents
```

## Purpose

Identifies entry points in your knowledge graph - documents that aren't referenced by others, potentially serving as main topics or starting points. Root documents are the top-level organizational nodes from which you navigate into more specific topics.

## Output Format

The command outputs a markdown document with block references to all root documents:

``` markdown
# Contents

[Project Overview](project-overview)

[Daily Journal](daily-journal)

[Research Topics](research-topics)
```

Each line is a markdown link in the format `[Document Title](document-key)`.

## Example

Given a knowledge base with these files:

- `project-overview.md` - Root document (not referenced elsewhere)
- `daily-journal.md` - Root document (not referenced elsewhere)
- `meeting-notes.md` - Referenced by `project-overview.md`
- `task-list.md` - Referenced by `project-overview.md`

Running `iwe contents` outputs:

``` markdown
# Contents

[Daily Journal](daily-journal)

[Project Overview](project-overview)
```

Only `daily-journal` and `project-overview` appear because they are not referenced by any other document.

## Contents vs Find --roots

Both commands identify root documents, but with different purposes:

| Command | Output Format | Use Case |
|---------|---------------|----------|
| `iwe contents` | Markdown with block references | Human-readable navigation, documentation |
| `iwe find --roots` | Line-based key list (optionally JSON) | Pipeline processing, scripting |

Use `iwe contents` when you need a navigable table of contents. Use `iwe find --roots` when processing results programmatically.

## AI Agent Tips

- Use `contents` to discover the primary organizational structure of a knowledge base
- Combine with `retrieve` to expand root documents into full context
- Root documents often represent the main topics or projects being tracked
- A large number of root documents may indicate an under-connected knowledge base
