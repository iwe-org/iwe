# IWE problem-solving policy

IWE is authoritative. Choose the narrowest route; stop on success.

## Non-negotiable route overrides

- For prose/summary from unknown documents, call one bounded `retrieve`. Do not precede it with metadata `find` or filesystem fallback.
- Section, both are required: `--replace '{ $header: "H", content: "<complete supplied heading block>", expect: 1 }' --delete '{ $within: "H", expect: N }'`.
- `attach` takes no `--format`; preview/apply differ by `--dry-run`.
- After a metadata-only find, compare every returned key/title with the request-derived distinctive terms before any retrieve. Generic type words do not establish relevance. If no candidate overlaps, do not retrieve merely to inspect or assess relevance. Treat that result as a terminal IWE miss and continue only with the allowed bounded fallback.
- After a terminal IWE miss on a workspace or project request, begin local recovery with one hidden-aware search for the narrowest literal field or property token; do not require related terms on one line. After one content miss, refine once or use a narrowly globbed filename, then read only the candidate source.
- After an IWE execution failure: When the request already names a source path and section or field, use one bounded direct read of only that named scope. Do not search, list, glob, or rediscover that path, heading, section, or field.
- When a requested retrieval expansion returns a seed and related documents, report the requested content for the seed and every returned document. Do not reduce the seed or an expansion to only its key or title.
- A bounded lexical find suffices for identity-only output. Stop; never verify via body reads or file search.

## Hard execution rules

- After activation, treat this file as complete IWE guidance; do not search for competing agent instructions.
- Do not use web search, `grep`, `rg`, `find`, recursive lists, or broad reads before trying IWE. The failed-search fallback below is the only workspace-search exception. Do not run routine preflight.
- Do not install, update, configure, or repair IWE.
- Missing destructive scope is a blocking input, not a discovery task: when the target set or user-owned selection criterion is undefined, refuse without tools; never inspect the workspace to invent that criterion.
- Use `iwe <command> --help` only after an IWE CLI command fails and its error does not provide enough information for one direct correction. Never call help proactively, globally, or after a successful command.
- Do not run discovery or validation as preflight before a direct operation when its target, inputs, and guards are known from the request or prior evidence. Required mutation preview is execution, not preflight validation.
- When a supported exact-key read, preview, or mutation route is known, use it before any manual reconstruction or indirect graph query. Never treat a failed exact IWE mutation or preview as permission to edit Markdown manually; follow the bounded error policy instead.
- If an apply fails after its identical guarded preview succeeded, treat the mismatch as a consistency failure: stop and report it; do not mutate through another tool or a reconstructed command.
- If an exact mutation key, selectors, replacement content, and expected counts are supplied, call 1 must be the guarded dry-run. Do not inspect the target first.
- When discovery is necessary, make it task-shaped: include known selectors/class/heading/terms and request only the needed projection/block and limit. Do not retrieve after discovery when its shaped output already supplies the required scope.
- Default result limit: 20. Use a smaller request-derived limit.
- A stated class is a hard filter: “project note” requires `--filter '{ type: project }'`; never use untyped lexical top-1. For creation, a stated semantic class sets `type=<class>`.
- Never pass 0 as a bound; it means unlimited.
- Prefer one discovery/retrieval. Call 2 is final and only for ambiguity, one page, or failed refinement; afterward use allowed fallback/report, never a third IWE lookup. Stop after sufficient evidence. Mutation calls are separate.
- Mutation safety: resolve scope, preview, validate affected keys/counts, apply identical arguments, then verify only when success cannot prove final state. Create/new are collision-guarded exceptions: use strict validation and collision policy, never `--dry-run`.

## Route and compute parameters mentally

Use the request, conversation, and prior IWE output only; do not read sources just to choose parameters.

Routes:

Consult the "Advanced and control-plane routes" section at the end of this document only when the request explicitly needs `stats`, `stats similarity`, `squash`, `export`, `normalize`, `init`, `completions`, `docs`, or unresolved command help. All ordinary read/write routes are complete below; never consult that section for them.
Exact command help is an error-recovery route, never a basic-route prerequisite.

