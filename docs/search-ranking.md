# Search Ranking

How IWE ranks search results. This document covers the internals: what gets
indexed, how relevance is scored, how the index stays in sync with edits, and
how the search surfaces differ.

For the user-facing overview see [Notes Search](feature-search.md). The
fuzzy/lexical split and RRF fusion described here are implemented across the CLI,
MCP, and LSP surfaces.

## Two rankers

IWE ranks with **two independent rankers**:

- **Fuzzy** — skim subsequence matching over a document's short `title + key`
  text. Tolerant of partial words and dropped characters (`auth` matches
  `Authentication`); it matches a subsequence of characters, not substitutions,
  so it does not correct a mistyped letter. Ranks by skim's match score.
- **Lexical** — [BM25](https://en.wikipedia.org/wiki/Okapi_BM25) full-text
  relevance over `title + body`. Stemmed exact tokens, ranked by term frequency,
  rarity, and length normalization.

When both are used for the same search, results are combined with **Reciprocal
Rank Fusion (RRF)** — see [Fusion](#fusion-rrf). The BM25 index is built and
maintained inside the document ingestion pipeline, so every surface reads the
same, always-current relevance data.

## Surfaces

| Surface | Query inputs | Default |
| --- | --- | --- |
| CLI [`iwe find`](cli-find.md) | positional (fuzzy) · `--fuzzy` · `--lexical` | fuzzy (positional is **deprecated** — use the flags) |
| MCP `iwe_find` | `fuzzy` · `lexical` | none — the caller picks a ranker explicitly |
| Query [`search`](query-language.md#search-find-only) stage on `find` (CLI `iwe find`, MCP `iwe_query`) | `lexical` · `fuzzy` | none — a `search` clause names its rankers |
| [`iwe retrieve`](cli-retrieve.md) seed query | `--lexical` · `--fuzzy` (`--limit` caps the seeds) | none — search runs only when a flag is given |
| LSP workspace symbols | one query string | always fuzzy **and** lexical, fused |

The query-language `search` stage and the `retrieve` seed query reuse this same ranking machinery — the `search` stage restricts membership and orders by relevance (see [Search](query-language.md#search-find-only)), and `retrieve` runs `search` over its candidate set to pick seeds before expanding the graph around them.

On the CLI, supplying both `--fuzzy` and `--lexical` fuses the two; the bare
positional query stays fuzzy for now but prints a deprecation warning and will be
removed. The MCP tool exposes only the explicit `fuzzy` / `lexical` parameters (an
agent issues precise queries; there is no implicit default). The LSP always runs
both rankers and fuses them.

## The BM25 index

Ranking uses [BM25](https://en.wikipedia.org/wiki/Okapi_BM25), the standard
full-text relevance function. It scores a document against a query from three
signals: how often the query terms appear in the document (term frequency), how
rare those terms are across the whole corpus (inverse document frequency), and
the document's length relative to the average (so a term in a short note counts
for more than the same term in a long one).

The index is an in-memory sparse-vector store: each document becomes a sparse
embedding keyed by token, plus an inverted index from token to the documents
that contain it. A query is embedded the same way, and only documents sharing at
least one token are scored — matching is sub-linear in corpus size.

### Corpus

Each document is indexed as plain text, not markup:

```
{title}
{body}
```

- **title** — the frontmatter title field if configured, otherwise the
  document's first `# ` header.
- **body** — the document rendered to plain text: headings lose their `#`,
  emphasis/strong/etc. unwrap to their text, link **display text is kept but the
  URL is dropped**, code block **content** is kept (searchable), and table header
  and cell text is included. Frontmatter is dropped. Block-reference (inclusion)
  links are **not** expanded, so transcluded content is not double-counted.

The document **key (path) is deliberately excluded** from the indexed text —
ranking is by content, not filename.

### Tokenization

The default tokenizer normalizes unicode, lowercases, splits on unicode word
boundaries, removes stop words, and applies [Snowball](https://snowballstem.org/)
stemming for the configured language. Stemming is why `deploying`, `deployed`,
and `deployment` all match a search for `deploy` — they share the stem `deploy`.

## Where the index lives

The index is a field on the `Graph`, built and kept current by the same three
mutators that own all document content:

- **build** — a full graph load (`from_state` / `from_path`) builds the index
  once, after titles are resolved. Corpus extraction and embedding run in
  parallel (above a 128-document threshold); the scorer is populated serially.
- **insert / update** — re-indexes the single changed document. An update
  removes the document's previous vector before inserting the new one, so terms
  that were edited out do not linger.
- **remove** — deletes the document's embedding and its inverted-index entries,
  so a deleted document can never appear in later results.

Because these three mutators are the only choke points for content, every
consumer (CLI, MCP tools, the MCP file watcher, LSP save/change/delete handlers)
keeps the index in sync with no extra wiring.

Indexing is opt-in per graph. A graph built without a search language carries no
index and its mutators do nothing search-related, so commands that never search
pay nothing. The CLI enables the index only for `find`; the long-running MCP and
LSP servers always enable it.

## Fusion (RRF)

When a search runs both rankers, their results are combined with **Reciprocal
Rank Fusion**. Each ranker produces its own ordered list; a document's fused score
sums a rank-based contribution from every list it appears in:

```
RRF(d) = Σ over each ranker where d appears:  1 / (k + rank_d)     rank is 1-based, k = 60
```

RRF fuses on **rank position only**, never on the raw scores. This is deliberate:
skim's fuzzy scores (arbitrary positive integers) and BM25's tf·idf floats live on
incomparable scales, so normalizing and averaging them is fragile. Rank-based
fusion sidesteps that and is the standard method for hybrid lexical retrieval.

The match set is the **union** — a document surfaced by either ranker appears;
documents ranked high in *both* float to the top, so a doc strong in title-fuzzy
*and* body-lexical outranks docs strong in only one. Ties break deterministically
(by key on the CLI/MCP; by path length, page rank, key, line on the LSP).

## CLI and MCP ranking

[`iwe find`](cli-find.md) and the MCP `iwe_find` tool share one code path. A query
first narrows the corpus to candidates via the structural filter (`--filter`,
`--includes`, `--references`, …). Then, within those candidates:

- one ranker set (`--fuzzy` only, or `--lexical` only) ranks by that ranker;
- both set (`--fuzzy` *and* `--lexical`) ranks by RRF fusion of the two.

A text query is a **filter** here (unlike the LSP): only matching documents are
returned — the union of the fuzzy and lexical matches. A no-query browse (filter
only) falls back to the popularity ordering (inbound reference + inclusion edge
count).

With `--sort`, the query acts purely as a membership filter (the same union) and
the survivors are ordered by the requested frontmatter field instead of by score.

## LSP fusion

The LSP workspace-symbol index is **section-grain** (one entry per header,
carrying the section's fuzzy-matchable path text), while the BM25 index is
**document-grain**. The LSP always runs both rankers and fuses with RRF:

1. Rank sections by fuzzy score against each section's path text.
2. Rank sections by **their document's** BM25 score for the query.
3. Fuse the two rankings with RRF and sort; sections matching neither ranker fall
   to the tail (still returned, up to 100). An empty query keeps the page-rank
   ordering unchanged.

So a section can rank on either signal — a body term lifts a document even when
the header does not fuzzy-match, and fuzzy matching still surfaces near-miss
header text that BM25's exact-token matching would miss.

Link completion is unaffected — it stays alphabetical and client-filtered.

## Configuration

Set the stemming language in `.iwe/config.toml`:

```toml
[search]
language = "english"
```

Default is `english`. Supported: `arabic`, `danish`, `dutch`, `english`,
`french`, `german`, `greek`, `hungarian`, `italian`, `norwegian`, `portuguese`,
`romanian`, `russian`, `spanish`, `swedish`, `tamil`, `turkish`. An unknown value
falls back to `english`. See [Configuration](configuration.md).

## Performance

Building the index adds **roughly a third** (~30–40%) to a cold graph load and
scales linearly at about **12 µs per document**. The cost is dominated by
**tokenization and Snowball stemming** — stemming every token is CPU-bound.
Corpus extraction and embedding run in parallel across documents (above a
128-document threshold); the final scorer population is serial because it writes
a shared inverted index.

The CLI pays this once per `find` invocation; the LSP and MCP servers pay it once
at startup and then only the per-edit cost of re-indexing a single document. See
[Benchmark](benchmark.md#stage-1--load) for the numbers.

## Caveats

- **Lexical is exact-token** — BM25 matches stemmed exact tokens, so `--lexical
  auth` will not match `authentication` (their stems differ). Reach for the fuzzy
  ranker (the CLI default, `--fuzzy`, or the LSP) when you want partial-word and
  subsequence tolerance; fuse both when you want relevance *and* tolerance.
- **avgdl drift** — the average document length used for length normalization is
  fixed when the index is built. Over a very long editing session the
  incrementally-maintained index (LSP, MCP) drifts slightly from the true
  average, mildly degrading scores. The CLI rebuilds per run and never drifts.
- **CJK** — languages without whitespace word boundaries are not tokenized by the
  default tokenizer; a custom (script-aware) tokenizer would be required. Tracked
  separately.
