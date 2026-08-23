# Reflect

The interactive layer of memory maintenance — and the only one: nothing curates
this store in the background. Everything here needs a human: **policy and
taxonomy evolution**, curation as far as the policy's curation section directs,
and the forget verbs on demand.

Invoke as `/iwe:reflect` from the plugin, `/reflect` when installed with the
skills CLI. Every command is a plain `iwe` command run from the repository root
— the workspace is the store. If there is no `MEMORY.md` document there is
nothing here to govern: say so and point at the `init` skill.

**Never commit.** Every change here is a working-tree edit the user reviews as a
diff; deletions included. Do not run `git add` or `git commit`.

## The MEMORY.md document is the policy

```bash
iwe retrieve -k MEMORY
iwe find --filter '{ $key: MEMORY }' --project 'fm=$frontmatter' -f json
```

Its body governs capture and curation: what to capture, how to write it,
dedup, provenance, and curation. Its frontmatter carries the sweep's knobs.
Changing behaviour means editing this document — nothing caches it, so the next
pass follows the new copy:

```bash
iwe update -k MEMORY --content - <<'EOF'
<edited body>
EOF
iwe update -k MEMORY --set sweep_threshold_lines=60 --set max_items_per_chunk=2 --expect 1
```

The knobs, with their defaults: `sweep_threshold_lines` (120) — how much a
transcript must grow before it is worth a pass; `chunk_chars` (10000) — how much
conversation one capture pass reads; `max_chunks_per_sweep` (30);
`max_items_per_chunk` (3); `inflight_ttl_minutes` (60) — how long a dead
capture's claim on its chunks survives; `injection_max_tokens` (2000) — what
session start puts in front of every session.

`chunk_chars` and `max_items_per_chunk` are read at import time and stamped
into each chunk, so a change to either governs the next sweep, never the chunks
already queued. `/iwe:init` raises both for a backfill and restores them
afterwards; if a store is sitting on `25000`/`7`, that is a drain that never
put them back, and the live-capture values are `10000`/`3`.

Two edits worth offering explicitly, because they are the ones users ask for
without knowing the vocabulary:

- **"capture less / capture more"** — the "what to capture" section, and
  `max_items_per_chunk`.
- **"stop reorganizing my notes"** — delete the policy's curation section, and
  this skill refuses to merge, promote, prune or regroup.

Turning memory off entirely is `iwe delete MEMORY --expect 1`: every hook goes
silent, and nothing else in the store changes.

## Analyze

```bash
iwe schema
iwe find --filter '{ distilled_lines: { $exists: false }, covers_lines: { $exists: false }, $key: { $nin: [MEMORY] } }' \
  --sort 'created:-1' --limit 40 --project 'title=$title,key=$key,created=created'
```

`iwe schema` infers the store's real frontmatter: per-field coverage, type
distribution, distinct counts, and value distributions for low-cardinality
fields. Read it before proposing anything. Then mine the dimensions the
frontmatter does not carry yet:

```bash
iwe find --matches '(?i)\b(postgres|redis|ci|docker|auth)\b' --limit 40 \
  --filter '{ distilled_lines: { $exists: false }, covers_lines: { $exists: false } }' \
  --project 'title=$title,key=$key'
```

Recurring components, tools, people, and error classes that show up in bodies
but not in frontmatter are the candidates — for an index field when the value
needs no description and has no parts (an enum, a severity), for an entity
page (§Extract entities) when the thing has an identity worth describing,
changes over time, or has parts. A user who wants both gets both.

## Propose

Present candidates to the user as a short list: the field name, the values you
actually observed with their counts, and the query each would unlock. Then stop
and let them pick. Reflect never invents taxonomy unilaterally: a field the user
did not choose is noise in every future `iwe find`, and it costs a backfill to
remove.

## Apply

For each field the user picked, in this order:

1. **Extend the schema**, if this store binds one — edit `.iwe/schemas/<name>.yaml`,
   adding the property as optional so existing documents keep validating while
   the backfill runs.
2. **Backfill**, one atomic mutation per value, preview first:

   ```bash
   iwe update --filter '{ $content: { $text: "postgres" } }' --set component=postgres --dry-run
   iwe update --filter '{ $content: { $text: "postgres" } }' --set component=postgres --expect <count from the dry run>
   ```

   The `--expect` guard is what makes this safe: it aborts if the match set moved
   between the preview and the write. Exclude the machinery from filter-wide
   updates with `distilled_lines: { $exists: false }, covers_lines: { $exists: false }`.
3. **Teach the write path.** Describe the new field in the policy's "how to
   write it" section, and add it to the template in `.iwe/config.toml` if the
   store uses templates. This is what closes the loop — capture re-reads the
   policy on its next run, so the field starts populating itself instead of
   decaying into a one-time backfill.