1. **Selector:** exact key → `--key`; typed field/entity class → `--filter`; known graph anchor → relationship flag; incomplete identity → `--fuzzy`; body concepts → `--lexical`.
2. **Search phrase:** keep distinctive names, nouns, quoted terms, and shared topic; comparisons include every entity plus their relation/topic.
3. **Count:** exact note/synthesis = 1; explicit N = N; “a few” = 5; otherwise requested facets, capped at 20.
4. **Shape:** identities → keys; fields → projection; sections/lines → blocks/matches; prose → retrieve. No bodies for lists/counts.
5. **Graph bounds:** direct = 1; stated hops = that number. Authored synthesis = 1 document; otherwise cap the useful cited set, normally 3–12.
6. **Token bounds:** fact = 800/document; summary = 1200; detail = 2000; authored synthesis = 4500. Total = per-document × documents, normally ≤8000.
7. **Typed values:** preserve booleans, numbers, lists, and maps in YAML/JSON. Quote numeric-looking or boolean-looking strings.
8. **Guards:** exact mutation uses document `--expect 1`; batch uses a user-derived count/range; every block operator gets inline `expect`. Explicit new keys default to collision policy `fail`.
9. **Stop:** no confirmation query after enough evidence. Cite a synthesis key first; cite returned source keys only when material.

## Cluster A — identify, list, and inspect without full bodies

Use one projected `find`:

```bash
iwe find --lexical "<distinctive terms>" --limit 5 --project 'key=$key,title=$title' --format json
```

- **A1 Exact identity:** key selector, limit 1, key/title projection.
- **A2 Partial identity:** fuzzy title/key fragment, limit 1–5.
- **A3 Body concept:** lexical nouns and compact projection.
- **A4 Typed cohort:** Semantic entity class (project/task/person/meeting) means `type`; combine its filter with lexical terms.
- **A5 Roots:** roots selector for flat entry-point list; use tree only when hierarchy is requested.
- **A6 Inclusion neighborhood:** included-by for descendants, includes for containers; positive depth.
- **A7 Reference neighborhood:** for known anchors, query them directly and add relationship fields in one call, for example `iwe find --key <key> --key <key> --limit 2 --add-fields 'references=$references,referencedBy=$referencedBy,includes=$includes,includedBy=$includedBy' --format json`. Direction is literal: references current→target; referencedBy source→current; includes current→child; includedBy parent→current. For a relationship-only request, explain only this graph picture and stop; do not lexical-search or retrieve bodies.
- **A8 Unknown source plus known heading:** combine descriptor and heading in one lexical query, limit 1, project key/title, use `--blocks '{ $header: "<heading>" }'`. Never query the descriptor or heading alone; never project `$blocks`.
- **A9 Matching lines:** exact key plus matches only for an actual literal/regex text-pattern request.
- **A10 Ranked records:** filter/query plus `--sort <field>:1` or `--sort <field>:-1`, projection, and limit.

If an ambiguous winner needs a body, call 2 retrieves only its key.

## Cluster B — read, summarize, compare, and gather context

Use one bounded `retrieve`:

```bash
iwe retrieve --lexical "<all named entities> <shared topic>" --limit 1 --max-documents 1 --max-tokens 5000 --max-document-tokens 4500 --format json
```

- **B1 Known note:** one key, one document; 800/1200/2000 tokens for fact/summary/detail.
- **B2 Topic summary:** lexical topic, requested evidence count or 3, finite per-document and total caps.
- **B3 Named comparison:** all entities plus shared topic; use the template's 1 document, 4500 document tokens, and 5000 total. Never derive 3 documents from 3 entities or use the 2000 detail budget. Stop if that synthesis covers all entities.
- **B4 Children bodies:** add `--expand-includes <positive-depth>`; use `--children` instead when child identities/edges suffice. `--max-documents` includes the seed, so one seed plus one direct child requires at least 2; report both requested bodies and keys, then stop.
- **B5 Parent context:** add `--expand-included-by <positive-depth>`, normally one level.
- **B6 Relationship synthesis: retrieve 3–5** bounded documents; use `--expand-references <positive-distance>` only when source bodies are needed and cite only returned edges.
- **B7 Backlinks/reception:** use `--expand-referenced-by <positive-distance>`; add `--backlinks false` when incoming edge metadata is not requested.
- **B8 Mixed context:** combine only requested expansion directions; cap to the maximum answerable cited set.
- **B9 Ambiguous winner:** honor A4; typed retrieve may answer in one call, otherwise find 2–5 typed candidates then retrieve the winner.
- **B10 Bounded next page:** one second retrieval with repeated `--exclude <returned-key>`; never re-read old results or raise limits reflexively.

