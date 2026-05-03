mod common;

use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use tempfile::TempDir;

use common::{build_graph, generate_corpus, read_state};

const SIZES: &[usize] = &[5_000, 10_000, 50_000];

fn bench_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("load");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(30));

    for &n in SIZES {
        let dir = TempDir::new().expect("create tempdir");
        generate_corpus(dir.path(), n, 42);
        let prebuilt_state = read_state(dir.path());

        group.bench_with_input(BenchmarkId::new("full", n), &n, |b, _| {
            b.iter(|| {
                let state = read_state(dir.path());
                build_graph(&state)
            });
        });

        group.bench_with_input(BenchmarkId::new("walk_only", n), &n, |b, _| {
            b.iter(|| read_state(dir.path()));
        });

        group.bench_with_input(BenchmarkId::new("parse_only", n), &n, |b, _| {
            b.iter(|| build_graph(&prebuilt_state));
        });
    }

    group.finish();
}

criterion_group!(benches, bench_load);
criterion_main!(benches);
