# Queries

Run these from the workspace root, as plain `iwe` commands — the repository is
the store.

## Read

- Recent memory: `iwe find --filter '{ distilled_lines: { $exists: false }, $key: { $nin: [MEMORY] } }' --sort 'created:-1' --limit 10 --project 'title=$title,key=$key,created=created'`
- Search memory: `iwe find --lexical "<terms>" --limit 5 --filter '{ distilled_lines: { $exists: false } }'`
- Search everything, session records included: `iwe find --lexical "<terms>" --limit 5`
- One document: `iwe retrieve -k <key>`
- What this store keeps and how: `iwe retrieve -k MEMORY`
- Store shape: `iwe schema`

## Capture state

The sweep keeps no state files: every answer is a query or a sweep command.
Mechanical documents carry no `type` — they are selected by the fields only the
machinery writes, and the machinery reads them by path, so these answers hold
whether or not the store tracks its chunks in git.

- What is still un-captured on disk, and what is queued (transcripts are files,
  not documents, so this one is a command):
  `iwe internal claude hook stop --survey`
- The head of the queue: `iwe internal claude job next`
- How far capture has got: `iwe find --filter '{ distilled_lines: { $exists: true } }' --sort 'created:-1' --limit 20 --project 'session=session,lines=distilled_lines,at=distilled_at'`
  Compare `lines` against `wc -l` on the session's `transcript` to see what is
  still pending.
- What a session produced: `iwe find --filter '{ $includedBy: sessions/<session-id> }' -f keys`
- Which sessions produced a document: `iwe find --filter '{ $includes: <key> }' -f keys`
- What happened around a date: `iwe find --filter '{ created: { $gte: "2026-08-01", $lt: "2026-09-01" } }' --project 'title=$title,key=$key,created=created'`
- Rules the user stated (stores that stamp `origin`): `iwe find --filter '{ origin: user }' --limit 20 --project 'title=$title,key=$key'`
- Capture history: the notes in a session document — `iwe retrieve -k sessions/<session-id>`