4. **Refresh the `queries` document**, if the store has one, with the
   invocations the new field unlocks. Session start points every future session
   at it.

## Forget, on request

```bash
iwe find --filter '{ created: { $lt: "2026-01-01" }, distilled_lines: { $exists: false }, covers_lines: { $exists: false } }' --format keys
iwe delete --filter '{ created: { $lt: "2026-01-01" }, distilled_lines: { $exists: false }, covers_lines: { $exists: false } }' --dry-run
iwe delete --filter '<the same filter>' --expect <count from the dry run>
```

`iwe delete` cleans up inbound references — inclusion links go, inline links
become plain text — so the capture notes in session documents do not rot. Always
dry-run first and always name the count. Read the output too, not just the
exit: the `Updated N document(s)` line counts referrer cleanups, so deleting
an unreferenced document prints `Updated 0 document(s)` and reads like a
no-op. Never redirect a curation command's output to `/dev/null` — a failure
you intended to assume away is exactly the one that leaves stray documents
behind. To change the policy rather than one
document, edit the curation section of `MEMORY.md` and show the user the diff.

## Merge and promote, on request

Only as far as the policy's curation section allows — no curation section, no
merges:

```bash
iwe update -k <survivor-key> --content - <<'EOF'
# <title>

<merged body>
EOF
iwe delete <duplicate-key> --expect 1
```

Keep the older document as the survivor so its `created` stamp and its place in
the timeline hold.

When the policy's curation section declares a lifecycle field
(`superseded_by`), tombstone instead of deleting: stamp the losing document
with `iwe update -k <duplicate-key> --set superseded_by=<survivor-key>
--expect 1`, and exclude `superseded_by: { $exists: true }` from later
passes. No declaration means the delete above.

## Organize into areas, on request

Grouping is curation, so the same gate applies: no curation section, no
reorganizing. The shape this skill builds is one level of subdirectories — a
note lives in exactly one area, keyed `<area>/<slug>` — and each area has a
**hub**: a top-level document keyed `<area>`, a plain page of inclusion links:

```markdown
# Postgres

[Connection pooling](postgres/connection-pooling)

[Vacuum tuning](postgres/vacuum-tuning)
```

Two rules look like style but are graph semantics:

- **One link per paragraph, never a bulleted list.** A standalone link
  paragraph is an inclusion link — the member gains `$includedBy`, nests under
  the hub in `iwe tree`, and expands with `--expand-includes`. The same link
  as a list item is a plain reference, and the member floats to the top of the
  tree as if the hub did not exist.
- **Hubs stay at the top level.** Links resolve relative to the containing
  document's directory, so a hub moved into a subdirectory points its
  root-style keys at the wrong documents.

The directory and the hub carry the same grouping twice — the directory for
humans, git and key uniqueness; the hub for the graph — and the convention
that hub `<area>` includes exactly the notes under `<area>/` is what keeps the
two aligned and checkable.

Start from the current shape — `iwe tree --depth 2` and the §Analyze listing —
then propose groups exactly as §Propose handles fields: names, observed member
counts, example keys, and stop for the user to pick. Flat is correct at small
scale; a hub wrapping two notes is worse than no hub. When an area name will
double as a frontmatter enum value, pick both at once and check the key it
produces for stutter: an area named after the store's own prefix yields
`memory/memory/<slug>`, and renaming it later is a three-step enum migration
(widen, rewrite every member, narrow) rather than one edit.

For each area the user picked:

1. **Move the members.** `iwe rename` rewrites every inbound link — capture
   notes included — so provenance survives the move. Never move `sessions/…`
   or `MEMORY`; the machinery owns those keys.

   ```bash
   iwe rename <slug> <area>/<slug> --dry-run
   iwe rename <slug> <area>/<slug>
   ```

2. **Create the hub** with the final keys, one inclusion link per paragraph. A
   note already sitting on the `<area>` key is either the natural hub seed —
   append into it — or one more member to rename first; `--if-exists fail`
   surfaces the clash instead of overwriting.

   ```bash
   iwe create <area> --if-exists fail --content - <<'EOF'
   # <Area title>

   [<title>](<area>/<slug>)
   EOF
   ```

3. **Prove the hierarchy exists.** Rendering cannot tell an inclusion link
   from a reference, so never trust the look of the page — a store of hubs
   that establish no parentage passes every visual inspection. One query per
   hub, and the count must equal the member count:

   ```bash
   iwe find --filter '{ $includedBy: <area> }' -f keys
   ```

   Zero members behind a full-looking hub means the links are list items:
   find such reference-style hubs wherever members float to the top of `iwe
   tree` despite a hub naming them, convert the body to one link per
   paragraph with `iwe update -k <area> --content -`, and re-run the proof.

