# Queries

Run these from the workspace root, as plain `iwe` commands — the repository is
the store.

## Read

- Recent memory: `iwe find --filter '{ $key: { $nin: [MEMORY] } }' --sort 'created:-1' --limit 10 --project 'title=$title,key=$key,created=created'`
- Search memory: `iwe find --lexical "<terms>" --limit 5`
- One document: `iwe retrieve -k <key>`
- What this store keeps and how: `iwe retrieve -k MEMORY`
- Store shape: `iwe schema`

## What memory has read, and what it has not

Nothing reaches memory unselected: `/iwe:distill` reads with you and writes
only what you pick, and `.iwe/claude/sessions/<id>.yaml` holds the whole of the
state for one session, staged candidates included — outside the graph, so these
are commands, not queries.

- What is still undistilled on disk (transcripts are files, not documents, so
  this one is a command): `iwe internal claude session list`
  Add `--all` to see the sessions already distilled or adopted.
- The next span of one session, as a distill run would read it:
  `iwe internal claude session read <session-id>`
- How far memory has got: `iwe internal claude session list --all`
- What a session produced: `iwe find --filter '{ session: "<session-id>" }' -f keys`
- Where a document came from: its `session` field — `iwe retrieve -k <key>`
- What was proposed, kept and turned down in a session: `.iwe/claude/sessions/<session-id>.yaml`
- What happened around a date: `iwe find --filter '{ created: { $gte: "2026-08-01", $lt: "2026-09-01" } }' --project 'title=$title,key=$key,created=created'`
