# Memory policy

This document — `MEMORY.md` — is the switch and the policy for durable memory in
this workspace. While it exists, session capture runs here; delete it and every
memory hook goes silent. Everything below is yours to edit — capture re-reads
it on every run, so a change here takes effect immediately.

This store uses the typed ontology: three document types, each with its own
template and schema, installed alongside this policy. The frontmatter above
carries the mechanical knobs — all optional, defaults in the plugin:
`sweep_threshold_lines` (120), `chunk_chars` (10000), `max_chunks_per_sweep`
(30), `max_items_per_chunk` (3), `inflight_ttl_minutes` (60),
`injection_max_tokens` (2000). A knob set here wins; where one is absent, an
`IWE_<KNOB>` environment variable — say `IWE_SWEEP_THRESHOLD_LINES` — moves the
default machine-wide.

## What to capture

The few things a future session in this repository would be worse off not
knowing, as one of three types:

- **decision** — a choice that was made and the reason it was made, where a
  future session would otherwise reopen the question. Architecture, tooling,
  naming, process, scope.
- **learning** — a durable fact about this codebase, its infrastructure, or the
  way the team works, that was not obvious from reading the code.
- **gotcha** — a trap that cost time, with the symptom that identifies it and
  the way around it.

Keep an item only when all of these hold:

- It stays true after the session that produced it ends. Anything scoped to the
  task at hand is not memory.
- It is not already obvious from the code, the README, `CLAUDE.md`, the git
  history, or a document already in this store.
- A future session would act differently for knowing it.

Prefer none over noise. An empty extraction is a correct, common outcome. The
opposite failure is folding: a session that hits two different traps and settles
one question holds three items, not one.

Two patterns are nearly always worth keeping: a trap the session actually hit
(that is a gotcha, and the error text is its fingerprint), and a rule stated in
the conversation — "never do X", "always Y" — which is a decision even when
nothing in the repository records it.

Write nothing about the mechanics of the conversation itself. Memory is about
the project, not the session.

## How to write it

Through the type's template, which stamps `type`, takes `created` from the
chunk, and derives the key from the title's slug
(`learnings/cache-warmup-order`):

```
iwe create --template <decision|learning|gotcha> --strict \
  --var title="<a specific noun phrase, not a sentence>" \
  --var body="<two to six sentences: the fact, then why it matters, quoting
error messages verbatim and naming the files and commands involved>" \
  --var session="<the session id of the capture chunk you are working>" \
  --var created="<the chunk's occurred stamp, or its created stamp when the
header shows no occurred line>" \
  --var origin="<user or claude>"
```

`--strict` validates against the type's schema before anything is written; fix
what it rejects rather than dropping the flag, and never pass `--set` alongside
a template. A colliding title suffixes its key rather than overwriting.

Then put each document on the day's hub, which is free to re-run:

```
iwe attach -k <key> --to daily --quiet
```

## Dedup and updates

Search before every write, scoped to the type, and judge the hits by reading
them rather than by rank:

```
iwe find --lexical "<the item's distinctive nouns>" --filter '{ type: <type> }' \
  --limit 5 --project 'title=$title,key=$key'
iwe retrieve -k <key>
```

Update an existing document only when the new item covers the *same fact*. A
related but different fact is a new document.

## Provenance

The templates carry `created`, `session` and `origin` fields. `created` and
`session` come verbatim from the capture chunk header. One date is enough, and
it is the one that says when the fact came about: `created` takes the chunk's
`occurred` stamp — when the conversation happened, not when it was captured —
falling back to the chunk's `created` stamp when the header shows no
`occurred` line. A hand capture with no chunk behind it passes the current
time in that same `YYYY-MM-DD HH:MM` shape rather than leaving the variable
unset: the template's fallback renders the workspace's display date format,
and one field must never mix date formats or a range query over it stops
working.
`origin` is a judgment: `user` when the user stated the fact — a rule, a
correction, an explicit "remember this" — and `claude` when it was inferred
from the work. The capture note's inclusion links in the session document are
the other half, and the one that survives a schema change: `iwe find --filter
'{ $includedBy: sessions/<id> }' -f keys` is what a session produced.

## Curation

Curation is human-invoked, through `/iwe:reflect`. When asked, it may:

- merge two documents that state the same fact, keeping the older key as the
  survivor and deleting the other with `--expect 1`; at most five merges a pass;
- promote a subject three or more documents circle into a topic document
  (`iwe create --template topic --strict --if-exists skip`), whose body links to
  each member on its own line; at most two promotions a pass;
- prune by age: gotchas older than 180 days and learnings older than 365 days,
  never decisions, never a document a topic includes
  (`$includedBy: { match: { type: { $ne: daily } }, $size: 0 }`), at most 20
  deletions a pass, each with `--expect 1`.