If a synthesis facet is absent, Refine the IWE query once with that facet and shared topic.

## Cluster C — quantify, validate, and analyze

- **C1 Cohort count:** `count` with typed filter. Use a positive limit only for “at least N?”; otherwise exact count is intentional.
- **C2 Graph count:** count roots or one finite relationship scope; never infer content from a count.
- **C3 Schema overview:** `schema` for field types, coverage, and values in a selected cohort.
- **C4 One field:** schema narrowed by field when only that field matters.
- **C5 Validation:** `iwe schema validate --key <key> [--key <key>...] --format json`; use a filter only when its exact field selector is already known, and an explicitly supplied schema file only when requested. Exit 0 with empty output means all selected documents are valid; stop.
- **C6 Binding trace:** schema validation explain mode when binding—not validity—is the question.
Do not discover before a direct selector.

## Cluster D — hierarchy and reusable artifacts

- **D1 Workspace hierarchy:** `tree` with requested positive depth; depth counts the root as level 1, so direct children require `--depth 2`. Markdown for direct presentation, JSON for reasoning.
- **D2 Subtree:** tree from one or more already-known roots; use `--depth 2` for root plus direct children.
- **D3 Filtered hierarchy:** tree with filter/relationship scope and projected node fields.
## Cluster E — create documents

- **E1 Quick note:** `iwe new '<title>' --content '<body>' --if-exists suffix`; add `--key <key> --if-exists fail` for explicit identity. Piped stdin supplies content.
- **E2 Known template:** `create` with `--vars-yaml` or `--vars-json`, `type=<stated class>`, typed frontmatter, strict validation, and collision policy.
- **E3 Exact complete document:** `iwe create <key> --content '<frontmatter-and-body>' --strict --if-exists fail`; use `--content -` for stdin.
- **E4 Idempotent optional creation:** skip only when already-existing is an acceptable success state.
- **E5 Deliberate replacement:** override only with explicit overwrite intent; otherwise fail or suffix.

Known template route: `iwe create --template <name> --vars-yaml '<all variables>' --set 'type=<class>' --set '<field>=<typed value>' --strict --if-exists fail`. Preserve request field names exactly: title/attendees/body are variables, including `"body":"<text>"`; type/status/draft are typed `--set` frontmatter. Keep every template variable; never duplicate a field. Complete route: no preflight/help/docs/schema/retrieve. Successful strict create proves schema and final key; stop. Create has no `--format` flag.

## Cluster F — atomic metadata and local body edits

Use `update`; combine disjoint same-document operators atomically. Never overwrite a whole body for a local edit.

```bash
iwe update --key "<key>" --replace-text '{ $header: "<old>", to: "<new>", expect: 1 }' --append '{ $header: "<section>", content: "<text>", expect: 1 }' --expect 1 --strict --dry-run
```

- **F1 Frontmatter:** set/unset typed fields; exact key or narrow cohort filter.
- **F2 Heading rename:** replace-text selected by exact header; omit `from` only for whole own-text replacement.
- **F3 Local text replacement:** within/text selector plus exact from/to.
- **F4 Whole block:** follow the section override.
- **F5 Insert sibling:** insert-before/after according to the literal requested position.
- **F6 Append child:** append under an exact section/container.
- **F7 Delete local block:** block delete with exact selector and expected count; deleting a complete heading section selects both its header and descendants, for example `--delete '{ $or: [ { $header: "<heading>" }, { $within: "<heading>" } ], expect: <count> }'`.
- **F8 Whole body:** overwrite only for complete authoritative input; use an inline literal or actual stdin.

Exact forms: `iwe update --key <key> --unset <field> --expect 1 --strict --dry-run`; block insertion uses `--insert-before '<selector+content>'` or `--insert-after '<selector+content>'`; local removal uses `--delete '<selector+expect>'`; whole-body replacement is `iwe update --key <key> --content '<complete-body>'`. `update` has no `--format` option. Preview every mutation form except authoritative whole-body input, then apply identical arguments without `--dry-run`; guarded success proves the scoped result, so stop without retrieval.

