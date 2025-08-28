# Data Flow Architecture

## CLI Operations

CLI commands operate directly on the graph:

``` rust
// Normalization: Rewrite all documents with consistent formatting
fn normalize() { graph.export() → filesystem }

// Export: Generate visualization formats (DOT, etc.)
fn export() { graph → GraphData → DOTExporter }

// Contents: Generate table of contents
fn contents() { graph.paths() → filtered paths → markdown }

// Squash: Extract partial content at specified depth
fn squash() { graph.squash(key, depth) → markdown }
```

## LSP Server Integration

The LSP server maintains a live `Database` wrapper around the graph:

``` rust
pub struct Database {
    graph: Graph,                    // Core graph structure
    content: HashMap<Key, Content>,  // Original markdown content
    paths: Vec<SearchPath>,          // Pre-computed search paths
}
```

**Real-time operations:**

- **Document updates**: Incremental graph rebuilding
- **Reference resolution**: Live link validation
- **Search**: Fuzzy matching against pre-computed paths
- **Completion**: Context-aware suggestions based on graph structure

## Memory and Performance Characteristics

**Graph construction:**

- **Parallel processing**: Rayon integration for multi-document parsing
- **Incremental updates**: Only affected nodes rebuilt on changes
- **Memory efficiency**: Arena pattern minimizes allocation overhead

**Search performance:**

- **Pre-computed paths**: Search index built once, queried repeatedly
- **Fuzzy matching**: SkimMatcher for intelligent search ranking
- **Parallel search**: Multi-threaded query processing
