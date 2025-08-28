# Key System and Cross-References

## Document Identification

Each document is identified by a `Key` - a path-based identifier:

``` rust
pub struct Key {
    pub relative_path: Arc<String>,  // e.g., "folder/document"
}
```

**Key features:**

- **Path-based**: Hierarchical organization support
- **Reference counting**: Arc enables efficient cloning
- **Extension handling**: Automatic .md extension management
- **Relative linking**: Support for ../parent/document syntax

## Reference Types

IWE supports three reference types:

1.  **Regular markdown links**: `[text](document.md)`
2.  **Wiki-style links**: `[[document]]`
3.  **Piped wiki links**: `[[document|display text]]`

Each reference type is preserved and can be normalized or converted as needed.
