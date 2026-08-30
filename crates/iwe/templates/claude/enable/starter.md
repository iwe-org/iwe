# Memory policy

`MEMORY.md` is the switch and the policy for durable memory in this workspace. While it exists, session start puts recent memory in front of every session and `/iwe:distill` writes here; delete it and both go silent. Every run re-reads it, so an edit takes effect immediately. Nothing captures memory on its own: `/iwe:distill` reads a session *with you*, proposes, and writes only what you select.

The sections below are read by name: `/iwe:distill` follows "what to capture", "how to write it" and "dedup and updates"; `/iwe:reflect` follows "curation"; "at session start" goes in front of every session verbatim. A missing section is followed as missing.

The frontmatter carries two things, both optional: `distill`, how sessions are read — `max_chunk_size` (25000), `max_proposals` (5), `remind_after_days` (7; `-1` never, `0` every session) — and `injection`, the queries session start lists, each with an optional `heading`, `limit` and `max_tokens`, by default the documents carrying `created`, newest first:

```yaml
distill:
  max_chunk_size: 25000
injection:
  - { heading: "Most recently recorded, newest first — titles and keys only:", filter: { created: { $exists: true } }, sort: created:-1, limit: 20 }
```

`/iwe:reflect` tunes them; an `IWE_<KNOB>` environment variable (`IWE_DISTILL_MAX_PROPOSALS`) moves a default machine-wide.

## What to capture

The few things a future session in this repository would be worse off not knowing: still true after the session ends, not obvious from the code, the README, `CLAUDE.md`, the history or a document already here, and enough to make a future session act differently. Prefer none over noise: most sessions produce nothing durable. The opposite failure is folding: two traps and one settled question are three items, not one.

Nearly always worth keeping:

- a trap the session actually hit — a command failed, the cause was found, something made it work. The error text is its fingerprint.
- a rule stated in the conversation — "never do X", "always Y" — especially when nothing in the repository records it.
- a correction the user made — the assistant did, assumed or proposed something and the user reversed it. Write the mistaken form, in the words that were wrong, then what is right. A correction scoped to a single reply ("shorter", "not now") is not kept unless it recurs.

**A decision is recorded only when the user requested or confirmed it, in their own words.** An assistant recommendation, however well argued, is not a decision, nor is a proposal that went unanswered — drop it, or write it as an open question, never as settled.

Write nothing about the conversation itself. Memory is about the project, not the session.

## How to write it

One document per fact, keyed by a slug of its title at the top level of the workspace (`cache-warmup-order`, not `notes/2026/misc-3`). The `memory` schema in `.iwe/schemas/` binds every key and `--strict` enforces it: a document carrying `created` stamps it `YYYY-MM-DD HH:MM` and keeps `session` a non-empty string; a write that fails is fixed, never retried without the flag. Compose them with `--content -`, the document on stdin via a quoted heredoc:

```
iwe create <slug> --strict --content - <<'EOF'
---
created: "<the occurred stamp of the span this came from>"
session: "<the session id it came from>"
---

# <Title: a specific noun phrase, not a sentence>

<Two to six sentences. State the fact, then why it matters. Quote error
messages verbatim — codes, identifiers, environment variable names — and name
the files and commands involved, so the item is findable by lexical search.>
EOF
```

`created` and `session` are copied from the header `iwe internal claude session read` prints: `created` is the `occurred` stamp — when the fact came about, not when it was written down. For something established in the live conversation, use the current time in the same `YYYY-MM-DD HH:MM` shape and `$CLAUDE_CODE_SESSION_ID`. One field never mixes date formats.

## Dedup and updates

Search before every write, and judge the hits by reading them:

```
iwe find --lexical "<the item's distinctive nouns>" --limit 5 \
  --filter '{ created: { $exists: true } }' \
  --project 'title=$title,key=$key'
iwe retrieve -k <key>
```

The filter selects memory documents: those without `created` are the repository's own markdown, which is never updated. Update an existing document only when the new item covers the *same fact*, keeping its `created` stamp; a related but different fact is a new document.

## Provenance

`session` names the session the fact came from. `iwe find --filter '{ session: "<id>" }' -f keys` is what a session produced; a document's own `session` field is where it came from. The session's own record — what was read, offered and turned down — is `.iwe/claude/sessions/<id>.yaml`, outside the graph.

## Curation

Human-invoked, through `/iwe:reflect`. When asked, it may merge two documents that state the same fact, keeping the older key and deleting the other with `--expect 1`, at most five merges a pass — and nothing else. No pruning by age; reorganizing the store (new fields, hubs, schemas) is a human decision.

## At session start

Nothing here writes to memory on its own. When this session establishes something the policy keeps — a trap hit and fixed, a rule the user stated, a correction the user made — end that turn with a one-line offer: "Worth remembering: <title> — say the word and I'll record it." One offer per item, at most one per turn, nothing written until the user says yes. A yes runs `/iwe:distill` for that item; a no is recorded once with `iwe internal claude session complete --offered 1 --rejected "<title>"` (no `--lines`) and never re-offered.

`/iwe:distill` records something worth keeping, `/iwe:reflect` reorganizes the store.
