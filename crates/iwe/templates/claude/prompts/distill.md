# Distill

Read sessions with the user and write what they select. Nothing reaches memory unselected: this skill is the only write path, and every document it writes was chosen by the user — by name ("remember that X") or from the candidates §5 puts to them.

Every command is a plain `iwe` command run from the repository root; the only permission needed is `Bash(iwe:*)`. A run is read → stage → select → write → record. Reading fills an **inbox** that lives on disk under `.iwe/claude/`, outside the graph: candidates go there before anyone is asked about them, and selection empties it. That is what lets a whole backlog be read once, deduped as a whole and asked about in one pass, and what lets a run that compacts or stops half-way pick up where it left off.

Unattended (`claude -p`, a routine, a subagent — anywhere nobody can answer): read and stage, write nothing into the store, and say what is waiting in the inbox. An item named in the instruction is still written.

## 1. Check that memory is on

```bash
iwe internal claude session brief
```

Prints the `MEMORY.md` policy, a policy check, the knowledge filter, the frontmatter the store's documents carry, the schemas that bind them, the area hubs and what they include, the ten most recent documents and the recent rejections. A non-zero exit means memory is not enabled here: do not improvise a policy, `/iwe:init` turns it on. A policy check that is not `ok` is fixed with the user first (§8).

**The policy defines what is worth keeping and what a document looks like.** This body is procedure only; a policy silent on some part of the shape is followed as written, never filled in from habit.

## 2. Choose the span

The current session by default: the row marked `current` in

```bash
iwe internal claude session list
```

none marked, name the newest and ask. Add whatever else the user names, or the backlog when they ask for it — report the count and get a scope before spending anything, in user turns, not lines: a session with three hundred pending lines and two turns is an autonomous run that settled nothing; one with forty turns is where the choices were made. The usual recommendation is "read these few, adopt the rest", both numbers named.

```bash
iwe internal claude session adopt
iwe internal claude session adopt <id>
```

Adopting marks pending sessions distilled through their current length without reading a word and drops whatever was staged against them; it refuses the current session and any other live conversation. Rows marked `active` are other conversations in flight: leave them out unless the user names one. A closing line naming staged proposals is an earlier run that stopped: the inbox is on disk, so pick those up at §4 rather than reading their sessions again.

## 3. Read the span and stage what it holds

For the current session, prefer the conversation you already hold; read the transcript only when context was compacted away:

```bash
iwe internal claude session read
```

serves a session from its distilled line, one `chunk_chars` window at a time; the header names `covers_from`, `covers_lines`, `transcript_lines` and `max_proposals`. Repeat with `--from <covers_lines>` until `covers_lines` reaches `transcript_lines`. `<session-id>/subagents/` directories are never read.

At most `max_proposals` candidates per window, each weighed against the policy, with a **title** (a specific noun phrase, not a sentence), a **one-line body**, the **target key** in the convention "how to write it" names, whatever classification "what to capture" asks for, and the **evidence** — the line range plus a short quote. No quotable evidence, no candidate: an idea you raised and argued well is a suggestion, not a decision.

Dedup each candidate against the store as "dedup and updates" says; when it names no query:

```bash
iwe find --lexical "<the item's distinctive nouns>" --limit 5 \
  --filter '<the knowledge filter from the brief>' \
  --project 'title=$title,key=$key'
```

Read the plausible hits with `iwe retrieve -k <key>`. A document covering the *same fact* makes the candidate an update of that key, carried as `updates:`; a related but different fact keeps it a new document.

Stage each candidate, one call apiece:

```bash
iwe internal claude session stage <id> --content - <<'EOF'
title: <a specific noun phrase>
key: <the target key>
body: <one line>
evidence: 'lines <from>-<to>, "<a short quote>"'
classification: <only what the policy names>
updates: <an existing key, only when the dedup search found the same fact>
EOF
```

Then close the read:

```bash
iwe internal claude session complete <id> --lines <covers_lines> --offered <n>
```

`--lines now` for the current session; `<n>` counts what was staged, not what was asked. Staging writes nothing into the store and asks nobody anything.

**More than one session: one subagent each.** Reading is the long half and nobody has to sit through it. Give each subagent one session id, the policy's "what to capture", "how to write it" and "dedup and updates" sections, and the knowledge filter from the brief; have it run this section for its session and stop there. Each stages into its own session record, so parallel readers never collide.

## 4. Dedup the inbox against itself

```bash
iwe internal claude session inbox
```

Every staged candidate nobody has been asked about, grouped by the key it targets, so a fact several sessions raised shows up as one group instead of once per session. Merge each such group into a single candidate whose evidence names every contributing session and line range. `iwe internal claude session inbox <id>` prints one session's entries in full, evidence included, when a group needs a closer look.

