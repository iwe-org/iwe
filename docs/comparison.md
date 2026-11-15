# How IWE Compares

IWE turns a directory of markdown files into a knowledge graph — a connected structure you browse from your editor and your AI queries from the command line. Same files, same links, two interfaces.

That sentence is also the short answer to most questions on this page. The tools below are one of three things: a **markdown language server** (marksman, markdown-oxide), a **note-taking application** (Obsidian), or a **CLI notebook** (zk, telekasten.nvim). IWE overlaps all three because the graph is the product — the LSP server, the CLI, and the [MCP server](mcp.md) are three views of one in-memory model.

## Which Tool Should You Use

- **A markdown language server for almost any editor, with the least setup** — use **marksman**. It is the incumbent, it is small, and it does links, references, and diagnostics well.
- **Obsidian-style daily notes, block references, and tags without leaving your editor** — use **markdown-oxide**. Natural-language dates are better there than in IWE today.
- **A cross-platform GUI, mobile apps, a plugin ecosystem, canvas, and a web clipper** — use **Obsidian**. IWE has a native Mac app, [iWe](https://iwe.md/app/), and nothing on Windows, Linux, or mobile.
- **A Zettelkasten CLI with templates and an fzf browser** — use **zk**.
- **Notes as a graph you can query, refactor, and hand to an AI agent — from the editor *and* the command line** — use **IWE**.

If you are weighing IWE against one specific tool, skip to that section. If you want the argument against IWE, read *Where IWE Is Weaker* below; it is not a short list.

## marksman vs markdown-oxide

If you only want a markdown language server and are choosing between the two established ones, the sections below give the details. In short:

|                    | marksman                                                               | markdown-oxide                                           |
| ------------------ | ---------------------------------------------------------------------- | -------------------------------------------------------- |
| **Aimed at**       | Correct links, headings, and references in any editor                  | Obsidian's editing model inside a text editor            |
| **Link targets**   | Documents and headings                                                 | Documents, headings, and blocks (`[[doc#^block]]`)       |
| **Daily notes**    | None                                                                   | `:Today`, `:Daily next monday`, relative `prev` / `next` |
| **Diagnostics**    | Live wiki-link diagnostics                                             | Unresolved-reference diagnostics                         |
| **Editors**        | VS Code, Neovim, Vim, Emacs, Helix, Kakoune, Sublime Text, BBEdit, Zed | Neovim, VS Code, Zed, Helix, Kakoune                     |
| **Implementation** | F#/.NET                                                                | Rust                                                     |

**Pick marksman** for the widest editor support and the least setup, especially in Vim, Emacs, Sublime Text, or BBEdit. **Pick markdown-oxide** if you are coming from Obsidian and want daily notes, block references, and tags in your editor. **Pick IWE** if you also want to query the notes, restructure them, and let an AI agent work on them from the CLI or MCP. The two sections below say where each of them is ahead of IWE.

## IWE vs marksman

*Verified 2026-11-15 against [marksman's feature documentation](https://github.com/artempyanykh/marksman/blob/main/docs/features.md), its source, and its 2026-02-08 release, which is still the latest.*

[marksman](https://github.com/artempyanykh/marksman) is the default answer to "is there a markdown LSP?" It has the widest editor coverage of any tool on this page, it is the tool other markdown language servers get compared against, and for a large class of users it is enough.

**The difference in one sentence: marksman is a markdown language server; IWE is a knowledge graph that speaks LSP.** marksman's job ends at making links, headings, and references work correctly inside an editor — and it does that job well. IWE's LSP is one interface onto a graph that also has a CLI, a query language, and an MCP server; the editor features fall out of the graph rather than being the point.

| Capability                | IWE                                                                                                  | marksman                                                                                                             |
| ------------------------- | ---------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| **Link syntax**           | ✅ Markdown links, `[[wiki]]`, `[[wiki\|piped]]`, and [inclusion links](inclusion-links.md)           | ✅ Markdown inline links, reference links, and wiki links                                                             |
| **Link targets**          | ⚠️ Document granularity — `#fragment` is dropped when resolving ([Keys](keys.md))                    | ✅ Documents *and* headings — `[[doc#heading]]`, `[[#heading]]`                                                       |
| **Diagnostics**           | ❌ None published by the LSP; `iwe stats` reports broken links in batch                               | ✅ Live wiki-link diagnostics in the editor                                                                           |
| **Workspace symbols**     | ✅ Section-grain, carrying hierarchical path text (`Journal ⇒ 2026 ⇒ Jan 26`)                         | ✅ Heading-grain                                                                                                      |
| **Search ranking**        | ✅ Fuzzy + BM25 fused with [RRF](search-ranking.md), shared by CLI, LSP, and MCP                      | ⚠️ Subsequence matching, no relevance ranking                                                                        |
| **Reference counts**      | ⚠️ Whole-document backlink count as an [inlay hint](feature-inlay-hints.md)                          | ✅ Per-heading counts via code lens                                                                                   |
| **Graph transformations** | ✅ [Extract](feature-extract.md) / [inline](feature-inline.md) sections, section↔list, attach, squash | ❌ Not a goal                                                                                                         |
| **Query language**        | ✅ [MongoDB-style YAML](query-language.md) over frontmatter and graph edges                           | ❌ None                                                                                                               |
| **CLI**                   | ✅ [Full CLI](cli.md) — find, retrieve, tree, stats, schema validation, normalize, bulk update/delete | ❌ None (a standalone check command is planned; [PR #458](https://github.com/artempyanykh/marksman/pull/458) is open) |
| **MCP for AI agents**     | ✅ [14 tools](mcp.md), prompts, resources, file watching                                              | ❌ None                                                                                                               |
| **Table of contents**     | ❌ Not supported                                                                                      | ✅ Code action generates and updates a ToC                                                                            |
| **Rename**                | ✅ Rename a file, all references follow                                                               | ✅ Rename updates cross-references in the configured link style                                                       |
| **Daily notes**           | ⚠️ Template-based via [attach](feature-attach.md); no date resolution                                | ❌ None                                                                                                               |
| **Folding**               | ✅ Folding ranges over sections and lists                                                             | ❌ Not implemented                                                                                                    |
| **Workspace scoping**     | Directory the server starts in, plus `workspace.path`; honors `.gitignore`                           | VCS root or `.marksman.toml`; honors `.gitignore`, `.hgignore`, `.ignore`                                            |
| **Editors**               | ✅ Documented setup for VS Code, Neovim, Zed, Helix; any LSP client works                             | ✅ VS Code, Neovim, Vim, Emacs, Helix, Kakoune, Sublime Text, BBEdit, Zed                                             |
| **Implementation**        | Rust; pulldown-cmark AST with wikilinks enabled                                                      | F#/.NET; custom parser built on Markdig                                                                              |

**Choose marksman if** you want link and reference correctness in an editor marksman already supports, with a `.marksman.toml` and nothing else to learn — especially Emacs, Vim, Sublime Text, or BBEdit, where IWE has no documented setup. **Choose IWE if** the notes are something you also want to query, restructure, and expose to agents outside the editor.

## IWE vs markdown-oxide

*Verified 2026-11-15 against [oxide.md](https://oxide.md/), the markdown-oxide repository, and release `v0.25.12` (2026-06-29), which is still the latest.*

[markdown-oxide](https://github.com/Feel-ix-343/markdown-oxide) is a PKM language server aimed squarely at bringing Obsidian's editing model into a text editor: daily notes, block references, tags, callouts. It is an actively maintained solo project with a real community around it, and on the Obsidian-compatibility axis it is ahead of IWE.

**The difference in one sentence: markdown-oxide brings Obsidian's model into your editor; IWE treats the notes as a graph with an editor attached.** They overlap on links, backlinks, and completion, and diverge on almost everything else.

| Capability                 | IWE                                                                                                                               | markdown-oxide                                                                                                                                                                                                                                                                                                                                     |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Parsing**                | AST — pulldown-cmark with wikilinks enabled                                                                                       | Regular expressions ([`src/vault/mod.rs`](https://github.com/Feel-ix-343/markdown-oxide/blob/main/src/vault/mod.rs)), a deliberate tradeoff for indexing speed ([#2](https://github.com/Feel-ix-343/markdown-oxide/issues/2))                                                                                                                      |
| **Link granularity**       | Document — `#fragment` is dropped; the answer to "link to a section" is to [extract it](feature-extract.md) into its own document | Document, heading, and block — `[[doc#heading]]` and `[[doc#^block]]` resolve directly                                                                                                                                                                                                                                                             |
| **Search & ranking**       | ✅ Section-grain workspace symbols with path context; fuzzy + BM25 fused with [RRF](search-ranking.md)                             | ⚠️ Filename, heading, and block completion, fuzzy-matched with a server-side score ([`matcher.rs`](https://github.com/Feel-ix-343/markdown-oxide/blob/main/src/completion/matcher.rs)); no full-text relevance ranking, and the order depends on the client honoring `sortText` ([#409](https://github.com/Feel-ix-343/markdown-oxide/issues/409)) |
| **Daily & periodic notes** | ⚠️ Template-based via [attach](feature-attach.md) (`daily/{{today}}`); no date resolution                                         | ✅ `:Today`, `:Daily next monday`, `:Daily two days ago`, relative `prev` / `next` / `+7`                                                                                                                                                                                                                                                           |
| **Diagnostics**            | ❌ None from the LSP; `iwe stats` reports broken links in batch                                                                    | ✅ Unresolved-reference diagnostics in the editor                                                                                                                                                                                                                                                                                                   |
| **Reference counts**       | ⚠️ Whole-document backlink count as an inlay hint                                                                                 | ✅ Per-referenceable counts via code lens (needs enabling in some clients)                                                                                                                                                                                                                                                                          |
| **Backlinks**              | ✅ Go-to-references over reference and inclusion edges                                                                             | ✅ Files, headings, and blocks                                                                                                                                                                                                                                                                                                                      |
| **Hover preview**          | ✅ Linked note rendered inline, frontmatter stripped                                                                               | ✅ Hover preview                                                                                                                                                                                                                                                                                                                                    |
| **Tags**                   | ⚠️ Frontmatter `tags` are queryable; no inline `#tag` index or completion                                                         | ✅ Tag completion, and tag references through workspace symbols                                                                                                                                                                                                                                                                                     |
| **Graph transformations**  | ✅ Extract / inline sections, section↔list, attach, squash                                                                         | ❌ Linking and rename only                                                                                                                                                                                                                                                                                                                          |
| **Query language**         | ✅ MongoDB-style filters over frontmatter *and* graph edges, with projection, sort, limit                                          | ❌ None                                                                                                                                                                                                                                                                                                                                             |
| **CLI**                    | ✅ Full CLI, including bulk `update` / `delete` across the workspace by filter                                                     | ⚠️ Small CLI for opening daily notes and config files                                                                                                                                                                                                                                                                                              |
| **MCP for AI agents**      | ✅ 14 tools, prompts, resources, file watching, dry-run on every mutation                                                          | ❌ None (an MCP server was proposed in [#236](https://github.com/Feel-ix-343/markdown-oxide/pull/236) and closed unmerged)                                                                                                                                                                                                                          |
| **External commands**      | ✅ [Configurable CLI pipeline](feature-ai-commands.md) for block-level transformations                                             | ❌ None                                                                                                                                                                                                                                                                                                                                             |
| **Auto-formatting**        | ✅ [Full normalization](feature-normalization.md) — headers, lists, link titles, wrapping                                          | ❌ No formatting provider                                                                                                                                                                                                                                                                                                                           |
| **Editors**                | ✅ VS Code, Neovim, Zed, Helix                                                                                                     | ✅ Neovim, VS Code, Zed, Helix, Kakoune                                                                                                                                                                                                                                                                                                             |
| **Performance**            | ✅ Rust; [20,000 files in under a second](benchmark.md)                                                                            | ✅ Rust; fast in the common case, no published benchmark                                                                                                                                                                                                                                                                                            |

### On parsing

Both approaches work; they fail differently. Regex parsing is very fast to index and gets the common cases right. An AST costs more to build and settles a class of question by construction rather than by pattern: whether `[[` inside an inline-code span is a link, whether a fenced block nested in a list item is code, whether seven `#` characters open a heading. IWE pays the parse cost once at startup in the LSP and MCP servers, and per invocation in the CLI — [the benchmark](benchmark.md) is where that cost is measured rather than asserted.

### On link granularity

This is a design difference, not a scoreboard. markdown-oxide resolves heading and block anchors, so a link can point at a paragraph. IWE resolves at document granularity and drops `#fragment` when computing the target [key](keys.md); if a section is worth linking to, the [extract](feature-extract.md) code action turns it into its own document and rewrites the link for you. That is a real answer, but it is not the same answer, and if you want to point at a block *without* moving it, markdown-oxide does what you want and IWE does not.

## IWE vs Obsidian

*Verified 2026-11-15 for pricing, platform support, the official CLI, and MCP; the other feature rows are carried forward from May 2026.*

**Obsidian** is a GUI PKM application with strong visualization, an extensive plugin ecosystem, and Bases (typed database views), Canvas, and Web Clipper:

| Feature                                   | IWE                                                                                                       | Obsidian                                                                                            |
| ----------------------------------------- | --------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| **Editor Integration**                    | ✅ Works with your preferred text editor (VS Code, Neovim, Zed, Helix); a native Mac app is also available | ❌ Proprietary editor only                                                                           |
| **Cost**                                  | ✅ Free and open source (the Mac app is free)                                                              | ✅ Free, including for work; Sync $4–5/mo, Publish $8–10/mo, optional commercial license $50/user/yr |
| **Performance**                           | ✅ Rust-powered, instant operations on thousands of files                                                  | ⚠️ Electron-based, can be slower with large vaults                                                  |
| **Graph Transformations**                 | ✅ Automated extract/embed operations, section-to-list conversions                                         | ❌ Manual linking and organization                                                                   |
| **Query Language**                        | ✅ MongoDB-style YAML over frontmatter and graph edges                                                     | ✅ Bases (typed database views, queryable from the CLI with `base:query`) and the Dataview plugin    |
| **MCP / AI Integration**                  | ✅ Native MCP server with 14 tools, prompts, and resources                                                 | ⚠️ No official MCP server; agents can drive the official CLI, and MCP needs third-party plugins     |
| **External Commands**                     | ✅ Configurable CLI tools (AI agents, scripts, Unix tools)                                                 | ⚠️ Official CLI (1.12+) for scripting; in-editor pipelines need third-party plugins                 |
| **[Inclusion Links](inclusion-links.md)** | ✅ Native support with automatic linking                                                                   | ⚠️ Available via plugins                                                                            |
| **Auto-formatting**                       | ✅ Comprehensive markdown normalization on save                                                            | ⚠️ Basic formatting; plugins for advanced normalization                                             |
| **Batch Operations**                      | ✅ CLI for bulk transformations, update, delete with filters                                               | ⚠️ Official CLI (1.12+) scripts the running desktop app; no filter-based bulk update                |
| **Graph Visualization**                   | ⚠️ CLI-based dot export; a graph view in the Mac app only                                                 | ✅ Interactive graph view with customizable styling                                                  |
| **Canvas / Whiteboard**                   | ❌ Not supported                                                                                           | ✅ Native infinite canvas                                                                            |
| **Web Clipper**                           | ❌ Not supported                                                                                           | ✅ Native browser extension                                                                          |
| **Plugin Ecosystem**                      | ⚠️ LSP capabilities, CLI extensions, and MCP tools                                                        | ✅ Thousands of community plugins                                                                    |
| **Learning Curve**                        | ⚠️ The CLI and editor setup need terminal knowledge; the Mac app does not, and is macOS only              | ✅ GUI-friendly with intuitive interface                                                             |
| **Sync & Collaboration**                  | ✅ Git-based sync (free), works with any Git hosting                                                       | ⚠️ Obsidian Sync (paid) with shared vaults, or manual Git setup                                     |
| **Publishing**                            | ✅ Export to various formats via CLI                                                                       | ⚠️ Obsidian Publish (paid) or manual export                                                         |
| **Mobile Support**                        | ❌ No mobile app                                                                                           | ✅ Native mobile apps with sync                                                                      |

**The philosophical difference**: IWE is editor-agnostic and developer-focused, designed to sit inside existing technical workflows and to be readable by agents. Obsidian is a complete environment with its own ecosystem, better suited to people who want one application that does everything, with visual interfaces and plugin-level customization. If you are not already living in a text editor, Obsidian is very likely the right choice; on macOS, [iWe for Mac](https://iwe.md/app/) is a GUI over the same graph.

## IWE vs zk and telekasten.nvim

*Verified 2026-11-15 against zk `v0.15.7` (2026-10-15) and the telekasten.nvim README; telekasten.nvim has no tagged releases and its last commit is dated 2026-03-17.*

**zk** (zk-org) is a CLI-driven Zettelkasten tool with an LSP server and integrations for Neovim, VS Code, and Emacs. **telekasten.nvim** is a Neovim-only plugin with calendar, image paste, and Telescope integration:

| Feature                    | IWE                                                           | zk                                                          | telekasten.nvim                         |
| -------------------------- | ------------------------------------------------------------- | ----------------------------------------------------------- | --------------------------------------- |
| **Editor Support**         | ✅ VS Code, Neovim, Zed, Helix, any LSP-compatible editor      | ✅ Neovim, VS Code, Emacs via LSP                            | ❌ Neovim only                           |
| **Graph Transformations**  | ✅ Automated extract/embed, structural changes                 | ❌ Basic note creation and linking                           | ❌ Basic note creation                   |
| **Query Language**         | ✅ MongoDB-style YAML filters over frontmatter and graph edges | ⚠️ Filter flags for tags, links, mentions, and broken links | ❌ None                                  |
| **MCP / AI Integration**   | ✅ Native MCP server with 14 tools                             | ❌ None                                                      | ❌ None                                  |
| **External Commands**      | ✅ Configurable CLI tools (AI agents, scripts, Unix tools)     | ⚠️ Aliases and shell automation                             | ❌ Manual workflows only                 |
| **Performance**            | ✅ Rust-powered LSP                                            | ✅ Go-based CLI                                              | ⚠️ Lua-based, editor-dependent          |
| **Batch Operations**       | ✅ CLI for bulk operations with filters                        | ⚠️ Limited (notebook housekeeping)                          | ❌ One-note-at-a-time workflow           |
| **Auto-formatting**        | ✅ Built-in normalization                                      | ❌ Requires external tools                                   | ❌ Requires external tools               |
| **Note Templates**         | ✅ Note templates via `attach` command                         | ✅ Template-based note creation                              | ✅ Static templates                      |
| **Daily / Periodic Notes** | ⚠️ Template-based via `attach`; no date resolution            | ⚠️ Via templates                                            | ✅ Daily/weekly/monthly/quarterly/yearly |
| **Search Integration**     | ✅ LSP-based with any picker                                   | ✅ Built-in `fzf` browser                                    | ✅ Telescope/fzf integration             |
| **Calendar View**          | ❌ Not supported                                               | ❌ Not supported                                             | ✅ Calendar with note highlights         |
| **Image / Media Paste**    | ❌ Not supported                                               | ❌ Not supported                                             | ✅ Clipboard image paste                 |
| **Installation**           | ✅ Single binary + editor extension                            | ✅ Single binary + LSP                                       | ⚠️ Complex Neovim plugin setup          |

zk is a strong choice for Zettelkasten purists who want a CLI-first workflow with a simpler data model. telekasten.nvim is the best fit for Neovim users who want a polished journaling experience with calendar and image-paste workflows — neither of which IWE offers.

## IWE vs mdbase

*Verified 2026-11-15 against the [v0.3.0 spec tag](https://github.com/mdbase-dev/mdbase-spec), [mdbase-connect](https://github.com/mdbase-dev/mdbase-connect), and the implementation repositories. The file-watching row was not re-checked.*

**mdbase** is a specification (v0.3.0, tagged 2026-11-15 and described as its first stable release) for treating folders of markdown files as typed, queryable data collections. Types are JSON Schema documents and queries are CEL expressions. Its authoritative implementation is the Rust runtime in mdbase-connect, which ships an `mdbase` CLI and an MCP gateway, both in beta; a TypeScript library and a Go library also exist. An LSP server existed and was archived in November 2026, and an Obsidian plugin is in beta. It overlaps with IWE on querying, frontmatter typing, and validation — but not on graph transformations or editor integration.

| Feature                   | IWE                                                                                                                        | mdbase                                                                                     |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| **Project Maturity**      | ✅ Production tool: CLI, LSP, MCP shipping today                                                                            | ⚠️ Spec v0.3.0; implementations in beta, with conformance gaps named in the release record |
| **Editor Integration**    | ✅ LSP for VS Code, Neovim, Zed, Helix                                                                                      | ⚠️ Obsidian plugin (beta); the LSP server is archived                                      |
| **CLI**                   | ✅ `iwe` with find, retrieve, update, delete, normalize, schema, etc.                                                       | ⚠️ `mdbase` CLI (beta) — validate and query                                                |
| **MCP / AI Integration**  | ✅ Native MCP server with 14 tools                                                                                          | ⚠️ Hosted or self-hostable MCP gateway (beta)                                              |
| **Query Language**        | ✅ MongoDB-style filters over frontmatter and graph edges                                                                   | ✅ CEL expressions for filters, ordering, and projections, plus saved views                 |
| **Frontmatter Schemas**   | ✅ [Document schemas](document-schema.md) bound to documents by path, plus inference from existing documents (`iwe schema`) | ✅ JSON Schema type definitions stored in the collection                                    |
| **Validation**            | ✅ `iwe schema validate`; `--strict` rejects a write that breaks a bound schema                                             | ✅ Validation against the type definitions                                                  |
| **Graph Transformations** | ✅ Extract, inline, attach, normalize, rename with link updates                                                             | ⚠️ Rename updates references; no structural refactors                                      |
| **Inclusion Links**       | ✅ Native block-reference inclusion                                                                                         | ❌ Wikilinks, markdown links, and bare paths                                                |
| **Performance**           | ✅ Rust core, in-memory graph                                                                                               | Rust runtime (beta); TypeScript library with SQLite-backed cache                           |
| **Batch Operations**      | ✅ `iwe update`/`delete` with filters                                                                                       | ✅ `batchUpdate` / `batchDelete` library calls                                              |
| **File Watching**         | ✅ Built into LSP and MCP servers                                                                                           | Not re-checked for v0.3                                                                    |

**mdbase's advantage**: vendor-neutral type definitions in JSON Schema, with validation defined by a spec rather than by one tool, suit teams that want to enforce frontmatter contracts across tools. If you primarily need typed records rather than a knowledge graph, mdbase's typed-collection model may fit better.

## Where IWE Is Weaker

Everything above is easy to write in IWE's favor, so here is the other list — checked against IWE `0.33.0` on 2026-11-15.

- **Daily and periodic notes.** IWE has templates: an [attach](feature-attach.md) action with `key_template = "daily/{{today}}"` creates and links today's note. It has no natural-language date resolution, no `[[next monday]]` as a link target, no relative jump commands, and no weekly, monthly, or yearly periods. markdown-oxide is clearly ahead here; telekasten.nvim has the weekly, monthly, and yearly periods.
- **Live diagnostics.** IWE's LSP publishes none. Broken links are detectable, but only in batch, through `iwe stats`. Both marksman and markdown-oxide flag unresolved references as you type.
- **Section and block link targets.** IWE resolves links at document granularity and drops `#fragment`. marksman resolves heading anchors; markdown-oxide resolves headings and blocks. A `[[note#section]]` wiki link counts as a link to the note and keeps its anchor through a rename, but the anchor itself is not resolved. Extraction is IWE's answer, and it is a different answer, not a strictly better one.
- **Per-heading reference counts.** IWE's inlay hint shows one backlink count for the whole document. marksman and markdown-oxide both show counts per heading.
- **Tags.** IWE queries frontmatter `tags` arrays well, but has no index, completion, or go-to-definition for inline `#tag` syntax. markdown-oxide does.
- **Aliases.** A document has one title. If you want a note findable under several names, IWE has no answer today.
- **Editor breadth.** IWE documents VS Code, Neovim, Zed, and Helix. marksman additionally covers Emacs, Vim, Sublime Text, BBEdit, and Kakoune; markdown-oxide covers Kakoune. Any LSP client can be wired up by hand, but "by hand" is the operative phrase.
- **Table of contents.** marksman generates and maintains one as a code action. IWE does not.
- **Mobile and non-Mac GUI.** Obsidian has native mobile apps with sync and a desktop app on every platform. IWE has no mobile app, and its GUI, [iWe for Mac](https://iwe.md/app/), is macOS only.
- **Plugin ecosystem, canvas, and web clipper.** Obsidian wins these outright, and it is not close. Its interactive graph view runs everywhere; IWE has a graph view only in the Mac app, and DOT export from the CLI.
- **First-run clarity.** IWE is a binary, an editor config, and a directory of markdown. If you have never used an LSP-backed tool before, Obsidian will be working before IWE is configured. The Mac app and the Claude Code plugin's `/iwe:init` shorten that path, but only on macOS and in Claude Code.

## What IWE Adds Beyond a Markdown LSP

The tables above compress a lot. The capabilities that have no counterpart in a markdown language server — graph transformations, the query language, the MCP server, search ranking, schema inference, external commands, normalization, and inclusion links — are catalogued with links to their documentation in the [Feature Overview](feature-overview.md).

## Method and Dates

- **Each comparison block carries its own verification date.** marksman, markdown-oxide, zk, telekasten.nvim, and mdbase were re-verified on 2026-11-15 against their documentation, source, and releases. Obsidian's pricing, platform support, official CLI, and MCP status were re-checked the same day; its other feature rows are carried forward from May 2026.
- **Claims about IWE** were checked against IWE `0.33.0`.
- **Claims about other tools** link to their documentation, source, or issue tracker wherever the claim is not obvious from the tool's front page.
- **Corrections are welcome and will be applied.** If something here is wrong, out of date, or unfair — including by the maintainers of the tools compared — [open an issue](https://github.com/iwe-org/iwe/issues) and it will be fixed. This page is maintained, not marketing.
