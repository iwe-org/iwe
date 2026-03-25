# Write And Refactor

Use this reference for graph-aware write operations. Prefer these commands over manual markdown edits when the task changes note structure or references.

## `iwe new`

Create a document through project templates.

```bash
iwe new <TITLE> [OPTIONS]
```

Useful examples:

```bash
iwe new "My New Note"
iwe new "Meeting Notes" --content "Discussed project timeline"
pbpaste | iwe new "Clipboard Note"
iwe new "Daily Journal" --template journal
```

Agent advice:

- Prefer `iwe new` over creating files by hand.
- The command prints the created absolute path, which is useful for follow-up edits.
- Check config if template behavior matters.

## `iwe extract`

Split a section into a new document and replace it with an inclusion link.

```bash
iwe extract <KEY> [OPTIONS]
```

Safe workflow:

```bash
iwe extract my-document --list
iwe extract my-document --section "Configuration" --dry-run
iwe extract my-document --section "Configuration"
```

Use when:

- A document has become too large
- One section deserves its own reusable note
- You want to preserve structure without copy-paste

## `iwe inline`

Embed a referenced document back into the source document.

```bash
iwe inline <KEY> [OPTIONS]
```

Safe workflow:

```bash
iwe inline my-document --list
iwe inline my-document --reference "getting-started" --dry-run
iwe inline my-document --reference "getting-started"
```

Important behavior:

- Default behavior deletes the target document after inlining.
- Use `--keep-target` when the target should remain reusable elsewhere.
- Use `--as-quote` when the content should become a blockquote instead of a section.

## `iwe rename`

Rename or move a document while updating references.

```bash
iwe rename <OLD_KEY> <NEW_KEY> [OPTIONS]
```

Safe workflow:

```bash
iwe rename old-key new-key --dry-run
iwe rename old-key new-key
```

Use when:

- Renaming a note key
- Moving a note into a subdirectory
- Standardizing naming conventions

## `iwe delete`

Delete a document and clean references automatically.

```bash
iwe delete <KEY> [OPTIONS]
```

Safe workflow:

```bash
iwe delete obsolete-doc --dry-run
iwe delete obsolete-doc --force
```

Important behavior:

- Inclusion links are removed.
- Inline links are converted to plain text.
- Without `--force`, the command prompts for confirmation.

## Command selection guide

- Create a note: `iwe new`
- Split one section out: `iwe extract`
- Merge linked content in: `iwe inline`
- Change key or path: `iwe rename`
- Remove a note safely: `iwe delete`

## Agent advice

- Use `--dry-run` first whenever multiple files may be touched.
- If the user asks for a structural change and the CLI supports it, use the CLI instead of editing markdown references by hand.
- After a write operation, inspect affected files or rerun `find`/`retrieve` if you need to confirm the graph state.
