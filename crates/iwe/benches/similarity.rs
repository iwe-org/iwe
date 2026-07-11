mod common;

use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use tempfile::TempDir;

use diwe::search::Language;
use diwe::stats::SimilarityIndex;

use common::{generate_corpus, load_graph};

const SIZES: &[usize] = &[100, 1_000];
const SEED: u64 = 42;

fn bench_similarity(c: &mut Criterion) {
    let mut group = c.benchmark_group("similarity");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(15));

    for &n in SIZES {
        let dir = TempDir::new().expect("create tempdir");
        generate_corpus(dir.path(), n, SEED);
        let graph = load_graph(dir.path());

        group.bench_with_input(BenchmarkId::new("build", n), &n, |b, _| {
            b.iter(|| SimilarityIndex::build(&graph, Language::English));
        });

        let similarity = SimilarityIndex::build(&graph, Language::English);
        group.bench_with_input(BenchmarkId::new("pairs", n), &n, |b, _| {
            b.iter(|| similarity.pairs());
        });
    }

    group.finish();
}

criterion_group!(benches, bench_similarity);
criterion_main!(benches);
