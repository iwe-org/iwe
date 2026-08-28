# Distill

Read this session with the user and write what they select. Nothing captures memory on its own: this skill is the only write path, and every document it writes was chosen by the user — by name ("remember that X") or from the proposals in §3.

Every command is a plain `iwe` command run from the repository root; the only permission needed is `Bash(iwe:*)`. A run is read → propose → select → write → record, the current session first, then the backlog.

Unattended (`claude -p`, a routine, a subagent — anywhere nobody can answer): read, list what you would have proposed, write nothing, advance nothing, and say so. An item named in the instruction is still written.

## 1. Check that memory is on

```bash
iwe internal claude session brief
```

Prints the `MEMORY.md` policy, a policy check, the knowledge filter, the frontmatter the store's documents carry, the schemas that bind them, the area hubs and what they include, the ten most recent documents and the recent rejections. A non-zero exit means memory is not enabled here: do not improvise a policy, `/iwe:init` turns it on. A policy check that is not `ok` is fixed with the user first (§6).

**The policy defines what is worth keeping and what a document looks like.** This body is procedure only; a policy silent on some part of the shape is followed as written, never filled in from habit.

## 2. Read the current session

Prefer the conversation you already hold. Read the transcript only when context was compacted away:

```bash
iwe internal claude session read
```

serves the current session from its distilled line, one `chunk_chars` window at a time; the header names `covers_from`, `covers_lines`, `transcript_lines` and `max_proposals`. Repeat with `--from <covers_lines>` until `covers_lines` reaches `transcript_lines`.

The current session is the row marked `current` in `session list`; none marked, name the newest and ask. `<session-id>/subagents/` directories are never read.

## 3. Propose, and let the user select

At most `max_proposals` candidates against the policy, each with a **title** (a specific noun phrase, not a sentence), a **one-line body**, the **target key** in the convention "how to write it" names, whatever classification "what to capture" asks for, and the **evidence** — a short quote or a line reference. No quotable evidence, no proposal: an idea you raised and argued well is a suggestion, not a decision.

Put them up as a numbered list and take a selection — the host's selection control where there is one, otherwise free-text indices. **Selecting nothing is a valid and common answer**: take it without argument and never re-propose the same items.

## 4. Write what was selected

"How to write it" is the whole of the shape: follow it verbatim and add no field, directory or link it does not name. Before the first write, read one recent document in full (`iwe retrieve -k <a key the brief listed>`) — the schema table shows fields, not body length, heading style or link usage.

Dedup every item first, as "dedup and updates" says. When it names no query:

```bash
iwe find --lexical "<the item's distinctive nouns>" --limit 5 \
  --filter '<the knowledge filter from the brief>' \
  --project 'title=$title,key=$key'
```

Read the plausible hits with `iwe retrieve -k <key>`. Update only a document covering the *same fact*; a related but different fact is a new document.

Then write, in the form the policy names:

```bash
iwe create <key> --strict --content - <<'EOF'
<the document, exactly as the policy shapes it>
EOF
```

```bash
iwe create --template <name> --strict --var <name>=<value>
```

```bash
iwe update -k <existing-key> --strict --content - <<'EOF'
<the merged document>
EOF
```

Whatever the policy says:

- `--strict` on every write. It enforces whatever the brief's schemas section names; when it fails, fix the item against the report, never drop the flag, and never pass `--set` alongside a template.
- After a create, the `PostToolUse` net may report that the new document closely matches one the store already has. Read the match; if it is the same fact, merge into the older key and delete the newer one, as "dedup and updates" says — then tell the user which key survived.
- The CLI is the write path, never the Write or Edit tool. When the `PostToolUse` hook reports a document written around the CLI, redo it through `iwe`.
- The body goes on stdin — `--content -` with a quoted heredoc; an inlined multi-line argument trips the shell-safety prompt.
- Provenance values come from the `session read` header for a transcript span, from the current time and `$CLAUDE_CODE_SESSION_ID` for the live conversation, in the shape the store's other documents use.
- Links are graph semantics: a link alone in its paragraph makes the target a child, inline or in a list item it is a reference. A key resolves against the document's own directory, a root-absolute one (`/components/session-record`) from the library root. Never mint a page the policy leaves to `/iwe:reflect`.

## 5. Record what happened

```bash
iwe internal claude session complete --lines now --wrote <key> --offered <count> --rejected "<a title the user turned down>" --title "<short subject>" --summary "<one line on what the session was about>"
```

Run this even when nothing was kept: `--lines now` moves the distilled line past this exchange, and the rejections are the only signal the policy loop learns from. `--wrote` and `--rejected` repeat; the command accumulates. `--wrote` is what ties a document to its session in the record; the document's own `session` field is the other direction. A written key under an area directory (`<area>/<slug>`) whose hub document `<area>` exists is linked into that hub by the command itself — it reports `linked <key> into its area hub <area>` — so a policy that groups into areas needs no separate append step. For a backlog session: `iwe internal claude session complete <id> --lines <covers_lines>`. Without `--lines` the distilled line stays put — that is how an offer declined mid-conversation is recorded.

## 6. Suggest a policy edit, when the rejections say something

When the recent rejections show a pattern — the same kind of item declined repeatedly, or a kind the user keeps accepting that the policy never asked for — propose one concrete `MEMORY.md` edit with exact wording and apply it only after they confirm:

```bash
iwe update -k MEMORY --strict --content - <<'EOF'
<the edited policy body>
EOF
```

A problem the policy check reported is proposed the same way. Otherwise say nothing.

## 7. Then offer the backlog

```bash
iwe internal claude session list
```

Report the count and ask whether to continue — all, a few named ones, or none. Each chosen session runs §2–§5 again, one at a time, with a "keep going?" between sessions. Rows marked `active` are other conversations in flight: leave them out unless the user names one. A large stale backlog need not be read: `iwe internal claude session adopt` marks every pending session distilled without reading a word, `iwe internal claude session adopt <id>` a named one; both refuse current and active rows.

## 8. Report

Name each key written, new or updated, and what is left in the backlog. Nothing commits: the documents are working-tree changes the user reviews as a diff.
