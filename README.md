# IWE - Memory system for you and your AI agents

> Turn your thinking into queryable context

[![Crates.io](https://img.shields.io/crates/v/iwe.svg)](https://crates.io/crates/iwe)
[![Downloads](https://img.shields.io/crates/d/iwe.svg)](https://crates.io/crates/iwe)
[![License](https://img.shields.io/crates/l/iwe.svg)](https://github.com/iwe-org/iwe/blob/master/LICENSE-APACHE)
[![Build](https://github.com/iwe-org/iwe/workflows/Rust/badge.svg)](https://github.com/iwe-org/iwe/actions)
[![Documentation](https://img.shields.io/badge/docs-iwe.md-blue)](https://iwe.md)
[![Discussions](https://img.shields.io/github/discussions/iwe-org/iwe)](https://github.com/iwe-org/iwe/discussions)
[![Twitter](https://img.shields.io/badge/Twitter-@iwe__md-blue?logo=x)](https://x.com/iwe_md)
[![Reddit](https://img.shields.io/badge/Reddit-r%2Fiwe-orange?logo=reddit)](https://www.reddit.com/r/iwe/)
[![BundleDex](https://bundledex.net/badge/iwe.svg)](https://bundledex.net/bundles/iwe/)

![Knowledge Graph](docs/docs-detailed.svg)

IWE turns a directory of markdown files into a knowledge graph — a connected structure you browse from your editor and your AI queries from the command line. Same files, same links, two interfaces. No cloud, no database, no lock-in. Version everything with git.

Write in **Markdown**, structure with links, give AI agents the **tools** to navigate your knowledge. IWE itself has no built-in AI — it works alongside Claude, Codex, Gemini, and any tool that speaks the [Model Context Protocol](https://modelcontextprotocol.io).

## What You Get

- **Plain markdown, full ownership.** Your notes are `.md` files in a local directory. Read them, edit them, `git push` them. Nothing proprietary.
- **A graph, not a folder tree.** Link notes together and the same note can belong to multiple topics without copying the file. ([How linking works](https://iwe.md/docs/concepts/inclusion-links/))
- **IDE features for your editor.** Real LSP integration with [VS Code](https://iwe.md/docs/editors/vscode/), [Neovim](https://iwe.md/docs/editors/neovim/), [Zed](https://iwe.md/docs/editors/zed/), and [Helix](https://iwe.md/docs/editors/helix/) — search, refactor, rename, autocomplete.
- **Structured access for AI agents.** [CLI tools](https://iwe.md/docs/cli/) and an [MCP server](https://iwe.md/docs/agentic/mcp/) let agents search, retrieve, and refactor the same notes you edit by hand.
- **Fast.** Built in Rust, [processes 20,000 files in under a second](docs/benchmark.md).

## How It Works

IWE treats your notes as a connected structure. You organize them with two types of links:

- **Nesting** — a link on its own line means "this topic includes that subtopic." Your notes form a tree you can browse and refactor. IWE calls these [inclusion links](https://iwe.md/docs/concepts/inclusion-links/).
- **Cross-references** — regular inline links connect notes across topics, creating a web of relationships.
- **Multiple parents** — the same note can live under several places at once. A "Meditation" note can belong to both "Health" and "Productivity" without duplicating the file.
- **Context from parents** — when you retrieve a note, IWE can include context from the notes above it in the hierarchy.

This structure makes retrieval powerful — whether you're browsing in your editor or an agent is querying via CLI, ask for a topic and get its full context in a single call.

## Working with AI

IWE gives AI agents structured access to your notes through two interfaces: a CLI for scripting and shell-based workflows, and an MCP server for native connection with AI tools. Both expose the same operations — search, retrieve, create, refactor — so you can choose whichever fits your setup.

### Integration Server (MCP)

IWE includes a server (`iwec`) that lets AI tools like Claude Desktop, Cursor, and Windsurf work directly with your notes using the [Model Context Protocol](https://modelcontextprotocol.io). The server watches your files for changes, so edits you make in your editor are reflected immediately.

### Command-Line Tools

The CLI lets you (and AI agents) work with your notes from the terminal or in scripts.

**Example: preparing context for an AI conversation**
```bash
iwe find auth

iwe retrieve --key authentication --depth 2

iwe tree --key oauth
```

**Available commands:**

| Command | What it does |
|---|---|
| `find` | Search notes with fuzzy matching |
| `retrieve` | Get a note with its children and parent context |
| `tree` | Show the hierarchy from any starting point |
| `squash` | Flatten a subtree into one document |
| `new` | Create a note (accepts content from stdin) |
| `extract` | Pull a section into its own note |
| `inline` | Merge a linked note back into its parent |
| `rename` | Rename a note; all links update automatically |
| `delete` | Remove a note and clean up references |

More information: [Working with AI](https://iwe.md/docs/agentic/) · [CLI Reference](https://iwe.md/docs/cli/) · [MCP Server](https://iwe.md/docs/agentic/mcp/)

## Editor Integration

IWE gives your editor IDE-like features for markdown notes. It works with [VS Code](https://iwe.md/docs/editors/vscode/), [Neovim](https://iwe.md/docs/editors/neovim/), [Zed](https://iwe.md/docs/editors/zed/), [Helix](https://iwe.md/docs/editors/helix/), and any editor that supports the Language Server Protocol (LSP).

- **Search** — find notes by title or content
- **Navigate** — go to definition, find references (backlinks)
- **Preview** — hover over links to see content
- **Auto-complete** — link suggestions as you type
- **Inlay hints** — show parent references and link counts
- **Extract** — pull sections into new notes
- **Inline** — embed note content back into parent
- **Rename** — rename files with automatic link updates
- **Format** — normalize documents, update link titles
- **Transform** — pipe text through external commands
- **Templates** — create notes from templates (daily notes, etc.)
- **Outline conversion** — switch between headers and lists

More information: [Editor Features](https://iwe.md/docs/getting-started/usage/)

## Quick Start

1. **Install** the CLI and LSP server:

   Using Homebrew (macOS/Linux):
   ```bash
   brew tap iwe-org/iwe
   brew install iwe
   ```

   Or using Cargo:
   ```bash
   cargo install iwe iwes iwec
   ```

2. **Initialize** your workspace:
   ```bash
   cd ~/notes
   iwe init
   ```

3. **Pick your path:**

   **Set up your editor** — [VS Code](https://iwe.md/docs/editors/vscode/) · [Neovim](https://iwe.md/docs/editors/neovim/) · [Helix](https://iwe.md/docs/editors/helix/) · [Zed](https://iwe.md/docs/editors/zed/)

   **Connect your AI agent** — point it at the MCP server:
   ```json
   {
     "mcpServers": {
       "iwe": {
         "command": "iwec",
         "args": ["--project", "~/notes"]
       }
     }
   }
   ```

## Documentation

- [Getting Started](https://iwe.md/docs/getting-started/installation/) — Installation and setup
- [Usage Guide](https://iwe.md/docs/getting-started/usage/) — Editor features and workflows
- [CLI Reference](https://iwe.md/docs/cli/) — Command-line tools
- [Working with AI](https://iwe.md/docs/agentic/) — AI agent integration
- [MCP Server](https://iwe.md/docs/agentic/mcp/) — Native AI tool integration via Model Context Protocol
- [Configuration](https://iwe.md/docs/configuration/) — Settings and customization
- [Examples](https://iwe.md/docs/examples/) — Example projects and case studies

## Get Involved

IWE is open source and community-driven. Join the [discussions](https://github.com/iwe-org/iwe/discussions), report [issues](https://github.com/iwe-org/iwe/issues), or contribute to the [documentation](docs/).

**Community:** [Twitter/X](https://x.com/iwe_md) · [Reddit](https://www.reddit.com/r/iwe/) · [Discussions](https://github.com/iwe-org/iwe/discussions)

**Editor plugins:** [VS Code](https://github.com/iwe-org/vscode-iwe) · [Neovim](https://github.com/iwe-org/iwe.nvim) · [Zed](https://github.com/iwe-org/zed-iwe)

**Agentic skills:** [iwe-org/skills](https://github.com/iwe-org/skills) — agentic AI skills for knowledge graph management. Contributors welcome.

[![OKF BundleDex](https://bundledex.net/static-badge.svg)](https://bundledex.net)

## License

[Apache License 2.0](LICENSE-APACHE)
