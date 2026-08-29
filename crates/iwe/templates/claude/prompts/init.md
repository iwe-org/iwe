# Set up memory

Memory lives in the repository's own IWE workspace — the same graph as everything else, no separate store. Two things switch it on: a workspace marker at the root and a `MEMORY.md` policy document. This skill writes both, in the shape the store already uses, then hands the sessions already on disk to `/iwe:distill`.

Run every command from the repository root. Nothing commits: the whole run is a working-tree diff the user reviews. Never run `git add` or `git commit`.

## 1. Look before you write

```bash
iwe retrieve -k MEMORY
iwe find --filter '{}' -f keys --limit 0
iwe schema
iwe retrieve -k <two or three representative keys>
```

Is memory already on (a `MEMORY.md` that is not a memory policy is a naming clash to surface, not to overwrite)? What is the key convention (`--limit 40` in a large store), which frontmatter do the documents carry, what does a body look like? Then read `.iwe/config.toml` and `CLAUDE.md` with the Read tool for the library path, the templates and their `key_template`s, schema bindings and stated conventions. Prefer these over `ls`, `find` and `cat`: the graph's keys are not the filesystem's paths.

Three outcomes:

- **`MEMORY.md` exists** — memory is on. Skip to §3, or edit the policy with the user.
- **An existing knowledge base** — the policy must describe *that store's* conventions, so capture writes documents indistinguishable from the ones already there.
- **A bare repository or an empty workspace** — the starter policy's flat, slug-keyed, `created`-stamped notes are a good default.

Not a workspace yet? `enable` runs `iwe init`, which writes `.iwe/config.toml` and nothing else. Say plainly that the repository's own markdown becomes part of the graph.

## 2. Switch memory on

Bare repository or empty workspace:

```bash
iwe internal claude enable
```

That also installs `.iwe/schemas/memory.yaml`, bound to every key: a document carrying `created` must stamp it `YYYY-MM-DD HH:MM` and keep `session` a non-empty string, and nothing else is constrained — the shape "how to write it" describes is enforced by `--strict` from the first write, and a reflect session tightens it later.

Existing knowledge base — a store with a shape of its own must not be given the starter's, since a distill run follows the policy literally. Compose the body first, then:

```bash
iwe internal claude enable --body <file>
iwe internal claude enable --body <file> --knobs <yaml-file>
iwe internal claude enable --body <file> --config <ontology.toml> --schema <type.yaml>
```

- Keep the starter's section names ("what to capture", "how to write it", "dedup and updates", "provenance", "curation", "at session start"); the machinery reads them by name.
- "How to write it" states what §1 observed: the key convention, the frontmatter fields, the template names and whether `--strict` applies, the body shape. What it does not say is not done. A store with pages for the things its notes are about (people, releases, components) gets those named too, with the rule that capture links a page by root-absolute key and never creates one.
- Provenance: adopt the starter's `created` and `session` under the store's own names. One date per document — the span's `occurred` stamp, when the fact came about — never a second stamp for the distill run. A store whose documents carry `date:` gets the value there, plus `recency_field: date` in the knobs.
- Carry the decision rule across: a decision is recorded only when the user requested or confirmed it in their own words.
- `--body` is the body only, no frontmatter. `--knobs` is plain YAML for `recency_field` (default `created`), which orders documents, and `knowledge_filter`, which says which documents are memory — what session start lists, what the brief infers a schema from, what every `/iwe:reflect` census walks:

  ```yaml
  recency_field: date
  knowledge_filter:
    type: { $in: [note, decision] }
  ```

- `--config` is appended to `.iwe/config.toml` (refused on a table clash) and each `--schema` file lands in `.iwe/schemas/`. Guard optional provenance fields in a `document_template` with `{% if %}`: an unguarded `{{session}}` renders empty and breaks `$exists: false` queries.
- A store with conventions gets a schema for them, not prose alone: from the `iwe schema` table, write `.iwe/schemas/<name>.yaml` with the fields and enum values the documents already carry, `required` for the provenance fields, and the body shape they share (`iwe docs schema`), then pass it with `--schema` and bind it in `--config` to the keys the knowledge filter selects. Run `iwe schema validate` afterwards and fix the documents that fail with the user — every failure is a convention the store was already breaking.

