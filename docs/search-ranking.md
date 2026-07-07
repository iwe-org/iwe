# Search Ranking

How IWE ranks search results. This document covers the internals: what gets
indexed, how relevance is scored, how the index stays in sync with edits, and
how the three search surfaces differ.

For the user-facing overview see [Notes Search](feature-search.md).

## Surfaces and rankers

IWE exposes search through three surfaces backed by two ranking behaviors:

| Surface | Ranker | Corpus |
| --- | --- | --- |
| CLI [`iwe find`](cli-find.md) | BM25 | title + body |
| MCP `iwe_find` | BM25 | title + body |
| LSP workspace symbols | fuzzy **blended with** BM25 | title + body (BM25), section path (fuzzy) |

The BM25 index is built and maintained inside the document ingestion pipeline,
so every surface reads the same, always-current relevance data.

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

## CLI and MCP ranking

[`iwe find`](cli-find.md) and the MCP `iwe_find` tool share one code path. A
query first narrows the corpus to candidates via the structural filter
(`--filter`, `--includes`, `--references`, …), then BM25 ranks those candidates
by relevance, most relevant first. A no-query browse (filter only) falls back to
the previous popularity ordering (inbound reference + inclusion edge count).

With `--sort`, BM25 acts as a membership filter — non-matching documents are
dropped — and the surviving documents are ordered by the requested frontmatter
field instead of by score.

## LSP blend

The LSP workspace-symbol index is **section-grain** (one entry per header,
carrying the section's fuzzy-matchable path text), while the BM25 index is
**document-grain**. At query time each section is blended with **its document's**
BM25 score:

1. Compute the fuzzy score of the query against the section's path text.
2. Look up the document's BM25 score for the query.
3. Min-max normalize both signals to `[0, 1]` across the candidate set (a
   zero-range signal — all-equal — contributes `0`).
4. Combine: `0.5 × bm25_norm + 0.5 × fuzzy_norm`. The weight is a single tunable
   constant.

Ties fall back to the existing order (path length, page rank, key, line). An
empty query keeps the page-rank ordering unchanged.

The blend means a section can rank on either signal: a query term in a
document's body lifts it even when the header text does not fuzzy-match, while
fuzzy matching still surfaces near-miss header text that BM25's exact-token
matching would not. A document strong in **both** signals outranks documents
that are strongest in only one.

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

- **Typo tolerance** — BM25 matches stemmed exact tokens, so the CLI and MCP
  `find` lose the fuzzy subsequence matching of the old ranker (`find fizz` will
  not match `fuzz`). The LSP keeps fuzzy in its blend, so its typo tolerance is
  retained.
- **avgdl drift** — the average document length used for length normalization is
  fixed when the index is built. Over a very long editing session the
  incrementally-maintained index (LSP, MCP) drifts slightly from the true
  average, mildly degrading scores. The CLI rebuilds per run and never drifts.
- **CJK** — languages without whitespace word boundaries are not tokenized by the
  default tokenizer; a custom tokenizer would be required.
