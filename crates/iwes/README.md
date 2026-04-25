# IWE LSP Server

Language Server Protocol (LSP) server for IWE - memory system for you and your AI agents.

## Installation

```bash
cargo install --path .
```

## Supported editors

- **VS Code** - via [IWE extension](https://marketplace.visualstudio.com/items?itemName=IWE.iwe)
- **Zed** - via [zed-iwe plugin](https://github.com/iwe-org/zed-iwe)
- **Neovim** - via LSP configuration
- **Helix** - via LSP configuration

## LSP capabilities

- **Go to definition** - navigate to referenced documents
- **Find references** - find all documents referencing a given document
- **Hover** - preview referenced document content
- **Completion** - document link completion (triggered by `+`)
- **Rename** - rename document keys with cross-graph reference updates
- **Document symbols** - outline view of document structure
- **Workspace symbols** - search across all documents
- **Code actions** - configurable actions (extract, inline, attach, transform, sort)
- **Document formatting** - normalize markdown formatting
- **Inlay hints** - inline reference metadata
- **Folding ranges** - collapse document sections
- **Execute command** - run configured text transformation commands

## Configuration

The server reads `.iwe/config.toml` from the workspace root.

Text transformations use the `[commands]` section to define shell commands that process text via stdin/stdout:

```toml
[commands]
summarize = "llm -m gpt-4o 'Summarize this text'"
```

## License

Apache-2.0

For more information, visit [iwe.md](https://iwe.md).
