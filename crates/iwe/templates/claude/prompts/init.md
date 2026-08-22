# Set up memory

Memory lives in the repository's own IWE workspace: the same graph as everything
else, no separate store and no `-C` anywhere. Two things switch it on — a
workspace marker at the root, and a `MEMORY.md` policy document. This skill writes both,
in the shape the store already uses, and then drains the transcripts already on
disk into it.

Invoke as `/iwe:init` from the plugin, `/init` when installed with the skills
CLI. Run every command from the repository root: hooks and agents use the
session's working directory verbatim, so that is where memory happens.

**Nothing here commits.** The entire run lands as working-tree changes the user
reviews as a normal diff.

## 1. Look before you write

Four reads, and between them they answer everything §2 has to write down. Take
them in this order and stop as soon as the picture is clear:

```bash
iwe retrieve -k MEMORY
iwe find --filter '{}' -f keys --limit 0
iwe schema
iwe retrieve -k <two or three keys that look representative>
```

- `retrieve -k MEMORY` — is memory already on? An `MEMORY.md` whose content is not a
  memory policy is a naming clash to surface to the user, not to overwrite.
- `find -f keys` — **the key convention, exactly**: every key in the graph, so
  the directory shape and naming style are in front of you. Add `--limit 40`
  in a large store.
- `iwe schema` — what frontmatter these documents actually carry, with per-field
  coverage, so you can tell a convention from an accident.
- `retrieve` on a couple of real documents — the shape capture has to imitate:
  heading style, body length, how links are used.

Then read `.iwe/config.toml` **with the Read tool** for what no query exposes:
the library path, the templates and their `key_template`s, and which schemas
bind to which key patterns. Read the repository's `CLAUDE.md` the same way if it
has one — it often states conventions the files only imply.

Prefer these over shelling out. `ls`, `find` and `cat` answer worse versions of
the same questions — the graph's keys are not the filesystem's paths — and they
sit outside the `Bash(iwe:*)` allowlist, so each one stops to ask the user for
permission. The Read tool needs no approval at all.

Three outcomes, and they lead to different policy bodies:

- **`MEMORY.md` already exists** — memory is already on. The user is asking to
  drain the backlog or to change the policy; skip to §3, or edit the policy
  with them.
- **An existing knowledge base** (documents, schemas, templates, key
  conventions of its own) — the policy's job is to describe *that store's*
  conventions back to it, so capture writes documents indistinguishable from the
  ones already there.
- **A bare repository or an empty workspace** — the starter policy's flat,
  slug-keyed, `created`-stamped notes are a good default.

If the directory is not an iwe workspace yet, that is fine: the next step
creates one with `iwe init`, which writes `.iwe/config.toml` and nothing else.
Say plainly what that means — the repository's own markdown (README, docs)
becomes part of the graph, which is what lets captured memory link to it.

## 2. Switch memory on

Which policy body gets written follows from §1's outcome, and the rule is
absolute: **enabling memory in a store that already has a shape of its own must
not impose the starter's.** The capture agent follows the policy literally,
so a starter that says "flat slug keys at the top level" is not a default they
will soften — it is the structure they will build.

**A bare repository or an empty workspace** — the starter fits as written:

```bash
iwe internal claude enable
```

**An existing knowledge base** — compose the policy body *first*, before
anything is written. Keep the starter's sections ("what to capture", "how to
write it", "dedup and updates", "provenance", "curation"), but make "how to
write it" describe what §1 actually observed, naming each thing from the read
that showed it: the key convention from `find -f keys`, the frontmatter fields
from `iwe schema`, the template names and `--strict` expectation from
`.iwe/config.toml`, and the body shape from the documents you retrieved. A
store that already keeps pages for the things its notes are about — people,
releases, components, competitors, each under a directory with a `type` —
gets those types named in "how to write it" too, with the rule that capture
links a page it mentions by root-absolute key and never creates one. The
provenance section is derived the same way: the starter's fields (`created`,
`session`, `origin`) are a menu, not a schema — adopt the ones the user wants,
under the names the store already uses. One date per document and no more: it
holds when the fact came about — the capture chunk header's `occurred` stamp —
never a second stamp for when capture ran. A store whose documents already
carry a `date:` gets that value stamped into `date:`, never a parallel field
beside it. Write
the body to a temporary file — body only, no frontmatter — and pass it in:

```bash
iwe internal claude enable --body <file>
```

When the composed policy needs *new* types — templates and schemas this store
does not have yet — compose those too and let the same command install them,
rather than hand-editing `.iwe/config.toml`:

```bash
iwe internal claude enable --body <file> --config <ontology.toml> --schema <type.yaml> --schema <other.yaml>
```

`--config` is appended to `.iwe/config.toml` (refused on a table clash, rolled
back if the result does not parse) and each `--schema` file lands in
`.iwe/schemas/`. In any `document_template` you compose, guard the optional
provenance fields with `{% if %}` — an unguarded `{{session}}` renders an
empty string when the variable is missing, which breaks `$exists: false`
queries; the shipped typed templates are the pattern to copy.

Say where the machinery's documents will actually land, because keys are
relative to the library path: the machinery owns exactly one key prefix,
`sessions/` — the session records at `sessions/<id>`, which carry the
watermarks and capture notes and are the reviewable part. In a store whose
`.iwe/config.toml` sets `path = "docs"`, the policy is `docs/MEMORY.md` and the
records land in `docs/sessions/…` — a new directory inside a documentation tree
the user may be particular about; say so before anything runs.

The raw capture chunks are not documents: they live outside the graph, under
`.iwe/claude-sessions/<id>/<start-line>.md` at the workspace root, where the
machinery reads them by path. Whether that directory enters git is the user's
call and git is the tool for it — `.iwe/claude-sessions/` in `.gitignore`
keeps the digests out of history while `.iwe/config.toml` stays tracked; say
so, and leave the choice to them. Capture works identically either way.

Either way the command runs `iwe init` only when the directory is not a
workspace yet, exits 2 if a `MEMORY.md` is already there, and writes nothing
else — and it is an `iwe` command, so the whole gesture sits inside the
`Bash(iwe:*)` allowlist without a permission prompt. Two more options, both
off by default:

- `--queries` also writes a `queries` cookbook document, which session-start
  points future sessions at.
- `--typed` installs the optional typed ontology — `decision`, `learning`,
  `gotcha` and `topic` templates, their schemas, and a daily hub action — and
  uses the policy body that describes them. **Offer it, never assume it.** It
  is the right choice for a repository with no knowledge base of its own whose
  user wants structure out of the box; it is the wrong choice for a store that
  already has conventions.

The sections capture and `/iwe:reflect` actually read are "what to capture",
"how to write it" (template names, key conventions, whether `--strict`
applies), "dedup and updates", "provenance", and "curation" — an absent
curation section means nothing reorganizes this store. Edit the live policy
any time with `iwe update -k MEMORY --content -`, the edited body on stdin
via a quoted heredoc.
Mechanical knobs live in the policy's frontmatter and all have defaults; set
one only when the user wants a different number, up or down:

```bash
iwe update -k MEMORY --set sweep_threshold_lines=60 --expect 1
```

The sweep threshold defaults to 30 transcript lines. It is a floor against
trivia, not a measure of substance: an efficient model settles real work in far
fewer lines than a verbose one — in the eval that set this default, a session
hit a planted trap, diagnosed it and deployed in 48 lines. Raise it for a store
that only wants long sessions, lower it for one that wants nearly everything.
Per-store tuning belongs in `MEMORY.md`; to move the default across a whole
machine or a CI job, set `IWE_SWEEP_THRESHOLD_LINES` — every knob has an
`IWE_<KNOB>` twin, and neither route needs a new binary.

