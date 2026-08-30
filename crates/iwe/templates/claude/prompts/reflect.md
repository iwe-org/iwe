# Reflect

The interactive layer of memory maintenance, and the only one: policy and taxonomy evolution, curation as far as the policy's curation section allows, and the forget verbs on demand — all with the user. Every command is a plain `iwe` command run from the repository root. No `MEMORY.md` document means nothing to govern: say so and point at `/iwe:init`. Never commit: every change, deletions included, is a file the user reviews.

## The MEMORY.md document is the policy

```bash
iwe internal claude session brief
```

Prints the policy body, a policy check, the frontmatter the store's documents carry, the schemas that bind them (`=== schemas ===`), the area hubs with what each includes and the documents outside every area (`=== hubs ===`), what session start lists, and the recent rejections. A failing check is the first thing to fix, with the user; the schemas and hubs sections are the census every pass ends with, so nothing here is counted by hand.

The body is a contract of level-two sections, read by name:

- `## What to capture` — what a distill run proposes.
- `## How to write it` — the whole shape of a document: keys, frontmatter, body, templates, schemas, hubs, pages. What it does not say is not done.
- `## Dedup and updates` — the search before every write.
- `## Provenance` — which fields say where a fact came from.
- `## Curation` — what this skill may do to the store. Absent, it may only edit the policy.
- `## At session start` — put in front of every session verbatim.

The first three are required. Nothing caches the policy, so the next run follows the edited copy:

```bash
iwe update -k MEMORY --content - <<'EOF'
<edited body>
EOF
iwe update -k MEMORY --set distill.max_proposals=3 --expect 1
```

The frontmatter knobs, with defaults: `distill.max_chunk_size` (25000) — how much conversation one `session read` serves; `distill.max_proposals` (5); `distill.remind_after_days` (7) — how long before session start reminds about the backlog again, `0` every session, `-1` never; and `injection` — what session start puts in front of every session, as a list of slices the binary runs in order, each a `filter` and/or a `sort: <field>:-1` with an optional `heading`, `limit` and `max_tokens`. A document listed by one slice is not repeated by a later one, and a slice with neither `limit` nor `max_tokens` lists everything it matches. A store with `kind` and `status` fields usually wants the rules and the open traps first:

```yaml
distill:
  max_proposals: 3
injection:
  - { heading: "Rules this store keeps:", filter: { kind: rule }, limit: 10, max_tokens: 400 }
  - { heading: "Still open:", filter: { kind: trap, status: open }, limit: 10 }
  - { heading: "Most recently recorded:", filter: { created: { $exists: true } }, sort: created:-1, limit: 10 }
```

`--set` takes YAML, so a store whose notes carry a `type` lists them once:

```bash
iwe update -k MEMORY --set 'injection=[{ filter: { type: { $in: [decision, learning, gotcha] } }, sort: created:-1, limit: 10 }]' --expect 1
```

