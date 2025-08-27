# Document Processing Pipeline

## 1. Markdown Parsing (DocumentBlock Creation)

Raw markdown is first parsed into intermediate `DocumentBlock` representations:

``` rust
pub enum DocumentBlock {
    Plain(Plain),           // Plain text paragraphs
    Para(Para),             // Regular paragraphs  
    CodeBlock(CodeBlock),   // Fenced code blocks
    Header(Header),         // Headers with level and content
    BulletList(BulletList), // List containers
    Table(Table),           // Table structures
    // ... additional block types
}
```

## 2. Graph Construction (DocumentBlock → GraphNode)

The `SectionsBuilder` transforms `DocumentBlock` elements into graph nodes:

``` rust
// High-level transformation process
DocumentBlock::Header(header) → GraphNode::Section(section)
DocumentBlock::Para(para) → GraphNode::Leaf(leaf) 
DocumentBlock::BulletList(list) → GraphNode::BulletList(bulletlist)
```

**Key transformations:**

- **Headers become Sections**: With child relationships to content
- **Lists become containers**: With children for each list item
- **Paragraphs become Leaves**: Terminal nodes with text content
- **Code blocks become Raw nodes**: With language and content metadata

## 3. Reference Resolution and Indexing

After graph construction, the `RefIndex` system processes all references:

``` rust
pub struct RefIndex {
    block_references: HashMap<Key, HashSet<NodeId>>,   // [[note]] references
    inline_references: HashMap<Key, HashSet<NodeId>>,  // [link](note) references  
}
```