4. **Teach the write path.** Add the convention to the policy's "how to write
   it" section: the list of areas, keys as `<area>/<slug>`, hub bodies as one
   inclusion link per paragraph, and that capture appends each document it
   creates to its area's hub with the guarded form below. A note that fits no
   existing area stays a flat slug at the top level — capture never invents an
   area; grouping the strays is this skill's job on a later pass.

The append — the one the policy teaches capture, and the one this skill uses
to adopt strays — is safe to repeat because the membership check and the write
are one atomic command:

```bash
iwe update -k <area> \
  --filter '{ $nor: [ { $includes: <area>/<slug> } ] }' \
  --append '{ $header: "<Area title>", content: "[<title>](<area>/<slug>)", expect: 1 }' \
  --expect 1
```

Already linked means the filter matches nothing and the command exits 2 with
`$append expects 1 block, selected 0` — a refusal to write twice, not an
error. The command does not validate the target, and a typo becomes a dangling
link the hub then carries: create the document first, append second, and
append only keys that exist. The mirror, for removing a member, is `--delete
'{ $ref: { $references: <area>/<slug> }, expect: 1 }'`.

Strays — notes no hub includes — are the recurring maintenance question. In a
store whose hubs carry a type, the census needs no enumeration:

```bash
iwe find --filter '{ type: { $in: [<knowledge types>] }, $includedBy: { match: { type: topic }, $size: 0 } }' -f keys
```

Untyped hubs anchor on the list the user just confirmed instead:

```bash
iwe find --filter '{ $nor: [ { $includedBy: { match: { $key: { $in: [<hub>, <hub>] } } } } ], distilled_lines: { $exists: false }, covers_lines: { $exists: false }, $key: { $nin: [MEMORY, <hub>, <hub>] } }' -f keys
```

Never build the census from `$nin` on the type field: `type: { $nin: [topic]
}` also matches every document with no `type` at all — the machinery's
records and the repository's own markdown — and reports a store-sized orphan
count.

Propose a home for each stray — an existing area, a new one, or leave it flat
— and apply with the same rename-then-append steps. The alignment check in
the other direction is the hierarchy proof above: a member key outside the
hub's own directory is drift only when it claims this hub as its home — a
document may be included by several hubs, and a secondary listing from
another area is the multi-parent case inclusion links exist for. The
directory names the one primary home; enforce that the directory's hub
includes it, and leave cross-area listings alone.

## Extract entities, on request

The nouns the census keeps surfacing — a person, a release, an application, a
competitor, a component, a tool — are entities: the things the facts are
*about*, and the second axis the graph can carry. An area gives a fact its one
home; an entity is many-to-many — a fact about the watermark is also about
chunks and the sweep — so entities are never directories facts live in. They
are pages of their own, and facts point at them.

The shape mirrors areas, one level per **entity type** the user names: a
directory `<type>/`, a top-level hub keyed `<type>`, and `type: <value>` on
every page — `people/<slug>` with `type: person` under a "People" hub,
`releases/<slug>` with `type: release`, `components/<slug>` with `type:
component`. There is no generic bucket: a thing without a type worth naming is
a noun, not an entity.

Three edges, and only the first is new:

- **fact → entity is a reference.** The link sits in the fact's own prose —
  `the [watermark](/components/watermark) must never advance…` — or in a
  trailing bulleted list. It is derived from the body, so there is nothing to
  append, nothing to drift and nothing to prove: `iwe retrieve -k
  components/watermark` prints `referencedBy:` with every fact about it, and
  the fact keeps its two parents in `iwe tree`.
- **type hub → entity is an inclusion**, one link per paragraph, with the same
  proof as an area hub: `{ $includedBy: components }` must count exactly the
  pages under `components/`.
- **entity → part is an inclusion**, optional: a page whose thing has parts
  includes them, one link per paragraph, in its own body. `iwe tree --depth 3`
  then shows the component map, and the transitive query below walks it.

The link form is the whole semantics, and two traps sit in it:

- A link alone in its paragraph is an inclusion, so a fact that ends with one
  bare entity link has just made itself the entity's parent. Inline in a
  sentence, or `- [Title](key)` as a list item — the form that is wrong for a
  hub is exactly right here.
- Links resolve against the source document's directory: `components/watermark`
  written inside `capture/x.md` resolves to `capture/components/watermark` and
  dangles silently. Write every entity link root-absolute —
  `/components/watermark` — and let `iwe` normalize it on write. `iwe rename`
  rewrites either form.

**Mine and propose.** The §Analyze census names the candidates; count the
facts per noun with the memory filter:

