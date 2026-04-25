# liwe

Core library for IWE - provides the graph-based document model, markdown parser, and all document operations.

## Data model

IWE represents markdown documents as a directed graph using an arena-based memory model:

- Every structural element (header, paragraph, list, list item, code block, table, block reference) becomes a node
- Nodes have two relationships: `next-element` (sibling) and `child-element` (first child)
- Documents are identified by `Key` (relative path without `.md` extension)

## Modules

- `graph` - graph operations, arena storage, node iteration, path computation
- `model` - core types: `Key`, `NodeId`, `Content`, `Position`, `Configuration`
- `markdown` - markdown parsing and rendering using pulldown-cmark
- `find` - document search and discovery with fuzzy matching
- `retrieve` - document content retrieval with depth expansion and backlinks
- `operations` - graph transformations: delete, extract, inline, rename
- `stats` - knowledge base statistics generation
- `fs` - filesystem abstraction
- `state` - document state management
- `locale` - locale support for date formatting

## Usage

```rust
use liwe::fs::new_for_path;
use liwe::graph::{Graph, GraphContext};
use liwe::model::config::load_config;

let config = load_config();
let fs = new_for_path("/path/to/workspace", &config);
let graph = Graph::parse(fs);
```

## License

Apache-2.0

For more information, visit [iwe.md](https://iwe.md).
