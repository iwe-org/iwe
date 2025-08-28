# Indexing and Reference Systems

## Reference Index Structure

The `RefIndex` maintains bidirectional reference mappings:

``` rust
impl RefIndex {
    // Find all nodes that reference a specific document
    pub fn get_block_references_to(&self, key: &Key) -> Vec<NodeId>
    
    // Find all inline references (links) to a document  
    pub fn get_inline_references_to(&self, key: &Key) -> Vec<NodeId>
    
    // Recursively index a node and all its children
    pub fn index_node(&mut self, graph: &Graph, node_id: NodeId)
}
```

**Indexing process:**

1.  **Graph traversal**: Depth-first traversal of all nodes
2.  **Reference extraction**: Parse inline content for links
3.  **Bidirectional mapping**: Build forward and reverse reference maps
4.  **Incremental updates**: Re-index only changed portions

## Search Path Generation

Search paths provide hierarchical navigation:

``` rust
pub struct SearchPath {
    pub search_text: String,    // Concatenated plain text for matching
    pub node_rank: usize,      // Importance ranking
    pub key: Key,              // Source document
    pub root: bool,            // Is document root
    pub line: u32,             // Line number in source
    pub path: NodePath,        // Complete navigation path
}
```

**Ranking algorithm:**

- **Content depth**: Deeper content ranked lower
- **Reference count**: More referenced content ranked higher
- **Document position**: Earlier content ranked higher
- **Search relevance**: Fuzzy match score integration
