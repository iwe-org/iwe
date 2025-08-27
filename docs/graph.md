# Graph Operations and Algorithms

## Tree Collection

Converting graph sections to tree structures for processing:

``` rust
// Collect a complete tree starting from a node
let tree = graph_node_pointer.collect_tree();

// Tree provides hierarchical access to content
for child in tree.children() {
    process_content(child);
}
```

## Squashing (Content Extraction)

Extract content at limited depth with proper hierarchy flattening:

``` rust
// Extract content up to depth 2
let squashed = graph.squash(&document_key, 2);

// Headers are flattened: h1 → h2, h2 → h3, etc.
// Content preserved with adjusted hierarchy
```

## Path Generation

Generate navigable paths through the document graph:

``` rust
pub struct NodePath {
    ids: Vec<NodeId>,        // Sequence of nodes forming path
    target: NodeId,          // Final destination node
}

// Paths enable:
// - Search result ranking
// - Navigation breadcrumbs  
// - Content organization
```
