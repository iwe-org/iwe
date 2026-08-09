# IWE MCP server

Markdown knowledge base as agent memory. This package runs the IWE MCP server (`iwec`) over stdio.

## Usage

Add to your MCP client configuration:

```json
{
  "mcpServers": {
    "iwe": {
      "command": "npx",
      "args": ["-y", "@iwe-org/mcp"]
    }
  }
}
```

The server operates on the markdown files in its working directory. No configuration is required to start; an `.iwe/config.toml` in the directory customizes behavior (see https://iwe.md).

Prebuilt binaries cover macOS (arm64, x64), Linux (x64, arm64, gnu libc), and Windows (x64).
