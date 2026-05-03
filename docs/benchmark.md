# Benchmark

How we measure IWE's performance, what the harness looks like, and the
numbers we have today.

## What we benchmark

The `iwe` CLI rebuilds the document graph on every invocation, then runs a
query. Each invocation pays both costs in sequence, so we measure them
separately:

1. **Load** — walk the project directory, read every `.md` file, parse it,
   build the in-memory `Graph` (arena + indexes). Paid once per invocation.
   The LSP server (`iwes`) and MCP server (`iwec`) pay this cost only once
   at startup and reuse the graph for the lifetime of the session.
2. **Query** — evaluate filters / structural anchors / projections against
   an already-loaded `Graph`. Paid per query inside an LSP session, or
   amortized in CLI commands that fan out (find, count, tree, retrieve,
   stats).

We don't benchmark `iwes` (the LSP server) end-to-end, the MCP server, or
mutating subcommands (`update`, `delete`, `extract`, `inline`, etc.) — they
write to disk and aren't in the hot read path.

## Harness

We use [`criterion`](https://docs.rs/criterion) — the de facto Rust
benchmarking framework. Two bench targets live under
[`crates/iwe/benches/`](../crates/iwe/benches/):

- `crates/iwe/benches/load.rs` — Stage 1, load only.
- `crates/iwe/benches/query.rs` — Stage 2, query only.

Both are wired in [`crates/iwe/Cargo.toml`](../crates/iwe/Cargo.toml) under
`[[bench]] harness = false`. Shared test helpers (corpus generator, graph
loader) live in `crates/iwe/benches/common.rs`.

Per-bench config: `sample_size(10)` for both stages (criterion's minimum),
`measurement_time(30s)` for load and `measurement_time(7.5s)` for query.
Trades narrower confidence intervals for shorter wall-clock — full runs
land at ~5 minutes per stage.

## Corpus

Synthetic, deterministic, generated into a `TempDir` at bench setup. The
generator (`crates/iwe/benches/common.rs::generate_corpus`) uses
`StdRng::seed_from_u64(42)` so the corpus is byte-identical across runs.

**Per doc** (`doc-NNNNN.md`):

```markdown
---
title: "Doc 00001"
type: post
category: <one of alpha | beta | gamma | delta>
status: <one of draft | published | scheduled>
created: 2026-01-01
---

[Doc 00742](doc-00742)

[Doc 00193](doc-00193)

[Doc 00518](doc-00518)

## Section 1

Some paragraph text mentioning [Doc 00077](doc-00077) in passing. Lorem ipsum dolor sit amet.

... (3 paragraphs per section, 2 sections per doc)
```

| Element | Count | Purpose |
|---|---|---|
| Frontmatter fields | 5 | exercise `Filter::Field` queries |
| Inclusion links (own line) | 3 per doc | inclusion edges for descendants/ancestors traversal |
| Headers (`##`) | 2 per doc | section structure for parser |
| Paragraphs | 6 per doc | parser load |
| Inline links | 6 per doc | reference edges for `referenced_by` / `references` |

Plus one `hub.md` with `n_docs / 10` inclusion links — anchor for
`included_by_d1` and `included_by_unbounded` benches.

**Sizes used**: `5_000`, `10_000`, `20_000` for both stages. Total file
counts on disk (each ~700 bytes): 5_001 / 10_001 / 20_001.

## Running

```bash
# Both stages
cargo bench -p iwe

# One stage at a time
cargo bench -p iwe --bench load
cargo bench -p iwe --bench query

# Filter to specific benches (substring match)
cargo bench -p iwe --bench load  -- 'load/10000'
cargo bench -p iwe --bench query -- 'query/filter/included_by_unbounded'
cargo bench -p iwe -- '/20000$'                # all 20k variants

# Quick mode (~10 samples, no statistical inference — for iteration)
cargo bench -p iwe -- --quick

# Save / compare baselines
cargo bench -p iwe -- --save-baseline before
# ... make changes ...
cargo bench -p iwe -- --baseline before
```

Criterion writes HTML reports + JSON to `target/criterion/`. Open
`target/criterion/report/index.html` for the overview.

## Reading criterion output

Each bench prints:

```
load/10000  time:  [low_95%  median  high_95%]
                 change: [low% median% high%]  (p = 0.NN < 0.05)
                 Performance has improved.
```

The three numbers are a **95% confidence interval** for per-iteration time,
derived from criterion's bootstrap of the 10 samples it took. Tight
intervals (low ≈ high, e.g. ±1%) mean the measurement is stable. Wide
intervals (±20%+) mean noise — re-run the affected benches in isolation:

```bash
cargo bench -p iwe --bench load -- 'load/20000$'
```

The `change:` line compares to the previous run criterion has saved to
disk (or to a named baseline if `--baseline NAME` was passed).

Outliers like `Found 2 outliers among 10 measurements (high severe)` flag
samples that fell far outside the typical distribution — usually system
noise (GC pause, disk hiccup). They don't invalidate the run but suggest
re-running for a clean signal.

## Numbers

All times are per-iteration, measured on an Apple M3 Pro laptop.

### Stage 1 — load

| Bench              | Time   |
| ------------------ | ------ |
| `load/5000`   | 128 ms |
| `load/10000`  | 285 ms |
| `load/20000`  | 631 ms |

### Stage 2 — query

| Bench                                  | n=5k     | n=10k    | n=20k    |
| -------------------------------------- | -------- | -------- | -------- |
| `query/filter/field_eq`                | 2.7 ms   | 5.7 ms   | 12.1 ms  |
| `query/filter/compound_and`            | 3.0 ms   | 6.3 ms   | 12.9 ms  |
| `query/filter/in_many_keys`            | 1.4 ms   | 2.9 ms   | 5.9 ms   |
| `query/filter/included_by_d1`          | 410 µs   | 920 µs   | 2.3 ms   |
| `query/filter/included_by_unbounded`   | 9.2 ms   | 21 ms    | 48 ms    |
| `query/filter/referenced_by`           | 121 µs   | 251 µs   | 499 µs   |
| `query/filter/roots`                   | 3.3 ms   | 13.3 ms  | 60 ms    |
| `query/op/find_full`                   | 1.8 ms   | 4.7 ms   | 15.0 ms  |
| `query/op/count`                       | 2.2 ms   | 5.8 ms   | 17.6 ms  |
| `query/op/retrieve_backlinks`          | 1.7 ms   | 3.5 ms   | 7.4 ms   |
| `query/op/stats_all`                   | 5.5 ms   | 13.0 ms  | 30.4 ms  |

Notable scaling:

- `query/filter/included_by_unbounded` is the slowest filter case — at
  n=20k the hub reaches ~94% of the corpus through 4-7 hops.
- `query/filter/roots` is super-linear in n. The `--roots` implementation
  at `crates/iwe/src/main.rs:1260` calls
  `graph.get_inclusion_edges_to(k).is_empty()` for every key, and that
  lookup is linear in total inclusion edges.
- `query/op/retrieve_backlinks` is super-linear for the same reason
  (inbound reference walk uses the same index).

## Iteration counts

Criterion runs each bench `(measurement_time / per_iter)` times, organized
into 10 samples for the bootstrap. Concrete iteration counts from the
current load run:

| Bench              | per-iter | iters/sample | total iters |
| ------------------ | -------- | ------------ | ----------- |
| `load/5000`   | 128 ms   | 28           | 280         |
| `load/10000`  | 285 ms   | 11           | 110         |
| `load/20000`  | 631 ms   | 5            | 50          |

At n=20 k the bench runs ~50 iterations per measurement — solid signal.
For tighter intervals at scale bump `sample_size(20)` and
`measurement_time(60s)`, accepting roughly 2× wall-clock.

## Profiling

For finding bottlenecks in the load path, `xctrace` (Apple's Instruments
CLI) works without sudo and produces a trace viewable interactively in
Instruments.app:

```bash
# Build the bench binary with debug symbols
CARGO_PROFILE_BENCH_DEBUG=true cargo bench -p iwe --bench load --no-run

# Find the binary path (hash will differ)
ls target/release/deps/load-*

# Record a 20-second profile of one bench
xctrace record --template "Time Profiler" \
  --output /tmp/load.trace \
  --launch -- target/release/deps/load-XXXXX \
  --profile-time 20 'load/20000'

# Open in Instruments
open /tmp/load.trace
```

The bench binary's `--profile-time SECONDS BENCH_NAME` flag (provided by
criterion) runs the iter closure in a tight loop without statistical
analysis, so you get clean profiling samples.

For flamegraph SVG output, `cargo flamegraph` works but needs `sudo`
because `dtrace` is restricted by SIP on macOS:

```bash
# cache sudo credentials, then:
sudo cargo flamegraph --bench load -- --profile-time 20 'load/20000'
```

## Out of scope

- `iwes` (LSP server) — separate harness; not currently benched.
- `iwec` MCP server — separate harness; not currently benched.
- Mutating CLI subcommands — not on the hot read path.
- Real-world corpus runs against `../marketing/` (102 files) — covered by
  the synthetic generator, which produces realistic frontmatter shapes.
- `iwe` binary as a subprocess — would re-pay the load on every
  invocation, defeating the load/query split.
