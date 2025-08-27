# Node Types and Structure

## GraphNode Enumeration

IWE defines 9 distinct node types, each optimized for specific markdown elements:

``` rust
pub enum GraphNode {
    Empty,                    // Deleted/placeholder nodes
    Document(Document),       // Root document container
    Section(Section),         // Headers (h1-h6)
    Quote(Quote),            // Blockquotes  
    BulletList(BulletList),  // Unordered lists
    OrderedList(OrderedList), // Numbered lists
    Leaf(Leaf),              // Paragraphs and simple blocks
    Raw(RawLeaf),            // Code blocks and raw content
    HorizontalRule(HorizontalRule), // Horizontal rules
    Reference(Reference),     // Block references to other documents
    Table(Table),            // Markdown tables
}
```

## Node Relationships and Navigation

Each node (except Document and Empty) contains:

- **id**: Unique identifier within the graph
- **prev**: Reference to previous sibling or parent
- **next**: Optional reference to next sibling
- **child**: Optional reference to first child (container nodes only)

**Navigation patterns:**

- **Siblings**: Follow `next` pointers horizontally
- **Children**: Follow `child` pointer then `next` for all children
- **Parent**: Use `prev` pointer and traverse up

## Content Storage Separation

Text content is stored separately from structure in `Line` objects:

``` rust
pub struct Line {
    id: LineId,
    inlines: GraphInlines,  // Vector of inline elements (text, links, formatting)
}
```

This separation enables:

- **Structure reuse**: Multiple nodes can reference same content
- **Efficient updates**: Content changes don't affect structure
- **Memory optimization**: Structure and content cached independently