Each knob has an environment twin named for its path with dots as underscores (`IWE_DISTILL_MAX_PROPOSALS`). `chunk_chars` (now `distill.max_chunk_size`), `max_proposals_per_read` (`distill.max_proposals`), `remind_every_days` (`distill.remind_after_days`) and `injection_max_tokens` (now the slice's own `max_tokens`) are the flat knobs the binary stopped reading; `sweep_threshold_lines`, `max_chunks_per_sweep`, `max_items_per_chunk` and `inflight_ttl_minutes` are dead knobs from before capture became manual. `--unset` clears one.

Requests users make without knowing the vocabulary:

- **"capture less / more"** — "what to capture", and `distill.max_proposals`.
- **"stop asking me about memory"** — `distill.remind_after_days=-1`, and "at session start".
- **"show me the rules / what is still broken when I start"** — an `injection` slice per question.
- **"stop reorganizing my notes"** — delete the curation section.
- **"turn it off"** — `iwe delete MEMORY --expect 1`; nothing else in the store changes.

## Analyze

Widen the brief's listing when ten is not enough, and read `iwe schema` — coverage, types, distinct counts, value distributions — before proposing anything:

```bash
iwe find --sort 'created:-1' --limit 40 --project 'title=$title,key=$key'
```

Then mine the dimensions the frontmatter does not carry:

```bash
iwe find --matches '(?i)\b(postgres|redis|ci|docker|auth)\b' --limit 40 \
  --project 'title=$title,key=$key'
```

Recurring components, tools, people and error classes in bodies but not in frontmatter are the candidates: an index field when the value needs no description and has no parts (an enum, a severity); an entity page (§Reorganize) when the thing has an identity, changes over time, or has parts.

## Propose

A short list: field name, the values observed with counts, and the query each unlocks. Then stop and let the user pick. A field nobody chose is noise in every future `iwe find` and costs a backfill to remove.

## Apply

For each field the user picked, in order:

1. **Extend the schema**, if one binds — add the property to `.iwe/schemas/<name>.yaml` as optional. If the brief's schemas section says nothing binds, write one first: the frontmatter fields and enums "how to write it" names, `required` for the provenance fields, and as much body shape as the store keeps (`iwe docs schema`); bind it in `.iwe/config.toml` to the keys the injection selects, run `iwe schema validate`, and fix or exempt what fails with the user. From then on `--strict` refuses what the policy forbids instead of asking the next distill run to remember it.
2. **Backfill**, one mutation per value, preview first. Every bulk `update` and `delete` carries the machinery exclusion `{ $key: { $nin: [MEMORY, queries] } }` so it cannot touch the policy or the cookbook:

   ```bash
   iwe update --filter '{ $and: [ <the machinery exclusion>, { $content: { $text: "postgres" } } ] }' --set component=postgres --dry-run
   iwe update --filter '<the same filter>' --set component=postgres --expect <count from the dry run>
   ```

   `--expect` aborts if the match set moved.
3. **Teach the write path** — describe the field in "how to write it" and add it to the template in `.iwe/config.toml`, or the backfill decays into a one-off.
4. **Refresh the `queries` document**, if the store has one.

## Forget, on request

```bash
iwe find --filter '{ created: { $lt: "2026-01-01" } }' --format keys
iwe delete --filter '{ $and: [ <the machinery exclusion>, { created: { $lt: "2026-01-01" } } ] }' --dry-run
iwe delete --filter '<the same filter>' --expect <count from the dry run>
```

Always dry-run first and name the count. `iwe delete` cleans up inbound references, so `Updated N document(s)` counts referrer cleanups and `Updated 0` is not a no-op; never send a curation command's output to `/dev/null`. To change the policy rather than one document, edit the curation section and show the diff.

## Merge and promote, on request

Only as far as the curation section allows — no section, no merges. Keep the older document as the survivor:

```bash
iwe update -k <survivor-key> --content - <<'EOF'
# <title>

<merged body>
EOF
iwe delete <duplicate-key> --expect 1
```

When the curation section declares a lifecycle field (`superseded_by`), tombstone instead: `iwe update -k <duplicate-key> --set superseded_by=<survivor-key> --expect 1`, and exclude `superseded_by: { $exists: true }` from later passes.

## Reorganize, when the policy allows it

Two reshapings, both curation — `## Curation` has to allow them, and the shape goes into `## How to write it` before capture is expected to follow it. A policy that allows neither gets the offer, not the reorganization; the same holds for any other shape a user asks for — policy edit first, shown as exact wording, backfill second. Both follow this skill's discipline: propose from counts and stop for the user to pick, `--dry-run` every bulk write and guard it with `--expect`, prove the result with a query, and end by editing the policy. The names — areas, types, hub titles — are the user's.

Two link forms carry the graph semantics both rely on, and rendering cannot tell them apart:

- A link alone in its paragraph is an **inclusion**: the target gains `$includedBy`, nests under the source in `iwe tree`, and expands with `--expand-includes`. The same link as a list item or inside a sentence is a **reference**: `$references` on the source, `referencedBy:` on the target, no parentage.
- Links resolve against the source document's directory: `components/x` written in `capture/y.md` resolves to `capture/components/x` and dangles silently. Write cross-directory links root-absolute (`/components/x`); `iwe` normalizes them on write, and `iwe rename` rewrites either form.

### Areas

One level of `<area>/<slug>` directories and a top-level hub keyed `<area>` — a page of inclusion links, one per paragraph, never a bulleted list — so `iwe tree` shows the store's shape and a note has one home. The directory and the hub carry the grouping twice, and the convention that hub `<area>` includes exactly the notes under `<area>/` is what keeps the two checkable. Hubs stay at the top level. Flat is correct at small scale; a hub wrapping two notes is worse than no hub. An area name that doubles as an enum value is picked with its key in view: an area named after the store's own prefix stutters (`memory/memory/<slug>`), and renaming it later is a three-step enum migration.

1. **Propose** from `iwe tree --depth 2` and the listing above: names, member counts, example keys.
2. **Move the members.** `iwe rename` rewrites every inbound link, so provenance survives the move. Never move `MEMORY`.

   ```bash
   iwe rename <slug> <area>/<slug> --dry-run
   iwe rename <slug> <area>/<slug>
   ```

3. **Create the hub** with the final keys. A note already on the `<area>` key is either the hub seed — append into it — or one more member to rename first; `--if-exists fail` surfaces the clash.

   ```bash
   iwe create <area> --if-exists fail --strict --content - <<'EOF'
   # <Area title>

   [<title>](<area>/<slug>)
   EOF
   ```

4. **Prove it**, one query per hub; the count must equal the member count. Zero behind a full-looking hub means the links are list items — rewrite the body with `iwe update -k <area> --content -` and re-run.

   ```bash
   iwe find --filter '{ $includedBy: <area> }' -f keys
   ```

5. **Teach the write path.** `## How to write it` gains the areas, keys as `<area>/<slug>`, hub bodies as one inclusion link per paragraph, and that a note fitting no area stays a flat slug — capture never invents an area; grouping strays is this skill's job on a later pass. `## Curation` gains the proof query.

`iwe internal claude session complete --wrote <area>/<slug>` links a written document into hub `<area>` on its own, once. Adopting a stray is the same append, atomic and safe to repeat — already linked means the filter matches nothing and the command exits 2 with `$append expects 1 block, selected 0`:

```bash
iwe update -k <area> \
  --filter '{ $nor: [ { $includes: <area>/<slug> } ] }' \
  --append '{ $header: "<Area title>", content: "[<title>](<area>/<slug>)", expect: 1 }' \
  --expect 1
```

Nothing validates the target: create the document first, append second. The mirror for removing a member is `--delete '{ $ref: { $references: <area>/<slug> }, expect: 1 }'`. The brief's `=== hubs ===` section is the stray census; in a store whose hubs carry a type it is one query — `{ $includedBy: { match: { type: topic }, $size: 0 } }` over the injection scope. Never build it from `type: { $nin: [topic] }`, which also matches every document with no `type` at all. A document included by several hubs is the multi-parent case inclusion links exist for; the directory names the primary home, and only that hub has to include it.

### Entities

Pages for the things facts are about — a person, a release, a component, a tool — that the facts link to. An area gives a fact its one home; an entity is many-to-many, so entities are never directories facts live in. One level per **entity type** the user names: a directory `<type>/`, a top-level hub keyed `<type>`, and `type: <value>` on every page (`components/<slug>` with `type: component` under a "Components" hub). No generic bucket: a thing without a type worth naming is a noun, not an entity. Three edges: fact → entity is a reference in the fact's own prose or a trailing list item — derived from the body, nothing to drift, and `iwe retrieve -k <key>` prints `referencedBy:` with every fact about it; type hub → entity is an inclusion, as for an area; entity → part is an optional inclusion in the entity's own body, so `iwe tree --depth 3` shows the component map.

1. **Propose** by counting facts per noun — fewer than three is a word, not a page, and a page nothing links to is pure upkeep:

   ```bash
   iwe find --matches '(?i)\bsession record\b' --limit 100 -f keys | wc -l
   ```

2. **Create the type hub** at the top level, then the pages: two to four sentences — what the thing is, where it lives, the one thing to know before reading facts about it — with the fields the policy stamps, `type` as the user named it, no fact taxonomy. Then the guarded hub append above; for a thing with parts, one inclusion link per part in the parent's body, parts created first.
3. **Backfill the links**, one guarded bulk command per entity. The `$nor … $references` guard makes it idempotent, so a clean re-run reports `$append expects 1 block, selected 0`. Read the dry-run keys — a fact the regex catches that is not about the thing gets the link by hand or not at all. `$content: { $text: … }` is the stemmed alternative; `$matches` narrows when the noun is also an ordinary word. `--replace-text` cannot do this in bulk, so backfill writes the trailing list item and capture writes the inline form from then on.

   ```bash
   iwe update --filter '{ $and: [ <the machinery exclusion>, { $content: { $matches: "(?i)\\bsession record" } }, { $nor: [ { $references: components/session-record } ] } ] }' \
     --append '{ content: "- [Session record](/components/session-record)" }' --dry-run
   iwe update --filter '<the same filter>' --append '<the same append>' --expect <count from the dry run>
   ```

4. **Prove it**, every count matching:

   ```bash
   iwe find --filter '{ $includedBy: components }' -f keys                        # = the pages under components/
   iwe find --filter '{ type: component, $referencedBy: { $size: 0 } }' -f keys   # orphans: must be empty
   iwe find --matches '(?i)\bsession record' --filter '{ $nor: [ { $references: components/session-record } ] }' -f keys   # strays: read each
   ```

5. **Teach the write path.** `## How to write it` gains the types — directory, `type` value, hub title — and the rule capture follows: list the pages before composing (`iwe find --filter '{ type: { $in: [component, tool] } }' --project 'key=$key,title=$title'`), link every one the item mentions inline by root-absolute key, and never mint one — naming a noun is this skill's job once three facts share it. `## Curation` gains the three proofs.

What it unlocks: `{ $references: components/session-record }` alone or beside `kind` and `status`; every fact about any part of a thing, however deep the parts nest — `{ $references: { match: { $includedBy: { match: { $key: components/session-start-hook }, maxDepth: 0 } } } }`; and `iwe retrieve -k <key>` as the dossier, nothing maintained by hand. Connections between facts use the same edge: when a merge or a status flip rewrites a body, name the other document inline so `$referencedBy` on a trap answers what resolved it. Typed relations the policy declares (`superseded_by`) stay frontmatter.

## Verify

```bash
iwe schema validate     # only if this store binds schemas
iwe schema
iwe tree --depth 2      # if the store groups into areas or has entity pages
iwe internal claude session brief
```

Confirm coverage moved as expected, the policy check is `ok`, the schemas section binds every document the injection selects, the hubs section shows every area complete and names no stray the user did not choose to leave, and the brief's listing shows what the user calls memory and nothing else. End with a summary of what changed: documents touched, fields added, keys moved, hubs and pages created, documents deleted, policy changes. No commit.