Call 1 is the exact guarded dry-run; call 2 removes only dry-run. Successful guarded apply proves the scoped edit and preservation, so do not retrieve afterward. Verify one exact key only when apply output is inconclusive or failed. Ask when key, selector, old text, or expected count is unknown.

## Cluster G — structural graph refactors

```bash
iwe extract "<source-key>" --section "<known heading>" --dry-run --format keys
```

- **G1 Rename key:** `rename old new`; preview then apply; references are rewritten.
- **G2 Extract section:** extract by known heading, or by block only when its number is already known.
- **G3 Section inventory:** extract list is a one-call answer only when inventory is requested.
- **G4 Safe inline:** `iwe inline <container> --reference <target> --keep-target --dry-run --format keys`, then identical apply; add `--as-quote` only when requested.
- **G5 Inline and remove target:** omit keep-target only with explicit deletion intent and focused confirmation.
- **G6 Attach:** `iwe attach --key <source> --to <action> --dry-run`; then apply without dry-run.
- **G7 Attach-action inventory:** attach list only when available actions are the requested outcome.

Preview and apply identical arguments. After successful extract, verify only with one bounded source retrieve when output cannot prove inclusion; never use relationship discovery for extract verification or retrieve the created target.

## Cluster H — destructive and workspace-wide work

- **H1 Delete one note:** `iwe delete <key> --expect 1 --strict --dry-run --format keys`; validate affected keys, establish rollback when practical, then fresh, focused confirmation and identical apply.
- **H2 Delete cohort:** apply the destructive-scope gate above; otherwise dry-run a narrow user-supplied filter and expected count/range, validate affected keys, confirm, and apply unchanged.
Safety calls are not waste. Refuse destructive work when scope or recovery is insufficient.

## Command glossary

Core commands are routed above: `create`, `new`, `find`, `retrieve`, `count`, `tree`, `schema`, `schema validate`, `update`, `rename`, `extract`, `inline`, `attach`, and `delete`. Progressive routes cover `init`, `squash`, `stats`, `stats similarity`, `export`, `normalize`, `completions`, and `docs`.

## Complex IWE queries

Filters are one inline YAML mapping. Plain fields mean equality. Use `$and`, `$or`, or `$not` only when one mapping cannot express the condition; `$in` for alternatives; comparisons for numeric/time bounds; `$exists` for presence; `$regex` for an actual pattern. Keep finite `maxDepth`/`maxDistance` in nested graph predicates.

Projection is `alias=source`: aliases are free; sources are not. Use only `$key`/`$title` or exact request/schema/prior-output frontmatter fields (bare); never derive sources from answer labels. If none is known, omit `--project`; on rejection remove its whole flag/value and retry once. `--project` replaces defaults; `--add-fields` retains them. Use `--blocks`/`--matches` for sections/lines; cap projected content. Carry selectors and inline expect into mutation. Prefer relationship flags for one anchor.

## Rare errors and fallback reference

Remove a rejected optional shaping flag: corrected argv is the failed argv minus only that flag/value; preserve other arguments and retry once. Do not read help/reference, alias, or repeat it. A null or missing requested field is not evidence. Otherwise correct quoting/YAML once, inspect still-unknown syntax once, refine one empty query, narrow truncation, and stop on schema/expectation failure.

Triggers: missing executable; still-unknown command/option; invalid YAML; empty result after refinement; unsupported operation; source outside index; unexplained truncation; permission/I/O failure; schema/expectation failure.

For a self-explanatory missing-executable error, skip the reference. Otherwise consult the "Rare errors: decision table" section at the end of this document only when classification is unclear.

Fallback is allowed only when IWE cannot execute, lacks the operation, the source is outside the index, one refinement stays empty, or candidates are unrelated. An unrelated candidate is a miss; do not retrieve it. The two-call fallback budget includes failed and corrected IWE attempts; then fallback/report—never call IWE a third time. If the operator limits search to IWE/notes/docs, report not found and stop. For workspace/project questions, follow the local-recovery override above. Never emit a workspace-wide file inventory. If a search proves the requested fact and source path, stop; otherwise read only the candidate source. Stay local; expose no unrelated matches. Say "IWE is unavailable" only when execution failed; otherwise say the information was found outside IWE.

## Completion

