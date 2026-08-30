# Memory policy

`MEMORY.md` is the switch and the policy for durable memory in this workspace. While it exists, session start puts recent memory in front of every session and `/iwe:distill` writes here; delete it and both go silent. Every run re-reads it, so an edit takes effect immediately. Nothing captures memory on its own: `/iwe:distill` reads a session *with you*, proposes, and writes only what you select.

This store uses the typed ontology: three document types, each with a template and a schema installed alongside this policy. The sections below are read by name: `/iwe:distill` follows "what to capture", "how to write it" and "dedup and updates"; `/iwe:reflect` follows "curation"; "at session start" goes in front of every session verbatim.

The frontmatter carries two things, both optional: `distill`, how sessions are read — `max_chunk_size` (25000), `max_proposals` (5), `remind_after_days` (7; `-1` never, `0` every session) — and `injection`, the queries session start lists, each with an optional `heading`, `limit` and `max_tokens`, here the decisions first:

```yaml
distill:
  max_chunk_size: 25000
injection:
  - { heading: "Decisions:", filter: { type: decision }, limit: 10 }
  - { heading: "Most recently recorded:", filter: { created: { $exists: true } }, sort: created:-1, limit: 10 }
```

`/iwe:reflect` tunes them; an `IWE_<KNOB>` environment variable (`IWE_DISTILL_MAX_PROPOSALS`) moves a default machine-wide.

## What to capture

The few things a future session in this repository would be worse off not knowing, as one of three types:

- **decision** — a choice that was made and why, where a future session would otherwise reopen the question. Architecture, tooling, naming, process, scope.
- **learning** — a durable fact about this codebase, its infrastructure or the way the team works, not obvious from the code.
- **gotcha** — a trap that cost time, with the symptom that identifies it and the way around it.

Keep an item only when it stays true after the session ends, is not obvious from the code, the README, `CLAUDE.md`, the history or a document already here, and would make a future session act differently. Prefer none over noise: most sessions produce nothing durable. The opposite failure is folding: two traps and one settled question are three items, not one.

Nearly always worth keeping: a trap the session actually hit — a gotcha, and the error text is its fingerprint; a rule stated in the conversation, "never do X", "always Y" — a decision, even when nothing in the repository records it; and a correction the user made — the assistant did, assumed or proposed something and the user reversed it — a decision whose body opens with the mistaken form, in the words that were wrong, then what is right. A correction scoped to a single reply ("shorter", "not now") is not kept unless it recurs.

**A `decision` is recorded only when the user requested or confirmed it, in their own words.** An assistant recommendation, however well argued, is not a decision, nor is a proposal that went unanswered — drop it, or write it as a `learning` about the open question, never as a settled `decision`.

Write nothing about the conversation itself. Memory is about the project, not the session.

## How to write it

Through the type's template, which stamps `type`, takes `created` from the span the item came from, and derives the key from the title's slug (`learnings/cache-warmup-order`):

```
iwe create --template <decision|learning|gotcha> --strict \
  --var title="<a specific noun phrase, not a sentence>" \
  --var body="<two to six sentences: the fact, then why it matters, quoting
error messages verbatim and naming the files and commands involved>" \
  --var session="<the session id this came from>" \
  --var created="<the occurred stamp of the span, or now for the live
conversation>"
```

`--strict` validates against the type's schema; fix what it rejects rather than dropping the flag, and never pass `--set` alongside a template. A colliding title suffixes its key rather than overwriting.

Then put each document on the day's hub, which is free to re-run:

```
iwe attach -k <key> --to daily --quiet
```

## Dedup and updates

Search before every write, scoped to the type, and judge the hits by reading them:

```
iwe find --lexical "<the item's distinctive nouns>" --filter '{ type: <type> }' \
  --limit 5 --project 'title=$title,key=$key'
iwe retrieve -k <key>
```

Update an existing document only when the new item covers the *same fact*. A related but different fact is a new document.

## Provenance

`created` and `session` come from the header `iwe internal claude session read` prints: `created` is the `occurred` stamp — when the fact came about, not when it was written down. An item from the live conversation passes the current time in the same `YYYY-MM-DD HH:MM` shape and `$CLAUDE_CODE_SESSION_ID` rather than leaving the variables unset: the template's fallback renders the workspace's display date format, and one field never mixes date formats. The `session` field is the other half: `iwe find --filter '{ session: "<id>" }' -f keys` is what a session produced.

## Curation

Human-invoked, through `/iwe:reflect`. When asked, it may:

- merge two documents that state the same fact, keeping the older key as the survivor and deleting the other with `--expect 1`; at most five merges a pass;
- promote a subject three or more documents circle into a topic document (`iwe create --template topic --strict --if-exists skip`) whose body links to each member on its own line; at most two promotions a pass;
- prune by age: gotchas older than 180 days and learnings older than 365 days, never decisions, never a document a topic includes (`$includedBy: { match: { type: { $ne: daily } }, $size: 0 }`), at most 20 deletions a pass, each with `--expect 1`.

## At session start

Nothing here writes to memory on its own. When this session establishes something the policy keeps — a trap hit and fixed, a rule the user stated, a correction the user made — end that turn with a one-line offer: "Worth remembering: <title> — say the word and I'll record it." One offer per item, at most one per turn, nothing written until the user says yes. A yes runs `/iwe:distill` for that item; a no is recorded once with `iwe internal claude session complete --offered 1 --rejected "<title>"` (no `--lines`) and never re-offered.

`/iwe:distill` records something worth keeping, `/iwe:reflect` reorganizes the store.
