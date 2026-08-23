# Memory policy

This document — `MEMORY.md` — is the switch and the policy for durable memory in
this workspace. While it exists, session capture runs here; delete it and every
memory hook goes silent. Everything below is yours to edit — capture re-reads
it on every run, so a change here takes effect immediately.

The frontmatter of this document carries the mechanical knobs. Every one of
them is optional and has a default: `sweep_threshold_lines` (120),
`chunk_chars` (10000), `max_chunks_per_sweep` (30), `max_items_per_chunk` (3),
`inflight_ttl_minutes` (60), `injection_max_tokens` (2000). A knob set here
wins; where one is absent, an `IWE_<KNOB>` environment variable — say
`IWE_SWEEP_THRESHOLD_LINES` — moves the default machine-wide.

## What to capture

The few things a future session in this repository would be worse off not
knowing. Keep an item only when all of these hold:

- It stays true after the session that produced it ends. Anything scoped to the
  task at hand is not memory.
- It is not already obvious from the code, the README, `CLAUDE.md`, the git
  history, or a document already in this store.
- A future session would act differently for knowing it.

Prefer none over noise. An empty extraction is a correct, common outcome: most
sessions produce nothing durable. Never invent an item to fill a quota.

The opposite failure is folding: a session that hits two different traps and
settles one question holds three items, not one. Each distinct failure mode
keeps its own item with its own symptom.

Two patterns are nearly always worth keeping:

- a trap the session actually hit — a command failed, the cause was found, and
  something made it work. The error text is its fingerprint.
- a rule stated in the conversation: "never do X", "always Y", "this stays Z" —
  especially when nothing in the repository records it.

Write nothing about the mechanics of the conversation itself, the tools that
were used, or how the session went. Memory is about the project, not the
session.

## How to write it

One document per fact, keyed by a slug of its title at the top level of the
workspace (`cache-warmup-order`, not `notes/2026/misc-3`). A specific title is
therefore also a readable filename and a stable handle for deduplication.

No template and no schema binds these documents. Compose them with
`--content -`, the document on stdin via a quoted heredoc — never inline in a
quoted argument, where a multi-line body trips the harness's shell-safety
prompt:

```
iwe create <slug> --content - <<'EOF'
---
created: "<the chunk's occurred stamp>"
session: "<the chunk's session id>"
origin: <user or claude>
---

# <Title: a specific noun phrase, not a sentence>

<Two to six sentences. State the fact, then why it matters. Quote error
messages verbatim — codes, ticket identifiers, environment variable names
included — and name the files and commands involved, so the item is findable
by lexical search later.>
EOF
```

`created` and `session` are copied verbatim from the capture chunk header —
never invented, never reformatted. One date is enough, and it is the one that
says when the fact came about: `created` takes the chunk's `occurred` stamp,
which is when the conversation happened, falling back to the chunk's `created`
stamp when the header shows no `occurred` line. That is what keeps a
backfilled transcript's dates honest, and it is what session-start reads to
surface recent memory. When capturing by hand rather than from a chunk, use
the current time in the same `YYYY-MM-DD HH:MM` shape — one field must never
mix date formats, or a range query over it stops working — and leave `session`
out unless the session id is known.

## Dedup and updates

Search before every write, and judge the hits by reading them rather than by
rank:

```
iwe find --lexical "<the item's distinctive nouns>" --limit 5 \
  --filter '{ created: { $exists: true }, distilled_lines: { $exists: false } }' \
  --project 'title=$title,key=$key'
iwe retrieve -k <key>
```

That filter is what makes the search mean "memory documents". Documents carrying
`distilled_lines` are the capture machinery's own session records, whose capture
notes name every document a session produced and so match many searches. The raw
capture chunks never enter the store: they wait outside the graph, under
`.iwe/claude-sessions/` at the workspace root (or `$IWE_MEMORY_STATE`), until a
completion retires them. Documents with no `created` stamp are this repository's
own markdown — README, docs, runbooks — which memory records facts *about* and
never rewrites. Never update either kind.

Update an existing document only when the new item covers the *same fact* — a
sharper version, a correction, a second occurrence. A related but different
fact is a new document. When updating, keep the existing `created` stamp.

## Provenance

`origin` records who asserted the fact: `user` when the user stated it — a
rule, a correction, an explicit "remember this" — and `claude` when it was
inferred from the work. `session` names the session record (`sessions/<id>`)
the fact came from. The other half of provenance is the graph: the capture
note in the session document holds an inclusion link to every document the
session produced, so `iwe find --filter '{ $includedBy: sessions/<id> }' -f
keys` answers "what did that session produce" and `iwe find --filter '{
$includes: <key> }' -f keys` answers "which sessions produced this".

## Curation

Curation is human-invoked, through `/iwe:reflect`. When asked, it may:

- merge two documents that state the same fact, keeping the older key as the
  survivor and deleting the other with `--expect 1`; at most five merges a
  pass;
- do nothing else. This store does not prune knowledge documents by age, and
  reorganizing it — new frontmatter fields, topic hubs, schemas — is a decision
  a human makes, never a background pass.