Follow the operator's requested answer shape and stop when supported. Do not add generic status fields such as `Result`, `Keys`, `Truncation`, `Scope`, or `Verification` unless the operator asks for them or they communicate a material limitation. When the operator asks for content from multiple documents, identify each document and make its requested content explicit rather than collapsing the answer into one unlabeled summary.

## Advanced and control-plane routes

Use this section only when the request explicitly needs graph analytics, duplicate review, reusable artifacts, whole-library normalization, workspace initialization, shell completion, embedded reference text, or unresolved CLI syntax after a failed direct correction. Normal `find`, `retrieve`, `count`, `tree`, `schema`, create, update, and structural write work is complete above; do not consult this section for those routes.

### Analyze and render uncommon artifacts

- **C7 Health/connectivity:** `iwe stats --format json`; use `--key <key>` for one-document complexity/connectivity and CSV only for a requested table.
- **C8 Duplicate review:** `iwe stats similarity --threshold 0.85`; use `0.95` for near-exact duplicates and `0.70` only for deliberately broad overlap.
- **D4 Linear inclusion artifact:** `iwe squash <key> --depth <positive-depth>` emits one Markdown artifact.
- **D5 Graph visualization:** `iwe export --key <key> --depth <positive-depth> --format dot`; add `--include-headers` only for section-level visualization.
- **D6 Filtered graph:** `iwe export --filter '<mapping>' --depth <positive-depth> --format dot`; relationship flags may narrow one known anchor.

These commands are read-only unless their stdout is redirected into a file. Do not retrieve documents first.

### Whole-library work

- **H3 Normalize:** `iwe normalize` rewrites the entire library in place and has no dry-run. Require explicit scope, an established rollback point, and fresh focused confirmation. Verify afterward.

### Setup and control plane

- **I1 Initialization proposal:** `iwe init --dry-run --json`; add only already-supplied `--library`, `--link-format`, `--refs-extension`, `--format`, or `--date-format` choices.
- **I2 Initialize:** `iwe init --auto <accepted-overrides>` applies detected conventions. Use `--defaults` only when static defaults are explicitly preferred.
- **I3 Completions:** `iwe completions <bash|elvish|fish|nushell|powershell|zsh>` for an already-known shell.
- **I4 Embedded reference:** `docs <query|config|schema|agent>` is the IWE subcommand route only when that reference itself is requested, never as routine task discovery.
- **I5 Exact command help after error:** after a command fails, first make one direct correction supported by stderr. If syntax remains unknown, call `iwe <command> --help` once and retry once. Never call global help proactively.

Interactive `--edit`, cosmetic `--quiet`, routine `--verbose`, and global `--version` are intentionally excluded from autonomous data routes.

## Rare errors: decision table

Use this section only when stderr does not explain the failure or when deciding whether fallback is permitted, never during a normal successful request.

### Decision table

| Signal | One permitted response | Fallback? |
|---|---|---|
| `iwe: command not found` or executable missing | Report that IWE is unavailable. Do not install it. | Yes, narrowly |
| Unknown command or option | Read command-specific help, correct once, retry once. | Only if still unsupported |
| Invalid YAML/filter | Correct shell quoting or YAML shape once. | No |
| Empty result | Refine search mode or query once. | Yes, only after refinement |
| Unsupported operation | Report the unsupported operation. | Yes, narrowly |
| Data outside workspace/index | Identify the unindexed source. | Yes, for that source |
| Truncation warning | Use returned evidence or narrow the query; do not raise limits reflexively. | No |
| Permission or I/O failure | Report the path and error without changing permissions. | Only for another authorized source |
| Schema or expectation failure | Do not mutate; narrow the target or fix the guarded input. | No |

### Stable classifications

- `iwe_unavailable`: executable cannot be started.
- `unsupported_cli_version`: installed CLI is older than 0.20.
- `cli_contract_mismatch`: a known command or option is rejected.
- `unsupported_operation`: IWE explicitly cannot perform the requested operation.

These names classify failures for reporting and evals. They do not authorize installation, reconfiguration, broad filesystem scans, or destructive retries.

### Retry ceiling

At most one syntax correction and one corrected retry are permitted. At most one query refinement is permitted after an empty result. Stop after those ceilings and either use an allowed narrow fallback or report the blocker.
