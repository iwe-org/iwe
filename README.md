# IWE - Memory system for you and your AI Agents

> Your second brain that AI agents can navigate. A structured knowledge graph for humans and machines.

[![Crates.io](https://img.shields.io/crates/v/iwe.svg)](https://crates.io/crates/iwe)
[![Downloads](https://img.shields.io/crates/d/iwe.svg)](https://crates.io/crates/iwe)
[![License](https://img.shields.io/crates/l/iwe.svg)](https://github.com/iwe-org/iwe/blob/master/LICENSE-APACHE)
[![Build](https://github.com/iwe-org/iwe/workflows/Rust/badge.svg)](https://github.com/iwe-org/iwe/actions)
[![Documentation](https://img.shields.io/badge/docs-iwe.md-blue)](https://iwe.md)
[![Discussions](https://img.shields.io/github/discussions/iwe-org/iwe)](https://github.com/iwe-org/iwe/discussions)
[![Twitter](https://img.shields.io/badge/Twitter-@iwe__md-blue?logo=x)](https://x.com/iwe_md)
[![Reddit](https://img.shields.io/badge/Reddit-r%2Fiwe-orange?logo=reddit)](https://www.reddit.com/r/iwe/)

![Knowledge Graph](docs/docs-detailed.svg)

> A messy knowledge base provides messy context to an AI, and messy context yields poor results.

The knowledge you need keeps growing. Codebases expand. Documentation multiplies. Decisions pile up. It's hard to keep it all in your head—or fit it in a context window.

We've all been there: you capture an insight, file it away, and two weeks later it's gone—buried in folders or scattered across apps. When you ask AI for help, you're feeding it fragments hoping something sticks. IWE gives both you and your AI the same map to navigate—one source of truth, shared understanding.

IWE is a knowledge graph that organizes your notes hierarchically and makes them accessible to AI agents. Write in **Markdown**, structure with links, give AI agents the **tools** to navigate your knowledge.

IWE itself has no built-in AI. It's designed to work alongside AI tools like Claude, Codex, and Gemini, giving them structured access to your notes so they can read, search, and update your knowledge base.

## What You Get

- **Your notes, your machine** — everything is plain Markdown files in a local directory. No cloud, no database, no lock-in.
- **Structure without folders** — link notes together and IWE understands parent-child relationships. The same note can belong to multiple topics without copying the file. ([How linking works](https://iwe.md/docs/concepts/inclusion-links/))
- **IDE features for notes** — search, autocomplete, go-to-definition, rename, and refactoring in [VS Code](https://iwe.md/docs/editors/vscode/), [Neovim](https://iwe.md/docs/editors/neovim/), [Zed](https://iwe.md/docs/editors/zed/), and [Helix](https://iwe.md/docs/editors/helix/)
- **AI agents can use your notes** — [CLI tools](https://iwe.md/docs/cli/) and an [integration server](https://iwe.md/docs/agentic/mcp/) let AI agents search, read, and update your notes with full context
- **Fast** — built in Rust, [processes 20,000 files in under a second](docs/benchmark.md)

## How It Works

IWE treats your notes as a connected structure. You organize them with two types of links:

- **Nesting** — a link on its own line means "this topic includes that subtopic." Your notes form a tree you can browse and refactor. IWE calls these [inclusion links](https://iwe.md/docs/concepts/inclusion-links/).
- **Cross-references** — regular inline links connect notes across topics, creating a web of relationships
- **Multiple parents** — the same note can live under several places at once. A "Meditation" note can belong to both "Health" and "Productivity" without duplicating the file.
- **Context from parents** — when you retrieve a note, IWE can include context from the notes above it in the hierarchy

This structure makes retrieval powerful: ask for a topic and get its full context—children, parents, and related notes—in a single query.

## Editor Integration

IWE gives your editor IDE-like features for markdown notes. It works with [VS Code](https://iwe.md/docs/editors/vscode/), [Neovim](https://iwe.md/docs/editors/neovim/), [Zed](https://iwe.md/docs/editors/zed/), [Helix](https://iwe.md/docs/editors/helix/), and any editor that supports the Language Server Protocol (LSP).

IWE understands document structure—headers, lists, and links—and provides refactorings like extracting sections into new notes and inlining them back. It supports standard Markdown, wiki-style links, tables, and other extensions.

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

## Working with AI

IWE gives AI agents structured access to your notes through two interfaces: a CLI for scripting and shell-based workflows, and an integration server for native connection with AI tools. Both expose the same operations—search, retrieve, create, refactor—so you can choose whichever fits your setup.

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

### Integration Server for AI Tools

IWE includes a server (`iwec`) that lets AI tools like Claude Desktop, Cursor, and Windsurf work directly with your notes. It uses the [Model Context Protocol (MCP)](https://modelcontextprotocol.io), an open standard for connecting AI tools to data sources.

The server provides the same operations as the CLI—search, retrieve, create, refactor—and watches your files for changes, so edits you make in your editor are reflected immediately.

More information: [Integration Server Documentation](https://iwe.md/docs/agentic/mcp/)

---

Other AI memory tools store your notes in formats you can't read or edit. IWE keeps everything in plain Markdown files on your disk. When an AI agent asks for information, it gets the exact notes that are connected to the topic—not "maybe relevant" guesses based on text similarity.

You stay in control. Your notes are plain text files you can read, edit, and version with git. AI agents become collaborators that navigate your knowledge alongside you, not black boxes that absorb it.

More information:
- [Working with AI](https://iwe.md/docs/agentic/)
- [CLI Reference](https://iwe.md/docs/cli/)
- [Integration Server](https://iwe.md/docs/agentic/mcp/)

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

3. **Configure** your editor — [VS Code](https://iwe.md/docs/editors/vscode/) · [Neovim](https://iwe.md/docs/editors/neovim/) · [Helix](https://iwe.md/docs/editors/helix/) · [Zed](https://iwe.md/docs/editors/zed/)

4. **Teach** your AI agent — ask it to learn the `iwe` command using its built-in help

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

## License

[Apache License 2.0](LICENSE-APACHE)
