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

[![Knowledge Graph](docs/docs-detailed.svg)](https://iwe.md)

IWE turns a directory of markdown files into a knowledge graph — a connected structure you browse from your editor and your AI queries from the command line. Same files, same links, two interfaces. No cloud, no database, no lock-in. Version everything with git.

IWE is for people who want database-style queries on their notes — "all drafts under this subtree", "every accepted decision in Q1" — without moving them into an actual database. Write in **Markdown**, structure with links, give AI agents the **tools** to navigate your knowledge. IWE itself has no built-in AI — it works alongside Claude, Codex, Gemini, and any tool that speaks the [Model Context Protocol](https://modelcontextprotocol.io).

## What You Get

- **Plain markdown, full ownership.** Your notes are `.md` files in a local directory. Read them, edit them, `git push` them. Nothing proprietary.
- **A graph, not a folder tree.** Link notes together and the same note can belong to multiple topics without copying the file. ([How linking works](https://iwe.md/docs/concepts/inclusion-links/))
- **IDE features for your editor.** Real LSP integration with [VS Code](https://iwe.md/docs/editors/vscode/), [Neovim](https://iwe.md/docs/editors/neovim/), [Zed](https://iwe.md/docs/editors/zed/), and [Helix](https://iwe.md/docs/editors/helix/) — search, refactor, rename, autocomplete.
- **Structured access for AI agents.** [CLI tools](https://iwe.md/docs/cli/) and an [MCP server](https://iwe.md/docs/agentic/mcp/) give agents parent context and structural navigation over the same notes you edit by hand — retrieval by structure, not similarity guessing.
- **Speaks OKF.** An [Open Knowledge Format](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md) bundle is markdown with YAML frontmatter — the format IWE already manages. `iwe init --okf` scaffolds a conformant bundle, `iwe schema validate` checks conformance mechanically, and `iwe find --filter '{type: …}'` queries OKF frontmatter directly.
- **Fast.** Built in Rust, [processes 20,000 files in under a second](docs/benchmark.md).

## Instructions Aren't Memory

Agents ship with an instructions file — CLAUDE.md, AGENTS.md — and it's tempting to let memory accumulate there too. It doesn't hold. Instructions are rules; memory is what builds up: decisions, corrections, the state of everything in flight. Kept in one flat file, the agent appends, contradictions pile up side by side each looking authoritative, and every session re-reads the whole thing.

IWE stores memory as many small linked documents, so the agent asks instead of re-reading. Most recall is a structured question — what's still open, what did we decide about X, which notes mention this person — answered by a query and one retrieve call that expands the linked context. The rest of the graph stays out of the context window. And because it's markdown in git, the memory is inspectable: open what the agent believes, diff what changed, git-blame when.

## How It Works

IWE treats your notes as a connected structure. You organize them with two types of links:

- **Nesting** — a link on its own line means "this topic includes that subtopic." Your notes form a tree you can browse and refactor. IWE calls these [inclusion links](https://iwe.md/docs/concepts/inclusion-links/).
- **Cross-references** — regular inline links connect notes across topics, creating a web of relationships.
- **Multiple parents** — the same note can live under several places at once. A "Meditation" note can belong to both "Health" and "Productivity" without duplicating the file.
- **Context from parents** — when you retrieve a note, IWE can include context from the notes above it in the hierarchy.

This structure makes retrieval powerful — whether you're browsing in your editor or an agent is querying via CLI, ask for a topic and get its full context in a single call.

## Working with AI

IWE gives AI agents structured access to your notes through two interfaces: a CLI for scripting and shell-based workflows, and an MCP server for native connection with AI tools. Both expose the same operations — search, retrieve, create, refactor — so you can choose whichever fits your setup.

IWE pairs **search with structure**: built-in fuzzy and full-text search finds the entry point, and the graph turns a hit into usable context — parent context, children, cross-references, link-safe refactoring. It also composes cleanly with any external tooling you already use (ripgrep, full-text, vector): whatever finds the note, IWE supplies the context around it.

### What the Engine Checks

Agent writes are checked, not trusted:

- **Declared scope.** A mutation carries `expect` guards stating how many documents and blocks it may touch. The whole update validates before anything is written; a mismatch aborts with the offending blocks named. Over MCP the guards are mandatory — an edit that won't declare its blast radius is refused.
- **Schemas.** Frontmatter and document structure are validated against per-type [document schemas](https://iwe.md/docs/concepts/document-schema/) — required fields, enums, ISO dates, required sections. A schema-violating MCP write is rejected with the violation named; from the CLI, `iwe schema validate` runs the same checks on demand.
- **Graph hygiene.** Mutations surface warnings for what they disturbed — dangling links, orphan pages — and `iwe stats similarity` flags near-duplicates.

### Integration Server (MCP)

IWE includes a server (`iwec`) that lets AI tools like Claude Desktop, Cursor, and Windsurf work directly with your notes using the [Model Context Protocol](https://modelcontextprotocol.io). The server watches your files for changes, so edits you make in your editor are reflected immediately.

### Command-Line Tools

The CLI lets you (and AI agents) work with your notes from the terminal or in scripts.

**Example: preparing context for an AI conversation**
```bash
iwe find --fuzzy auth

iwe retrieve --key authentication --expand-includes 2

iwe tree --key oauth
```

**Core commands:**

| Command | What it does |
|---|---|
| `find` | Search with fuzzy and full-text ranking, plus filters over frontmatter and graph edges |
| `retrieve` | Get a document with its linked context in one call |
| `tree` | Show the hierarchy from any starting point |
| `update` | Guarded edits: frontmatter changes and targeted block operations |
| `schema` | Infer the store's schemas, or validate documents against them |

The full set — `new`, `extract`, `inline`, `rename`, `delete`, `squash`, `stats`, `normalize`, `export` and more — is in the [CLI Reference](https://iwe.md/docs/cli/).

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

   Or from [conda-forge](https://anaconda.org/conda-forge/iwe) (community-maintained — thanks, [salim-b](https://github.com/salim-b)):
   ```bash
   conda install -c conda-forge iwe
   ```

2. **Initialize** your workspace:
   ```bash
   cd ~/notes
   iwe init
   ```

3. **Pick your path:**

   **Set up your editor** — [VS Code](https://iwe.md/docs/editors/vscode/) · [Neovim](https://iwe.md/docs/editors/neovim/) · [Helix](https://iwe.md/docs/editors/helix/) · [Zed](https://iwe.md/docs/editors/zed/)

   **Connect your AI agent** — point it at the MCP server. `iwec` serves the directory it runs in, so set the working directory to your notes:
   ```json
   {
     "mcpServers": {
       "iwe": {
         "command": "iwec",
         "cwd": "~/notes"
       }
     }
   }
   ```

   Or hand the setup to the agent — paste this into Claude Code or any agent with shell access:

   ```text
   Set up IWE for my notes: install it (brew tap iwe-org/iwe && brew install iwe,
   or cargo install iwe iwes iwec), run `iwe init` in my notes directory, then
   add the `iwec` MCP server with its working directory set to that folder.
   Docs: https://iwe.md/docs/agentic/
   ```

## Documentation

- [Getting Started](https://iwe.md/docs/getting-started/installation/) — Installation and setup
- [Usage Guide](https://iwe.md/docs/getting-started/usage/) — Editor features and workflows
- [CLI Reference](https://iwe.md/docs/cli/) — Command-line tools
- [Working with AI](https://iwe.md/docs/agentic/) — AI agent integration
- [MCP Server](https://iwe.md/docs/agentic/mcp/) — Native AI tool integration via Model Context Protocol
- [OKF](https://iwe.md/docs/agentic/okf/) — Open Knowledge Format: scaffold, validate, and query conformant bundles
- [Configuration](https://iwe.md/docs/configuration/) — Settings and customization
- [Examples](https://iwe.md/docs/examples/) — Example projects and case studies

## Get Involved

IWE is open source and community-driven. Join the [discussions](https://github.com/iwe-org/iwe/discussions), report [issues](https://github.com/iwe-org/iwe/issues), or contribute to the [documentation](docs/).

**Community:** [Twitter/X](https://x.com/iwe_md) · [Reddit](https://www.reddit.com/r/iwe/) · [Discussions](https://github.com/iwe-org/iwe/discussions)

**Editor plugins:** [VS Code](https://github.com/iwe-org/vscode-iwe) · [Neovim](https://github.com/iwe-org/iwe.nvim) · [Zed](https://github.com/iwe-org/zed-iwe)

**Workspace templates:** [marketing-workspace](https://github.com/iwe-org/marketing-workspace) — campaign memory for a marketing agent · [dev-workspace](https://github.com/iwe-org/dev-workspace) — project memory for a coding agent. Both ship as conformant [OKF](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md) v0.2 bundles, validated in CI on every commit.

**Agentic skills:** [iwe-org/skills](https://github.com/iwe-org/skills) — agentic AI skills for knowledge graph management. Contributors welcome.

**Building on IWE:** projects already embed IWE — as an agent-memory backend, as the graph layer of an LLM wiki engine, in research tooling. The practical integration surfaces today are the **CLI** and the **MCP server**; the [`liwe`](https://crates.io/crates/liwe) library is published but not yet API-stable, so pin your version if you build against it. A declared, stable integration surface is on the roadmap — if you're building on IWE, [tell us](https://github.com/iwe-org/iwe/discussions/362) what you depend on, so we know what not to break.

## License

[Apache License 2.0](LICENSE-APACHE)
