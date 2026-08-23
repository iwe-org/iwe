# Distill

Write one or more durable items into this repository's IWE workspace. You
already hold the context; this skill is the write path, not a research task —
the interactive twin of the automatic capture that runs at turn boundaries.

Installed as part of the `iwe` plugin this skill is `/iwe:distill`; installed
with the skills CLI it is plain `/distill`.

Every command here is a plain `iwe` command run from the repository root — the
workspace *is* the store, so there is no `-C` and no separate memory directory.
The only permission any of this needs is `Bash(iwe:*)`.

## 0. Check that memory is on here, and read the store

```bash
iwe internal claude job brief
```

One command answers both questions. It prints this store's `MEMORY.md`
policy — the switch, and the rules — then the frontmatter its own documents
actually carry and its ten most recent keys and titles, with the capture
machinery's own documents left out of both.

A non-zero exit means the `MEMORY.md` document is missing: this workspace does
not have memory enabled, or the directory is not a workspace at all. Do not
improvise one and do not scaffold anything by hand — the `init` skill turns
memory on in one step (`/iwe:init`), and it decides *with the user* what shape
this store's documents take.

## 1. Choose what to write

The policy's "what to capture" section is the bar. Hold user-invoked items to
it too — typically:

- it stays true after this session ends;
- it is not already obvious from the code, the README, `CLAUDE.md`, or the git
  history;
- a future session would act differently for knowing it.

When the user names the thing to remember, write that thing. When they say
"remember this" about the session at large, pick at most three items and name
them back.

Give each item a title that is a specific noun phrase rather than a sentence,
and a body of two to six sentences that states the fact, then why it matters,
naming the files, commands, error strings, and identifiers involved so lexical
search finds it later.

## 2. Write it the way this store writes

The policy's "how to write it" section is authoritative: it names the
templates to use, or the frontmatter shape to compose, the key convention, and
whether schemas bind. §0's brief already showed you what is actually here — the
fields with their coverage, the key convention, the recent titles. Read one of
those documents in full before writing the first one, because a schema table
does not show body length, heading style or how links are used:

```bash
iwe retrieve -k <one of the keys the brief listed>
```

The brief leaves the capture machinery's own documents out of both its schema
and its sample — session records carry `distilled_lines`, raw capture chunks
carry `covers_lines` — because they are never examples to copy and never
documents to write to.

Then either compose the document:

```bash
iwe create <key> --content - <<'EOF'
---
created: "<today, as YYYY-MM-DD HH:MM>"
origin: user
---

# <Title>

<body>
EOF
```

or use the store's template when the policy names one:

```bash
iwe create --template <name> --strict --var title="<title>" --var body="<body>"
```

Stamp only the fields the policy names, under the store's own names.
`origin: user` is the common case here — a hand capture is the user asking to
remember — and `created` is now, because a hand capture records the fact as it
comes about. Write it in the shape the store's other documents use, so one
field never mixes date formats. When the policy names a template, pass the
date explicitly rather than letting the template fall back to its own clock. `--strict` validates against the bound schema before anything is
written; when it fails, fix the item rather than dropping the flag, and never
pass `--set` alongside a template. Never hand-write a file into the workspace — the CLI is
the write path. The body always goes on stdin (`--content -` with a quoted
heredoc): inlining a multi-line document in a quoted argument trips the
harness's shell-safety prompt on every write.

## 3. Dedup before writing

```bash
iwe find --lexical "<the item's distinctive nouns>" --limit 5 \
  --filter '{ distilled_lines: { $exists: false }, covers_lines: { $exists: false } }' \
  --project 'title=$title,key=$key'
```

Keep that filter on the search: a raw capture chunk holds a whole span of
conversation, so it matches almost any query and would otherwise rank first.

Read the plausible hits with `iwe retrieve -k <key>`. Update an existing document
only when it covers the *same fact*; a related but different fact is a new
document. Never route by search rank alone.

```bash
iwe update -k <existing-key> --content - <<'EOF'
# <title>

<merged body>
EOF
```

Keep the existing `created` stamp when you update.

## 4. Anything else the policy asks for

Some stores attach new documents to a daily hub (`iwe attach -k <key> --to daily
--quiet`), some carry a `session` provenance field, some do neither. Do what the
policy says and nothing more.

When the policy declares entity types — pages for the people, releases,
components, tools or other things the store's facts are about — list them
first (`iwe find --filter '{ type: { $in: [...] } }' --project
'key=$key,title=$title'`) and link every one the item mentions inline in the
body, by root-absolute key (`[watermark](/components/watermark)`; a relative
key resolves against the document's own directory and dangles). Never mint an
entity page — `/iwe:reflect` names them — and never leave an entity link alone
in its own paragraph, which makes it an inclusion and the fact the entity's
parent.

## 5. Report

Name each key you wrote and whether it was new or an update — one line each.
Nothing commits: the new documents are working-tree changes the user reviews as
a normal diff.

## Drain past sessions

Processing sessions that already happened is the `init` skill's job — it claims
the pending tails in waves and curates between them. When the user asks for it
here, hand off rather than reimplementing.

## Related

- `/iwe:init` switches memory on for a workspace and drains the sessions that
  already happened into it.
- The background `distill` agent writes through this same policy at
  turn boundaries; you are its interactive twin.
- `/iwe:reflect` evolves the policy and the store's structure with the user.