`enable` exits 2 if `MEMORY.md` exists and writes nothing else. Two options, both off by default: `--queries` also writes a `queries` cookbook document; `--typed` installs the typed ontology (`decision`, `learning`, `gotcha` and `topic` templates and schemas, a daily hub) with its own policy body — offer it, never assume it: right for a repository with no knowledge base whose user wants structure, wrong for a store with conventions.

The machinery writes one document into the library, `MEMORY.md`; its own state lives under `.iwe/claude/`, outside the graph. Keys are relative to the library path, so a store with `path = "docs"` gets `docs/MEMORY.md`; say so before anything runs.

**Then check the policy and run every command it embeds, once** — a broken `iwe find` in a distill run prints nothing and reads as "no duplicates found":

```bash
iwe internal claude session brief
```

The policy check must say `ok`, the recent listing must show what the user calls memory and nothing else, the schema table must show the fields "how to write it" names, and the schemas section must bind them — a store where it says nothing binds has a policy nothing enforces. Run each dedup query as written; exercise each template with and without its optional vars (`iwe create --template <name> --strict --var title=probe`, then `iwe delete -k <key> --expect 1`); where schemas bind, submit one invalid document to confirm `--strict` rejects it. Then show the user the policy — it is theirs to edit, any time, with `iwe update -k MEMORY --content -`.

## 3. Hand over the past

```bash
iwe internal claude session list
```

One row per session, newest first: started, last activity, lines, user turns, distilled line, pending lines, state. Do not hand-roll it with `ls` and `wc`.

Report the numbers and get a scope before spending anything. Propose the scope in user turns, not lines — a session with three hundred pending lines and two turns is an autonomous run that settled nothing; one with forty turns is where the choices were made. The usual recommendation is "read these few, adopt the rest", both numbers named.

```bash
iwe internal claude session adopt
iwe internal claude session adopt <id>
```

Adopting marks pending sessions distilled through their current length without reading a word and drops whatever was staged against them; it refuses the current session and any other live conversation.

This skill does not read sessions. Once the policy is written and the scope agreed, hand over to `/iwe:distill` by name: it reads the scoped sessions into an inbox, puts every candidate to the user, and stopping half-way keeps everything already written and everything still staged.

## 4. Finish

```bash
iwe schema validate     # only if this store binds schemas
iwe internal claude session brief
iwe internal claude session list
```

Then tell the user: what the policy says (editing `MEMORY.md` changes what is captured, deleting it turns memory off); how much backlog is waiting and that `/iwe:distill` reads it with them; that reading a transcript can run unattended but nothing reaches memory unselected, and every document written is one they chose; that everything is uncommitted and reviewed as a diff; that session start shows the undistilled count and, at most weekly, offers to work it; that `/iwe:reflect` reorganizes the store once it has something to group; and to allowlist `Bash(iwe:*)`.

## Migrating from a `memory/` store

Earlier versions kept memory in a separate `memory/` workspace. Move the documents into this workspace and enable memory here:

```bash
iwe internal claude enable --typed   # if the old typed ontology should stay
for directory in learnings decisions gotchas topics daily; do
  [ ! -d "memory/$directory" ] || cp -R "memory/$directory" .
done
iwe schema validate
```

Then `iwe internal claude session adopt` so old transcripts are not re-read. Leave `memory/`, any `sessions/<id>` documents an earlier release kept in the store, and any `.iwe/claude-sessions/` directory for the user to delete. A policy still carrying `sweep_threshold_lines`, `max_chunks_per_sweep`, `inflight_ttl_minutes` or `max_items_per_chunk` predates manual distill; nothing reads them — offer `iwe update -k MEMORY --unset sweep_threshold_lines --expect 1`.

A session marked read that nobody read is reset by setting `distilled_lines: 0` in `.iwe/claude/sessions/<id>.yaml`; the next distill run re-reads it, and writes dedup.
