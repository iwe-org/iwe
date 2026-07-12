use diwe::search::Language;
use diwe::search_query::build_index;
use diwe::stats::{
    broken_links, graph_findings, mutation_findings, orphan_keys, Finding, GraphStatistics, Rule,
    SimilarityIndex,
};
use indoc::indoc;
use liwe::graph::Graph;
use liwe::markdown::MarkdownReader;
use liwe::model::Key;

fn graph_with(docs: &[(&str, &str)]) -> Graph {
    let mut graph = Graph::new();
    for (key, content) in docs {
        graph.from_markdown(Key::name(key), content, MarkdownReader::new());
    }
    graph
}

const ALPHA: &str = indoc! {"
    # Ada and Kai

    Ada and Kai met in Vienna in the spring of 1998 while both were studying
    analog synthesizers together. They collaborated on a modular sequencer and
    later co-founded a small workshop building custom filters for touring
    musicians across Europe and beyond.
"};

const BETA: &str = indoc! {"
    # Ada and Kai

    Ada and Kai met in Vienna in the summer of 1998 while both were studying
    analog synthesizers together. They collaborated on a modular sequencer and
    later co-founded a small workshop building custom filters for touring
    musicians across Europe and beyond.
"};

const PARAPHRASE: &str = indoc! {"
    # Kai and Ada

    Kai and Ada first crossed paths in Vienna during the spring season of 1998
    while the two of them were studying analog synthesizers. Together they built
    a modular sequencer, and afterwards launched a modest workshop crafting
    bespoke filters for gigging musicians touring the continent.
"};

const LONG: &str = indoc! {"
    # Modular Synth Workshop

    The workshop builds custom analog filters and modular sequencers for touring
    musicians across the continent. Ada and Kai founded it in Vienna in 1998
    after studying analog synthesizers together at the conservatory. It also
    repairs vintage tape machines, sells patch cables, teaches soldering classes
    on weekends, and hosts a monthly ambient concert out in the courtyard.
"};

const CONTAINED: &str = indoc! {"
    # Modular Synth Workshop

    The workshop builds custom analog filters and modular sequencers for touring
    musicians across the continent. Ada and Kai founded it in Vienna in 1998
    after studying analog synthesizers together at the conservatory. It also
    repairs vintage tape machines and sells patch cables.
"};

const DISTINCT: &str = indoc! {"
    # Tax Filing Checklist

    Gather every receipt, confirm the standard deduction amount, review the
    quarterly estimated payments, reconcile the brokerage statements, and submit
    the completed federal return well before the April filing deadline to avoid
    any late penalties or accrued interest charges this year.
"};

const SHORT: &str = "# Ada\n\nAda studied analog synthesizers.\n";

fn similar_keys(graph: &Graph, key: &str) -> Vec<String> {
    SimilarityIndex::build(graph, Language::English)
        .similar(&Key::name(key))
        .iter()
        .map(|page| page.key.to_string())
        .collect()
}

#[test]
fn orphan_keys_lists_zero_inbound_sorted() {
    let graph = graph_with(&[
        ("hub", "# Hub\n\nLinks to [Leaf](leaf).\n"),
        ("leaf", "# Leaf\n"),
        ("lonely", "# Lonely\n"),
    ]);

    let orphans: Vec<String> = orphan_keys(&graph).iter().map(|k| k.to_string()).collect();
    assert_eq!(orphans, vec!["hub".to_string(), "lonely".to_string()]);
}

#[test]
fn orphan_keys_exempts_index_pages() {
    let graph = graph_with(&[
        ("index", "# Home\n\nLinks to [Leaf](leaf).\n"),
        ("docs/index", "# Docs\n"),
        ("leaf", "# Leaf\n"),
        ("lonely", "# Lonely\n"),
    ]);

    let orphans: Vec<String> = orphan_keys(&graph).iter().map(|k| k.to_string()).collect();
    assert_eq!(orphans, vec!["lonely".to_string()]);
}

#[test]
fn broken_links_covers_both_edge_kinds_and_skips_external() {
    let graph = graph_with(&[(
        "doc",
        "# Doc\n\nInline [missing](gone) and [external](https://example.com).\n\n[block ref](block-missing)\n",
    )]);

    let links: Vec<(String, String)> = broken_links(&graph)
        .iter()
        .map(|link| (link.source_key.to_string(), link.target_key.to_string()))
        .collect();
    assert_eq!(
        links,
        vec![
            ("doc".to_string(), "block-missing".to_string()),
            ("doc".to_string(), "gone".to_string()),
        ]
    );
}

#[test]
fn similar_pages_flags_mutual_near_identical_pair() {
    let graph = graph_with(&[("alpha", ALPHA), ("beta", BETA), ("distinct", DISTINCT)]);

    assert_eq!(similar_keys(&graph, "alpha"), vec!["beta".to_string()]);
    assert_eq!(similar_keys(&graph, "beta"), vec!["alpha".to_string()]);
    assert_eq!(similar_keys(&graph, "distinct"), Vec::<String>::new());
}

#[test]
fn similar_pages_excludes_paraphrase_and_containment() {
    let graph = graph_with(&[
        ("original", ALPHA),
        ("paraphrase", PARAPHRASE),
        ("long", LONG),
        ("contained", CONTAINED),
    ]);

    assert_eq!(similar_keys(&graph, "original"), Vec::<String>::new());
    assert_eq!(similar_keys(&graph, "paraphrase"), Vec::<String>::new());
    assert_eq!(similar_keys(&graph, "long"), Vec::<String>::new());
    assert_eq!(similar_keys(&graph, "contained"), Vec::<String>::new());
}

#[test]
fn similar_pages_skips_pages_below_the_token_gate() {
    let graph = graph_with(&[("alpha", ALPHA), ("tiny", SHORT)]);

    assert_eq!(similar_keys(&graph, "tiny"), Vec::<String>::new());
    assert_eq!(similar_keys(&graph, "alpha"), Vec::<String>::new());
}

#[test]
fn similar_pairs_lists_each_mutual_pair_once_alphabetically() {
    let graph = graph_with(&[("beta", BETA), ("alpha", ALPHA), ("distinct", DISTINCT)]);

    let pairs: Vec<(String, String)> = SimilarityIndex::build(&graph, Language::English)
        .pairs()
        .into_iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
    assert_eq!(pairs, vec![("alpha".to_string(), "beta".to_string())]);
}

#[test]
fn mutation_findings_reports_orphan_dangling_and_similar() {
    let graph = graph_with(&[
        (
            "root",
            "# Root\n\n[Linker](linker)\n\n[Alpha](alpha)\n\n[Beta](beta)\n",
        ),
        ("linker", "# Linker\n\nSee [gone](gone) for details.\n"),
        ("alpha", ALPHA),
        ("beta", BETA),
    ]);
    let index = build_index(&graph, Language::English);

    let findings = mutation_findings(&graph, &index, &[Key::name("alpha")]);
    assert_eq!(
        findings,
        vec![
            Finding {
                rule: Rule::Orphan,
                key: Key::name("root"),
                other: None,
                message: "no page links here".to_string(),
            },
            Finding {
                rule: Rule::DanglingLink,
                key: Key::name("linker"),
                other: Some(Key::name("gone")),
                message: "links to missing 'gone'".to_string(),
            },
            Finding {
                rule: Rule::SimilarPage,
                key: Key::name("alpha"),
                other: Some(Key::name("beta")),
                message: "closely matches 'beta' (0.94)".to_string(),
            },
        ]
    );
}

#[test]
fn mutation_findings_similar_page_uses_per_target_token_map() {
    let graph = graph_with(&[
        (
            "index",
            "# Index\n\n[Alpha](alpha)\n\n[Beta](beta)\n\n[Distinct](distinct)\n",
        ),
        ("alpha", ALPHA),
        ("beta", BETA),
        ("distinct", DISTINCT),
    ]);
    let index = build_index(&graph, Language::English);

    let findings = mutation_findings(&graph, &index, &[Key::name("alpha")]);
    assert_eq!(
        findings,
        vec![Finding {
            rule: Rule::SimilarPage,
            key: Key::name("alpha"),
            other: Some(Key::name("beta")),
            message: "closely matches 'beta' (0.94)".to_string(),
        }]
    );
}

#[test]
fn graph_findings_returns_orphans_and_dangling_only() {
    let graph = graph_with(&[
        ("alpha", ALPHA),
        ("beta", BETA),
        ("linker", "# Linker\n\nSee [gone](gone).\n"),
    ]);

    let findings = graph_findings(&graph);
    assert_eq!(
        findings,
        vec![
            Finding {
                rule: Rule::Orphan,
                key: Key::name("alpha"),
                other: None,
                message: "no page links here".to_string(),
            },
            Finding {
                rule: Rule::Orphan,
                key: Key::name("beta"),
                other: None,
                message: "no page links here".to_string(),
            },
            Finding {
                rule: Rule::Orphan,
                key: Key::name("linker"),
                other: None,
                message: "no page links here".to_string(),
            },
            Finding {
                rule: Rule::DanglingLink,
                key: Key::name("linker"),
                other: Some(Key::name("gone")),
                message: "links to missing 'gone'".to_string(),
            },
        ]
    );
}

#[test]
fn graph_statistics_populates_orphans() {
    let graph = graph_with(&[("alpha", ALPHA), ("beta", BETA)]);
    let stats = GraphStatistics::from_graph(&graph);

    let orphans: Vec<String> = stats.orphans.iter().map(|k| k.to_string()).collect();
    assert_eq!(orphans, vec!["alpha".to_string(), "beta".to_string()]);
}