**Then execute every command the policy embeds, once, before the first wave.**
The policy is a program the capture agent runs verbatim, and a wrong flag does
not fail loudly there — a broken `iwe find` prints nothing, which reads as "no
duplicates found", and the store quietly fills with variants. Run each dedup
query as written; exercise each template with and without the optional vars
(`iwe create --template <name> --strict …`, then delete the probe with
`iwe delete -k <key> --expect 1`); and when schemas bind, submit one
deliberately invalid document to confirm `--strict` actually rejects it.

Show the user the policy you wrote. It is the whole policy, in prose, and
theirs to edit at any time.

## 3. Survey the backlog

Transcripts are files, not documents, so this is the one question the graph
cannot answer. The sweep surveys them for you — same transcript directory,
same watermarks, same threshold as a real sweep, but it imports nothing:

```bash
iwe internal claude hook stop --survey
```

```
transcripts: /Users/you/.claude/projects/-Users-you-code-app
threshold: 30 lines

session                                   lines   captured   pending  chunks  signal  
9c4f1a70-0000-4000-8000-00000000ef01        412        180       232       0      34  pending
5b2d8e10-0000-4000-8000-00000000cd01         24          0        24       0       0  under the threshold

2 transcript(s), 1 pending over 232 uncaptured line(s) carrying 34 user turn(s), 0 pending chunk(s) of which 0 claimed, 0 skipped
```

The `chunks` column is the queue that already exists: chunks imported and not
yet curated, and whether an agent currently holds them. The `signal` column is
the yield estimate: user turns in the pending span, counted the way the digest
renders it, so tool results and meta lines are not turns. Outside a
memory-enabled workspace, or without a transcript directory to read, the
command says so on stderr and exits 1 — an empty survey always means an empty
backlog, never a missing store.

Do not hand-roll this with `ls` and `wc`: the transcript directory is derived
from the working directory by a rule the sweep already implements, and an
ad-hoc pipeline both duplicates that rule and lands outside the `Bash(iwe:*)`
allowlist.

**Report the numbers and get a scope before spending anything.** A pending span
becomes several chunks and each chunk costs a model pass, so forty sessions is
well over forty passes. Say the count out loud, and **propose the scope in
signal, not in lines**: sessions sort by that column into a short head that
carries nearly all the user turns and a long tail that carries almost none. A
session with three hundred pending lines and two user turns is a long
autonomous run that asked nothing and settled nothing; one with forty turns is
where the decisions were made. So the recommendation to put to the user is
usually "drain these K, adopt the rest" — with both numbers named. Then offer
the three scopes:

- a first wave, to see the shape of what comes out;
- everything, when the user asks for it;
- none of it — adopting the past without reading it, so memory starts from
  today and the sweep stops offering the backlog:

  ```bash
  iwe internal claude hook stop --adopt
  ```

  That writes one session record per transcript with its watermark already at
  the current line count. It reads none of them, and it is reversible:
  `iwe internal claude job reset <session>` rewinds a session to the start
  (`--to <line>` for a partial rewind), drops its stale chunks, and the next
  sweep re-imports the span. That reversibility is what makes under-draining
  the safe error: a low-signal session adopted today can be drained tomorrow,
  while a pass spent on it is spent for good. Say so — it is the fact that
  lets a user pick a small scope without hedging.

Resumability is what makes a partial run safe to offer: watermarks live in the
session documents, so stopping after any wave and re-running this skill later
picks up exactly where it left off, with no duplicated work.

## 4. Process in waves, curating as you go

Repeat until the backlog is drained or the user's scope is met.

**Pick the drain's shape from the survey, before the first import.** The two
savings this skill knows — larger chunks, and batch triage — spend the same
budget and do not compose: a tool result holds only so much, so `job frontier`
stops a batch at 24000 characters, and a chunk cut at 25000 fills a batch by
itself. One lever per run, and the `signal` column has already chosen it.

**Mostly high-signal sessions pending** — the usual shape once the low-signal
tail is adopted — means nearly every chunk earns full curation anyway, so the
saving is fewer, larger passes. Raise both budgets and drain sequentially with
`job next`:

```bash
iwe update -k MEMORY --set chunk_chars=25000 --expect 1
iwe update -k MEMORY --set max_items_per_chunk=7 --expect 1
```

The item budget scales with the character budget (7 items per 25000 characters
is the default's 3-per-10000, rounded up), because a chunk 2.5× the size holds
2.5× the lessons and a budget that did not move would silently drop them.

**Mostly empty spans pending**, drained anyway at the user's ask, means most
passes will end at "kept nothing", so the saving is finishing several sessions
per read. Keep the defaults — two or three 10000-character digests fit one
`job frontier` batch — and drain in batches.

Either way the knobs are read **at import time**: every chunk document carries
the `max_items` and the character budget in force when the sweep materialized
it. So a raise has to happen before the first `hook stop --max-chunks` of the
run, and raised budgets go back once the last wave has been imported:

```bash
iwe update -k MEMORY --set chunk_chars=10000 --expect 1
iwe update -k MEMORY --set max_items_per_chunk=3 --expect 1
```

Those are the live-capture defaults, and they are what a turn boundary should
go back to: one 10000-character chunk arrives whole inside a tool result and
reads densely in a single pass, which is the property a sized-up backfill
trades away deliberately and a live capture should not.

**Import a wave.** The sweep the Stop hook runs is the same one that imports a
drain wave; run it directly, with the wave size as a flag:

```bash
iwe internal claude hook stop --max-chunks 10
```

Each run writes up to that many chunk documents — digests included — and prints
the block JSON the Stop hook would have emitted, which you can ignore here.
`--survey` again shows what is now queued. Ten chunks is the right first wave
whatever the sizing: lines-per-chunk varies about 2× between transcripts — and
another 2.5× between the backfill budget and the default — so size the later
waves from what `--survey` says is still pending, not from what the last wave
drained.

**Drain it.** With the `iwe` plugin installed, launch the `distill` agent; its
prompt carries no path, because it runs in this same directory. **One agent at
a time, never in parallel.** The completion protocol assumes a single writer
per store — the claim stamp exists to enforce exactly that when the Stop hook
does the launching, and hand-launching here bypasses it. Concurrent agents
race the session record's watermark and can strand pending chunks behind a
captured one (the sweep heals that on its next run, but only after the wave
has already stalled), and each one dedups against a store that is missing the
others' writes, so every duplicate survives to be merged by hand. Without the plugin there is no capture agent, so run its
loop yourself: it is written out in `agents/distill.md`, and the short version
is two-speed.

Read the store once with `iwe internal claude job brief` — the policy, the
frontmatter this store's documents carry, its most recent keys. Then run the
loop the survey's shape picked.

On a low-signal backlog, take a batch:

```bash
iwe internal claude job frontier
```

That prints one pending chunk per session in a single output, newest session
first — the only set that can be judged in any order, since a session's own
chunks complete in watermark order. Judge each digest against the policy's
"what to capture" bar and nothing else. A digest that keeps nothing is finished
right there, for the price of having read it:

```bash
iwe internal claude job complete <session> --lines <covers_lines>
```

A digest that keeps something gets the full rigor — dedup search, retrieve
before citing a document as already covering the item, imitating the store's
shape — and then the same completion with one `--wrote <key>` per document
written. Then take the next batch; empty output means the queue is drained.
That is triage's whole saving: one cheap pass per session plus the full rigor
per *keep*, instead of the full rigor per chunk.

On a high-signal backlog at the raised budgets, loop on `iwe internal claude
job next` instead — it serves one chunk at a time in the same order, which is
all a 25000-character chunk would leave of a batch anyway — and give every
digest the same two endings: a bare completion for one that keeps nothing, the
full rigor and `--wrote` keys for one that does.

**Never complete a chunk whose digest nobody read** — `iwe internal claude job
skip <session> --lines <covers_lines>` refuses one cheaply: it parks that
session, the queue serves the others, and the chunk returns to a fresh agent
after the in-flight TTL.

**Verify each wave against the store, never against the agent's report.**
Agents mis-tally and misreport queue state with some regularity; the store
does not. After every wave, `--survey` for what is actually still pending, §5's
`iwe find` for what actually landed, and read a couple of the new documents.
Treat "the queue is empty" as a claim until `iwe internal claude job next`
prints nothing.

Then, **before starting the next wave**, curate what just landed yourself,
exactly as far as the policy's curation section directs. Merging duplicates is
the floor — same fact means one document, the older key survives, the loser
goes with `iwe delete -k <key> --expect 1` — and whatever else the section
licenses (promotion into hubs, splitting, re-linking) is equally in scope
between waves. `iwe stats similarity -t 0.55` is the objective sweep for
near-duplicate pairs; a threshold loose enough to flag real overlap will also
flag structurally similar siblings, so read before merging.

**Curating between waves rather than at the end is the point, not politeness.**
A backlog of sessions relearns the same lesson over and over, so a single pass at
the end faces hundreds of near-duplicates at once. The queue helps here — it
serves the most recent session first, so the store's current truth lands before
the older spans that relearn it, and an old wave that resolves to "kept
nothing" is the design working rather than capture going blind. Worse, capture dedups by
searching the store before it writes: if the duplicates are still sitting there
unmerged, the next wave's `iwe find --lexical` returns five weak variants instead
of one good document, and it writes a sixth.

## 5. Finish

```bash
iwe schema validate     # only if this store binds schemas
iwe schema
iwe stats similarity -t 0.55
iwe find --filter '{ distilled_lines: { $exists: false }, covers_lines: { $exists: false }, $key: { $nin: [MEMORY] } }' \
  --sort 'created:-1' --limit 20 --project 'title=$title,key=$key'
iwe internal claude hook stop --survey
```

Then tell the user, concretely:

- how many documents memory added, and how many sessions were processed;
- how much backlog is left, if the run was scoped — and that re-running this
  skill continues it;
- that everything is uncommitted and theirs to review;
- that the `MEMORY.md` document is the switch and the policy: editing it changes
  what gets captured, deleting it turns memory off;
- how to keep it fed: install the plugin so the Stop hook captures at turn
  boundaries, and allowlist `Bash(iwe:*)` — all the background agents ever need;
- that `/iwe:distill` records something on demand and `/iwe:reflect` reorganizes
  the store with them.

## Migrating from a `memory/` store

Earlier versions kept memory in a separate `memory/` workspace with its own
`.iwe/`. The new model has no separate store. Migrate by moving the documents
into this workspace and enabling memory here:

```bash
iwe internal claude enable --typed   # if the old typed ontology should stay
for directory in learnings decisions gotchas topics daily sessions; do
  [ ! -d "memory/$directory" ] || cp -R "memory/$directory" .
done
iwe schema validate
```

Old session documents carry `distilled_lines` already if they were captured by a
recent version; if not, run §3's adopt-the-past loop so their transcripts are not
re-read. Leave `memory/` on disk for the user to review and delete — that is
their call, not yours.

## Guardrails

- **Chunks live in `.iwe/claude-sessions/`, never in the store.** Ignoring
  that directory in git is supported, and so is ignoring `sessions/` — the
  machinery reads its own state by path, never through the graph. Ignoring
  `sessions/` only hides the session records from graph browsing, which is
  what `/iwe:reflect` reads. Never move or hand-edit a chunk file.
- Watermarks only advance when a capture completes, to the `covers_lines` the
  chunk names. If state ever looks wrong — a session marked read that nobody
  read — `iwe internal claude job reset <session> --to <line>` rewinds it,
  drops the stale chunks past that line, and the next sweep re-imports the
  span; writes dedup, so re-capture costs duplicate work, never corruption.
  Never repair by editing chunk files or `distilled_lines` by hand.
- If capture fails on several chunks in a row, stop and report why rather than
  burning the rest of the backlog. The chunks left behind are the evidence:
  `iwe internal claude hook stop --survey` counts them (skipped ones
  included), and `iwe internal claude job next` shows the head.
- Never run `git add` or `git commit`, at any point.
