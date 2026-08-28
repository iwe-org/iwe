# Distill

Read this session with the user and write what they select. Nothing captures memory on its own: this skill is the only write path, and every document it writes was chosen by the user — by name ("remember that X") or from the proposals in §3.

Every command is a plain `iwe` command run from the repository root; the only permission needed is `Bash(iwe:*)`. A run is read → propose → select → write → record, the current session first, then the backlog: every proposal is prepared before the first question, the questions come one after another, and the writes come after the last answer.

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

## 3. Prepare the proposals, then ask about each

At most `max_proposals` candidates against the policy, each with a **title** (a specific noun phrase, not a sentence), a **one-line body**, the **target key** in the convention "how to write it" names, whatever classification "what to capture" asks for, and the **evidence** — a short quote or a line reference. No quotable evidence, no proposal: an idea you raised and argued well is a suggestion, not a decision.

Prepare the whole set before asking anything, dedup included. Dedup every candidate as "dedup and updates" says; when it names no query:

```bash
iwe find --lexical "<the item's distinctive nouns>" --limit 5 \
  --filter '<the knowledge filter from the brief>' \
  --project 'title=$title,key=$key'
```

Read the plausible hits with `iwe retrieve -k <key>`. A document covering the *same fact* makes the candidate an update of that key; a related but different fact keeps it a new document. The proposal says which.

Then put them to the user **one at a time, in quick succession**: show a single proposal in full — title, body, key and whether it is new or an update, classification, evidence — take the decision, and put up the next one at once. Nothing runs between two questions: no lookup, no write. The decision is **remember**, **skip** or **edit**. Use the host's question control where there is one, with three options, `Remember`, `Skip` and `Edit`; otherwise ask in plain text and read the reply.

`Edit` means the user has something to say about the item: ask what in plain text and read the reply — the control's own free-text answer, or text attached to a `Remember` or `Skip`, already is it. It is instruction, not commentary. A corrected title, body or key, a detail to add, a different classification: apply it, say in one line what the item now is, and move on — the item is remembered in its edited form. A reason the item is wrong or unwanted makes it a skip, carried into §5 as `--rejected "<title> — <reason>"`. A remark that reaches past the item ("skip anything about the eval", "this and the previous one are the same fact") reshapes the proposals still to come: drop, merge or reword them before they appear.

Nothing is written until every item has its answer (§4). **Skipping every item is a valid and common answer**: take it without argument and never re-propose the same items. Never open with the whole set, never ask for a batch of indices, and never write between two questions.

## 4. Write what was remembered

Once the last item is answered, write the remembered ones together. "How to write it" is the whole of the shape: follow it verbatim and add no field, directory or link it does not name. Before the first write, read one recent document in full (`iwe retrieve -k <a key the brief listed>`) — the schema table shows fields, not body length, heading style or link usage. A key an edit changed is looked up again (§3) before it is written.

Write in the form the policy names:

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

Run this even when nothing was kept: `--lines now` moves the distilled line past this exchange, and the rejections are the only signal the policy loop learns from. `--wrote` and `--rejected` repeat; the command accumulates. `--wrote` is what ties a document to its session in the record; the document's own `session` field is the other direction. A written key under an area directory (`<area>/<slug>`) whose hub document `<area>` exists is linked into that hub by the command itself — it reports `linked <key> into its area hub <area>` — so a policy that groups into areas needs no separate append step. A `warning:` line naming a written key the knowledge filter does not select means that document is invisible to session start and to every query the filter scopes: fix the document or its link with the user before going on, and never leave it as written. For a backlog session: `iwe internal claude session complete <id> --lines <covers_lines>`. Without `--lines` the distilled line stays put — that is how an offer declined mid-conversation is recorded.

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
