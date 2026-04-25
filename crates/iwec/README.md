# IWE MCP Server

Model Context Protocol (MCP) server for IWE - memory system for you and your AI agents.

## Installation

```bash
cargo install --path .
```

## MCP tools

| Tool | Description |
|------|-------------|
| `iwe_find` | Search documents by fuzzy query, structural filters, or reference relationships |
| `iwe_retrieve` | Retrieve documents with configurable depth expansion, context, and backlinks |
| `iwe_tree` | View hierarchical tree structure of the knowledge graph |
| `iwe_stats` | Get graph statistics including document counts, references, and connectivity |
| `iwe_squash` | Expand all block references into a single flat markdown document |
| `iwe_create` | Create a new document from a title and optional content |
| `iwe_update` | Update the full markdown content of an existing document |
| `iwe_delete` | Delete a document with automatic reference cleanup |
| `iwe_rename` | Rename a document key with cross-graph reference updates |
| `iwe_extract` | Extract a section into a new document, replacing it with a block reference |
| `iwe_inline` | Replace a block reference with the actual content of the referenced document |
| `iwe_normalize` | Normalize all document formatting across the knowledge graph |
| `iwe_attach` | Attach a document as a block reference in a target determined by a configured action |

## MCP resources

- `iwe://config` - current workspace configuration

## MCP prompts

- `review` - review a document
- `refactor` - analyze a document for restructuring

## Configuration

The server reads `.iwe/config.toml` from the workspace root. Set `IWE_DEBUG=1` for debug logging to stderr.

### Claude Desktop

Add to your Claude Desktop configuration:

```json
{
  "mcpServers": {
    "iwe": {
      "command": "iwec",
      "args": [],
      "cwd": "/path/to/your/workspace"
    }
  }
}
```

## License

Apache-2.0

For more information, visit [iwe.md](https://iwe.md).
