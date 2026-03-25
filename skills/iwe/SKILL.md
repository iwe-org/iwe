---
name: iwe
description: Use this skill when working in an IWE knowledge-graph workspace, especially to help an agent read, navigate, retrieve context from, and safely refactor Markdown notes with the `iwe` CLI instead of ad-hoc file edits. Covers project discovery, context-building, and structural operations such as `find`, `tree`, `retrieve`, `new`, `extract`, `inline`, `rename`, and `delete`.
---

# IWE

Use this skill when the repository or workspace is an IWE project, or when the user wants an agent to operate on an IWE knowledge base through the `iwe` CLI.

IWE is local-first and markdown-based. The important agent-facing idea is: prefer `iwe` commands for graph-aware reads and refactors, and only fall back to direct file edits when the CLI does not cover the task.

## Quick start

1. Confirm the workspace is an IWE project by checking for `.iwe/config.toml`.
2. Read `.iwe/config.toml` before assuming where notes live. `library.path` may point to a subdirectory.
3. Use `iwe find`, `iwe tree`, and `iwe retrieve` to explore before editing.
4. For structural note changes, prefer `iwe new`, `iwe extract`, `iwe inline`, `iwe rename`, and `iwe delete`.
5. Use `--dry-run` before destructive or high-impact operations.

## When to use which command

- Use `iwe find` to discover likely documents, roots, and references.
- Use `iwe tree` to understand hierarchy and entry points.
- Use `iwe retrieve` to build agent context with children, parents, backlinks, and optionally inline links.
- Use `iwe squash` when the goal is one merged markdown artifact for review, export, or LLM context.
- Use `iwe new` to create a note through project templates.
- Use `iwe extract` to split a section into a new document while keeping an inclusion link.
- Use `iwe inline` to embed a referenced document back into its parent.
- Use `iwe rename` to move or rename a document while updating references.
- Use `iwe delete` to remove a document while cleaning references safely.

## Recommended workflow

### 1. Discover the workspace

Start small:

```bash
test -f .iwe/config.toml
sed -n '1,200p' .iwe/config.toml
iwe find --roots
iwe tree --depth 2
```

If the user mentions a topic but not an exact key:

```bash
iwe find "topic words"
```

### 2. Build context deliberately

For a likely document:

```bash
iwe retrieve -k some-key --dry-run
iwe retrieve -k some-key -d 1 -c 1
```

Increase scope only when needed:

```bash
iwe retrieve -k some-key -d 2 -c 2
iwe retrieve -k some-key -l
```

Use `-f keys` or `-f json` when chaining commands or programmatic processing is easier than parsing markdown.

### 3. Prefer graph-aware refactors

If the task is a note-graph operation, use the dedicated command instead of editing links by hand.

- Split one section out: `iwe extract`
- Merge linked content back: `iwe inline`
- Rename a note key or move it into a subdirectory: `iwe rename`
- Remove a note and clean references: `iwe delete`

For any operation that may affect multiple files, check impact first:

```bash
iwe rename old-key new-key --dry-run
iwe delete old-key --dry-run
iwe inline parent --reference child --dry-run
```

## Guardrails

- Do not assume markdown files live at repository root; check `library.path`.
- Do not hand-edit references if `iwe` already has a safe operation for that change.
- Do not retrieve large context blindly; use `--dry-run` first when depth or context may expand significantly.
- Do not use `iwe delete` without `--dry-run` or explicit user intent when the note may still be important.
- If the task depends on exact CLI flags or behavior details, read the relevant reference file in `references/`.

## Reference map

- For read and navigation flows, read [references/read-and-navigate.md](references/read-and-navigate.md).
- For write and refactor flows, read [references/write-and-refactor.md](references/write-and-refactor.md).
- For project initialization and config assumptions, read [references/project-setup.md](references/project-setup.md).