```bash
iwe find --matches '(?i)\bwatermark\b' --limit 100 -f keys \
  --filter '{ created: { $exists: true }, distilled_lines: { $exists: false }, covers_lines: { $exists: false }, $key: { $nin: [MEMORY] } }' | wc -l
```

Present types and members exactly as §Propose handles fields — each type with
its directory, `type` value and hub title, each member with its count — then
stop. Fewer than three facts is a word, not a page, and a page nothing links to
is pure upkeep.

**Apply**, for each type the user picked:

1. **Create the type hub** at the top level with `--if-exists fail`, as in
   §Organize.
2. **Create the pages.** Two to four sentences: what the thing is, where it
   lives (files, commands, versions, people), and the one thing a future
   session should know before reading facts about it. Stamp `created` and
   `origin` the way the policy stamps them, `type` as the user named it, and no
   `kind` — that is fact taxonomy. Then the guarded hub append from §Organize,
   and, for a thing with parts, one inclusion link per part in the parent's
   body; parts are pages too, created first.
3. **Backfill the links.** One guarded bulk command per entity, preview first,
   the same discipline as a field backfill: every fact whose body mentions the
   thing and does not yet reference the page gets one list item appended.

   ```bash
   iwe update --filter '{ created: { $exists: true }, distilled_lines: { $exists: false }, covers_lines: { $exists: false }, $key: { $nin: [MEMORY] }, $content: { $matches: "(?i)\\bwatermark" }, $nor: [ { $references: components/watermark } ] }' \
     --append '{ content: "- [Watermark](/components/watermark)" }' --dry-run
   iwe update --filter '<the same filter>' --append '<the same append>' --expect <count from the dry run>
   ```

   `$content: { $text: "chunk" }` is the stemmed alternative (it matches
   "Chunks"); `$matches` takes a regex when the noun is also an ordinary word
   (`hook`, `schema`) and the match must be narrower. The `$nor … $references`
   guard is what makes the command idempotent, and it is why a clean re-run
   reports `$append expects 1 block, selected 0`: a zero-match filter, not a
   broken selector. Read the dry-run keys before writing — a fact the regex
   catches that is not about the thing gets the link by hand or not at all.
   `--replace-text` cannot do this in bulk (it demands the text occur exactly
   once per block), so the trailing list is what backfill writes, and the
   inline-prose form is what capture writes from then on.
4. **Prove it**, one query per type and per page, and every count must match:

   ```bash
   iwe find --filter '{ $includedBy: components }' -f keys                          # = the pages under components/
   iwe find --filter '{ type: component, $referencedBy: { $size: 0 } }' -f keys     # orphans: must be empty
   iwe find --matches '(?i)\bwatermark' --filter '{ created: { $exists: true }, $nor: [ { $references: components/watermark } ] }' -f keys   # strays: read each
   ```

   The type hubs join the area hubs in every explicit-hub census above, or
   the pages report as strays of the area axis.
5. **Teach the write path.** The policy's "how to write it" gains the entity
   types — directory, `type` value, hub title — and the rule capture follows:
   before composing, list the pages (`iwe find --filter '{ type: { $in:
   [component, tool] } }' --project 'key=$key,title=$title'`), link every one
   the item mentions inline by its root-absolute key, and **never mint one** —
   a noun with no page stays plain text, and naming it is this skill's job once
   three facts share it. Same invariant as areas: capture links, reflect names.
   The curation section gains the three proofs.

What it unlocks:

```bash
iwe find --filter '{ $references: components/watermark }' -f keys
iwe find --filter '{ kind: trap, status: open, $references: components/job-queue }' -f keys
iwe find --filter '{ created: { $exists: true }, $references: { match: { $includedBy: { match: { $key: components/stop-sweep-hook }, maxDepth: 0 } } } }' -f keys
iwe retrieve -k components/watermark
```

The third is the hierarchy paying off: every fact about any part of a thing,
however deep the parts nest. The last is the dossier — description, parts and
`referencedBy` on one page, nothing maintained by hand.

Connections between facts use the same edge. When a merge or a status flip
rewrites a body, name the other document inline — *fixed by the claim-on-serve
heartbeat in [spawn guard leak](/capture/spawn-guard-duplicate-agent-leak)* —
so `$referencedBy` on the trap answers what resolved it. Typed relations the
policy declares (`superseded_by`) stay frontmatter.

## Verify

```bash
iwe schema validate     # only if this store binds schemas
iwe schema
iwe tree --depth 2      # if the store groups into areas
iwe tree --depth 3      # if the store has entity pages with parts
```

Confirm the new field's coverage moved where you expected, then end with a
summary of the working-tree diff: documents touched, fields added, keys
moved, hubs created, entity pages created and facts linked to them, documents
deleted, and what changed in the policy. No commit.