## 5. Ask about each

The candidate is the **key**, not the staged entry: a group §4 merged is one decision, and it settles on every session that raised it. `session inbox` numbers the groups; keep those numbers.

**Round 0, always.** One question before any of the detail: `Keep all <n>` / `Skip all` / `Let me pick`. Most runs end here, and `Skip all` in particular should cost one keystroke, not one per candidate.

**Five or fewer, one at a time.** Show a single candidate in full — key, title, body, whether it is new or an update, classification, evidence — take the decision, and put up the next at once. Nothing runs between two questions: no lookup, no write. Use the host's question control with three options, `Remember`, `Skip` and `Edit`.

**More than five, in batches.** Print the whole inbox first, as a numbered list, one line per candidate — number, key, one-line body, `new` or `updates <key>`, and how many sessions raised it. That list carries the detail; the questions carry only the disposition. Then ask with the host's control, up to four questions of four candidates each in one call — sixteen a round, `multiSelect` on every question, options labelled `<number>. <key>`, the one-line body as the option's description. **Checked is remember, unchecked is skip**: one pass, no third state. Say up front how many rounds it will take. Where the host has no such control, the same numbered list plus one plain-text ask does the same work in one exchange — *"reply with the numbers to keep: `1 3 7-9`, or `all` / `none`, and say anything you want changed."*

**`Edit` is the free text either way** — the control's own `Other` answer, or text attached to a `Remember` or a `Skip`. It is instruction, not commentary. A corrected title, body or key, a detail to add, a different classification: apply it, say in one line what the candidate now is, and keep going — it is remembered in its edited form. An edit that changes the key means the candidate is written under the new key and the staged entry is settled in §7 as `--rejected "<title> — rewritten as <new-key>"`. A reason the candidate is wrong or unwanted makes it a skip, carried into §7 as `--rejected "<title> — <reason>"`. A remark that reaches past one candidate ("skip anything about the eval", "these two are the same fact") reshapes what is still to come: drop, merge or reword before it appears.

Nothing is written until every candidate has its answer (§6), and **skipping everything is a valid and common answer** — take it without argument and never re-propose the same items. Never open with the whole set as prose, and never make the user retype a title.

## 6. Write what was remembered

Once the last item is answered, write the remembered ones together. "How to write it" is the whole of the shape: follow it verbatim and add no field, directory or link it does not name — capture invents no area and no hub, however plainly a backlog clusters. Before the first write, read one recent document in full (`iwe retrieve -k <a key the brief listed>`) — the schema table shows fields, not body length, heading style or link usage. A key an edit changed is looked up again (§3) before it is written.

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

## 7. Record what happened

Once per session a candidate drew from, after the last answer:

```bash
iwe internal claude session complete <id> --wrote <key> --rejected "<a title the user turned down and why>" --drop-pending --title "<short subject>" --summary "<one line on what the session was about>"
```

`--wrote <key>` settles the staged candidate under that key, and a merged candidate settles on every session that raised it. `--rejected` settles by title, for the skips whose reason is worth keeping. `--drop-pending` then turns down whatever is left staged on that session, recording each title in the ledger — the sweep that closes the run and the whole of what `Skip all` needs. It cannot tell a candidate nobody asked about from one the user skipped, so it goes in the last call, never between rounds. Run this even when nothing was kept: the rejections are the only signal the policy loop learns from. `--wrote` and `--rejected` repeat; the command accumulates. `--wrote` is what ties a document to its session in the record; the document's own `session` field is the other direction. A written key under an area directory (`<area>/<slug>`) whose hub document `<area>` exists is linked into that hub by the command itself — it reports `linked <key> into its area hub <area>` — so a policy that groups into areas needs no separate append step. A `warning:` line naming a written key the knowledge filter does not select means that document is invisible to session start and to every query the filter scopes: fix the document or its link with the user before going on, and never leave it as written.

`--lines` already advanced in §3 and is not repeated here. An offer declined mid-conversation, with nothing read, is recorded with `--offered` and `--rejected` alone: the distilled line stays put.

## 8. Suggest a policy edit, when the rejections say something

When the recent rejections show a pattern — the same kind of item declined repeatedly, or a kind the user keeps accepting that the policy never asked for — propose one concrete `MEMORY.md` edit with exact wording and apply it only after they confirm:

```bash
iwe update -k MEMORY --strict --content - <<'EOF'
<the edited policy body>
EOF
```

A problem the policy check reported is proposed the same way. Otherwise say nothing.

## 9. Report

Name each key written, new or updated, anything still staged in the inbox, and what is left in the backlog. Nothing commits: the documents are files in the store the user reviews.
