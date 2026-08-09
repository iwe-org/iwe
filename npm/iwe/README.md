# IWE

Markdown-based knowledge management for developers. This package installs the prebuilt IWE binaries via npm:

- `iwe` — the CLI: search, query, refactor, and validate a markdown knowledge base
- `iwes` — the LSP server for VSCode, Neovim, Zed, Helix, and other editors
- `iwec` — the MCP server (also available as [@iwe-org/mcp](https://www.npmjs.com/package/@iwe-org/mcp))

## Usage

```
npx -y @iwe-org/iwe --help
```

Or install globally:

```
npm install -g @iwe-org/iwe
iwe init
```

Prebuilt binaries cover macOS (arm64, x64), Linux (x64, arm64, gnu libc), and Windows (x64). The matching platform package is selected automatically at install time; no install scripts run.

Documentation: https://iwe.md
