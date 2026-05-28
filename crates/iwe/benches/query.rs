mod common;

use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use tempfile::TempDir;

use liwe::model::Key;
use liwe::query::{
    evaluate, execute, CountOp, Filter, FindOp, InclusionAnchor, KeyOp, Operation, ReferenceAnchor,
};
use liwe::retrieve::{DocumentReader, RetrieveOptions};
use liwe::stats::KeyStatistics;

use common::{doc_key, generate_corpus, hub_key, load_graph, sample_keys};

const SIZES: &[usize] = &[5_000, 10_000, 20_000];
const SEED: u64 = 42;

fn bench_query(c: &mut Criterion) {
    for &n in SIZES {
        let dir = TempDir::new().expect("create tempdir");
        generate_corpus(dir.path(), n, SEED);
        let graph = load_graph(dir.path());

        bench_filters(c, &graph, n);
        bench_operations(c, &graph, n);
    }
}

fn bench_filters(c: &mut Criterion, graph: &liwe::graph::Graph, n: usize) {
    let mut group = c.benchmark_group("query/filter");
    group.sample_size(10);
    group.measurement_time(Duration::from_millis(7_500));

    let f_field_eq = Filter::eq("category", "beta");
    group.bench_with_input(BenchmarkId::new("field_eq", n), &n, |b, _| {
        b.iter(|| evaluate(&f_field_eq, graph));
    });

    let f_compound_and = Filter::and(vec![
        Filter::eq("type", "post"),
        Filter::eq("category", "beta"),
    ]);
    group.bench_with_input(BenchmarkId::new("compound_and", n), &n, |b, _| {
        b.iter(|| evaluate(&f_compound_and, graph));
    });

    let key_strs = sample_keys(n, 100);
    let key_refs: Vec<&str> = key_strs.iter().map(String::as_str).collect();
    let f_in_many_keys = Filter::Key(KeyOp::in_(&key_refs));
    group.bench_with_input(BenchmarkId::new("in_many_keys", n), &n, |b, _| {
        b.iter(|| evaluate(&f_in_many_keys, graph));
    });

    let f_included_by_d1 = Filter::IncludedBy(Box::new(InclusionAnchor::with_max(hub_key(), 1)));
    group.bench_with_input(BenchmarkId::new("included_by_d1", n), &n, |b, _| {
        b.iter(|| evaluate(&f_included_by_d1, graph));
    });

    let f_included_by_unbounded =
        Filter::IncludedBy(Box::new(InclusionAnchor::with_max(hub_key(), u32::MAX)));
    group.bench_with_input(BenchmarkId::new("included_by_unbounded", n), &n, |b, _| {
        b.iter(|| evaluate(&f_included_by_unbounded, graph));
    });

    let ref_target = doc_key(1);
    let f_referenced_by = Filter::ReferencedBy(Box::new(ReferenceAnchor::with_max(ref_target, 1)));
    group.bench_with_input(BenchmarkId::new("referenced_by", n), &n, |b, _| {
        b.iter(|| evaluate(&f_referenced_by, graph));
    });

    group.bench_with_input(BenchmarkId::new("roots", n), &n, |b, _| {
        b.iter(|| {
            let rk: Vec<Key> = graph
                .keys()
                .into_iter()
                .filter(|k| graph.get_inclusion_edges_to(k).is_empty())
                .collect();
            let f = Filter::Key(KeyOp::In(rk));
            evaluate(&f, graph)
        });
    });

    group.finish();
}

fn bench_operations(c: &mut Criterion, graph: &liwe::graph::Graph, n: usize) {
    let mut group = c.benchmark_group("query/op");
    group.sample_size(10);
    group.measurement_time(Duration::from_millis(7_500));

    let op_find = Operation::Find(
        FindOp::new()
            .filter(Filter::eq("category", "beta"))
            .limit(50),
    );
    group.bench_with_input(BenchmarkId::new("find_full", n), &n, |b, _| {
        b.iter(|| execute(&op_find, graph));
    });

    let op_count = Operation::Count(CountOp::new().filter(Filter::eq("status", "published")));
    group.bench_with_input(BenchmarkId::new("count", n), &n, |b, _| {
        b.iter(|| execute(&op_count, graph));
    });

    let reader = DocumentReader::new(graph);
    let target = Key::name(hub_key());
    let retrieve_opts = RetrieveOptions {
        backlinks: true,
        ..Default::default()
    };
    group.bench_with_input(BenchmarkId::new("retrieve_backlinks", n), &n, |b, _| {
        b.iter(|| reader.retrieve(&target, &retrieve_opts));
    });

    group.bench_with_input(BenchmarkId::new("stats_all", n), &n, |b, _| {
        b.iter(|| KeyStatistics::from_graph(graph));
    });

    group.finish();
}

criterion_group!(benches, bench_query);
criterion_main!(benches);
