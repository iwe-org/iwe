# IWE — A shared memory for you and your AI agents

> Your notes are your agent’s memory. iWe is where you work on them together.

[![Crates.io](https://img.shields.io/crates/v/iwe.svg)](https://crates.io/crates/iwe)
[![License](https://img.shields.io/crates/l/iwe.svg)](https://github.com/iwe-org/iwe/blob/master/LICENSE-APACHE)
[![Documentation](https://img.shields.io/badge/docs-iwe.md-blue)](https://iwe.md)

Too many AI conversations start from zero. You explain your project, your decisions, your reasoning — then find yourself explaining it all again next session. Your own notes live somewhere else, out of the agent's reach.

Or worse, the agent “remembers” something in its own internal memory, but you can't easily see what it saved or correct what it got wrong. And that memory doesn't travel with you to the other agents you work with.

IWE gives you and your AI agents **one shared place to work, right inside your project**: a folder of connected Markdown documents — specs, plans, decisions, and more. Keep your own ideas there, ask your agent to save what you learn together, and return to that context as your work grows. You can read, edit, and organize those documents yourself — and they stay yours.

## Choose how you work

There are four ways to work with IWE. Mix and match them to fit your day:

| Tool | How you'll use it |
| --- | --- |
| **[iWe for Mac](#iwe-for-mac)** | Write and browse in a native app, and connect your existing Claude/Codex agent. |
| **[Your own editor](#your-own-editor)** | Stay in Neovim, Helix, VS Code, or Zed, with note navigation, backlinks, and autocomplete. |
| **[CLI](#cli)** | Search and manage your notes from the terminal or scripts. |
| **[Claude/Codex](#claudecodex)** | Let your agent read, write, and organize memory while you work through conversation. |

All four work with the same Markdown documents. Draft a spec in your editor, develop it with Claude/Codex, browse it in iWe, or search it from the CLI. Use any combination you like; each tool also works independently.

**Bring your own AI.** Neither the IWE CLI nor the iWe Mac app includes an AI model. Both work on their own as tools for your notes. When you want AI help, use your existing Claude/Codex agent — in terminal or directly in the Mac app.

<a href="https://iwe.md/images/app/GH-screen-1.1.png"><img src="https://iwe.md/images/app/GH-screen-1.1.png" alt="iWe for Mac showing project plans and connected documents" width="820"></a>

<p>
  <a href="https://iwe.md/images/app/GH-screen-2.png"><img src="https://iwe.md/images/app/GH-screen-2.png" alt="Bug tracking with charts" width="32%"></a>
  <a href="https://iwe.md/images/app/GH-screen-3.png"><img src="https://iwe.md/images/app/GH-screen-3.png" alt="Technical documents with code" width="32%"></a>
  <a href="https://iwe.md/images/app/GH-screen-4.png"><img src="https://iwe.md/images/app/GH-screen-4.png" alt="Release documents" width="32%"></a>
</p>

*Plans · Charts · Technical documents · Releases — click a screenshot to view it full size.*

## Why IWE

- **Shared context.** Your agent can use the same notes you write for yourself. Notes link to each other, and one note can belong to several topics, so you can follow an idea wherever it connects.
- **Memory you can read.** Decisions and lessons saved by your agent are ordinary notes you can browse, correct, or delete.
- **Your notes stay yours.** Everything is Markdown in a folder on your machine. Back it up, sync it, or keep it in git. You don't need an IWE cloud account to keep your notes; connecting an AI agent involves that agent's own setup and account requirements.

## iWe for Mac

If you'd like a dedicated space for your notes, use the native Mac app. Open a folder of notes and you get a fast, keyboard-friendly space to write and browse. You can also connect your existing Claude/Codex agent to work alongside you.

- **Works with your Claude/Codex agent.** Connect the agent you already use, ask questions about your notes, and have it draft or reorganize them. Watch its edits appear in the document as it works.
- **Room to experiment.** Each agent run is one undo step — try an idea, and roll it back if you don't like it.
- **Follow your ideas.** Jump between related notes, search in a few keystrokes, and view Mermaid diagrams right in your documents. Your editor and other tools can keep working on the same files.

**[Download iWe for Mac](https://github.com/iwe-org/iwe-mac/releases)**

Open a notes folder and start with a note about something you're working on. Once Claude/Codex is connected, try asking it to summarize that note or help you develop an idea.

## Your Own Editor

Keep writing in [Neovim](https://iwe.md/docs/editors/neovim/), [Helix](https://iwe.md/docs/editors/helix/), [VS Code](https://iwe.md/docs/editors/vscode/), or [Zed](https://iwe.md/docs/editors/zed/). IWE adds link autocomplete, backlinks, search, and safe renaming across your notes.

Follow the guide for your editor, open your notes folder, and try linking two notes with autocomplete. You can use IWE for your own writing without connecting an AI agent.

## CLI

Prefer the terminal? Use IWE to search, create, and reorganize notes, or work it into your scripts. The CLI has no AI built in: these commands work on their own. Your existing Claude/Codex agent can use IWE's tools too.

Install with Homebrew, or choose another option from the [installation guide](https://iwe.md/docs/getting-started/installation/):

```bash
brew install iwe-org/iwe/iwe
```

Then run these commands inside your notes folder:

```bash
iwe init
iwe find
iwe find --lexical "project decisions"
```

`iwe find` lists your notes; `--lexical` searches their content. Explore more in the [CLI reference](https://iwe.md/docs/cli/) and [workflow examples](docs/cli-workflows.md).

## Claude/Codex

You can use IWE entirely through your agent. Ask Claude/Codex to save decisions, look up past work, and organize what you've learned. The agent works with your notes through IWE, and you can open those files yourself whenever you want.

IWE isn't tied to one AI. The same notes work with Claude Code, Claude Desktop, Codex, Gemini, Cursor, and any tool that supports the [Model Context Protocol](https://modelcontextprotocol.io).

**Give Claude/Codex a memory** — for Claude Code, install the plugin, then run `/iwe:init` in any project you want remembered. For Codex, follow the agent connection steps below.

```text
/plugin marketplace add iwe-org/skills
/plugin install iwe@iwe-org
/iwe:init
```

Then try the decision-saving example above in your project.

**Connect another agent** — with Homebrew installed, install IWE:

```bash
brew install iwe-org/iwe/iwe
```

Run `iwe init` in your notes folder, then follow the **[agent connection guide](https://iwe.md/docs/agentic/)** to add the `iwec` MCP server with that folder as its working directory. For other installation options, see the [installation guide](https://iwe.md/docs/getting-started/installation/).

Once connected, ask your agent: “Explore my IWE notes and summarize what's here.”

Or let the agent set itself up — paste this into any agent that can run commands:

```text
Set up IWE for my notes: install it (brew install iwe-org/iwe/iwe,
npm install -g @iwe-org/iwe, or cargo install iwe iwes iwec), run `iwe init`
in my notes directory, then add the `iwec` MCP server with its working
directory set to that folder.
Docs: https://iwe.md/docs/agentic/
```

## Learn More

- [Getting Started](https://iwe.md/docs/getting-started/installation/) — install and set up
- [Working with AI](https://iwe.md/docs/agentic/) — connect your agents
- [Usage Guide](https://iwe.md/docs/getting-started/usage/) — everyday workflows
- [Examples](https://iwe.md/docs/examples/) — how people use IWE
- [CLI Reference](https://iwe.md/docs/cli/) and [MCP Server](https://iwe.md/docs/agentic/mcp/) — for the technically curious

**Ready-made workspaces:** [dev-workspace](https://github.com/iwe-org/dev-workspace) — project memory for a coding agent · [marketing-workspace](https://github.com/iwe-org/marketing-workspace) — campaign memory for a marketing agent.

## Get Involved

Trying IWE with your own notes? Tell us what felt useful and where you got stuck. Questions, rough edges, and examples of how you use it are all welcome in [Discussions](https://github.com/iwe-org/iwe/discussions).

IWE is open source. You can also report a [bug](https://github.com/iwe-org/iwe/issues) or help improve the [documentation](docs/).

**Community:** [Twitter/X](https://x.com/iwe_md) · [Reddit](https://www.reddit.com/r/iwe/) · [Discussions](https://github.com/iwe-org/iwe/discussions)

**Building on IWE?** The CLI and MCP server are the supported ways to integrate today. [Tell us](https://github.com/iwe-org/iwe/discussions/362) what you depend on so we know what not to break.

## License

[Apache License 2.0](LICENSE-APACHE)
