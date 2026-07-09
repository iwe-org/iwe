use std::sync::Once;

use indoc::indoc;
use pretty_assertions::assert_str_eq;

use liwe::model::config::{MarkdownOptions, RefsPath, WikiLinkPath};
use liwe::{
    graph::Graph,
    markdown::MarkdownReader,
    model::State,
    state::{from_indoc, to_indoc},
};

#[test]
fn links_text_updated_from_referenced_header() {
    normalize(
        indoc! {"
            [title](2)
            _
            # title
            "},
        indoc! {"
            [another title](2)
            _
            # title
            "},
    );
}

#[test]
fn piped_wiki_links_text_not_updated_from_referenced_header() {
    normalize(
        indoc! {"
            [[2|custom title]]
            _
            # title
            "},
        indoc! {"
            [[2|custom title]]
            _
            # title
            "},
    );
}

#[test]
fn ref_links_updated_two_ways() {
    normalize(
        indoc! {"
            # title 1

            [title 2](2)
            _
            # title 2

            [title 1](1)
            "},
        indoc! {"
            # title 1

            [another title](2)
            _
            # title 2

            [another title](1)
            "},
    );
}

#[test]
fn keep_unknown_refs_as_is() {
    normalize(
        indoc! {"
            [some title](key)
            "},
        indoc! {"
            [some title](key)
            "},
    );
}

#[test]
fn keep_unknown_wiki_refs_as_is() {
    normalize(
        indoc! {"
            [[key]]
            "},
        indoc! {"
            [[key]]
            "},
    );
}

#[test]
fn keep_unknown_piped_wiki_refs_as_is() {
    normalize(
        indoc! {"
            [[key|title]]
            "},
        indoc! {"
            [[key|title]]
            "},
    );
}

#[test]
fn keep_title_there_is_no_title_in_referenced_file() {
    normalize(
        indoc! {"
        [title](2)
        _
        para
        "},
        indoc! {"
        [title](2)
        _
        para
        "},
    );
}

#[test]
fn normalization_drop_extension() {
    normalize(
        indoc! {"
        [title](1)
        "},
        indoc! {"
        [title](1.md)
        "},
    );
}

#[test]
fn normalization_ref_extension() {
    compare_with_extensions(
        indoc! {"
        [text](text.md)
        "},
        indoc! {"
        [text](text)
        "},
    );
}

#[test]
fn normalization_ref_existing_extension() {
    compare_with_extensions(
        indoc! {"
        [text](text.md)
        "},
        indoc! {"
        [text](text.md)
        "},
    );
}

#[test]
fn sub_links_text_updated_from_referenced_header() {
    compare_docs(
        merge([dir("", indoc! {"[text](a/1)"}), dir("a", indoc! {"# text"})]),
        merge([dir("", indoc! {"[old](a/1)"}), dir("a", indoc! {"# text"})]),
    );
}

#[test]
fn sub_dir_relative_inline_link_resolved() {
    compare_docs(
        merge([
            dir("a", indoc! {"[text](b/1)"}),
            dir("a/b", indoc! {"# text"}),
        ]),
        merge([
            dir("a", indoc! {"[old](b/1)"}),
            dir("a/b", indoc! {"# text"}),
        ]),
    );
}

#[test]
fn parent_dir_relative_inline_link_resolved() {
    compare_docs(
        merge([
            dir("a/b", indoc! {"[text](../c/1)"}),
            dir("a/c", indoc! {"# text"}),
        ]),
        merge([
            dir("a/b", indoc! {"[old](../c/1)"}),
            dir("a/c", indoc! {"# text"}),
        ]),
    );
}

#[test]
fn nested_parent_dir_relative_inline_link_resolved() {
    compare_docs(
        merge([
            dir("a/b/c", indoc! {"[text](../../d/1)"}),
            dir("a/d", indoc! {"# text"}),
        ]),
        merge([
            dir("a/b/c", indoc! {"[old](../../d/1)"}),
            dir("a/d", indoc! {"# text"}),
        ]),
    );
}

#[test]
fn wiki_link_across_directories_renders_as_bare_name() {
    compare_state(
        vec![
            ("notes/note", "[[target]]\n"),
            ("clippings/target", "# Target\n"),
        ],
        vec![
            ("notes/note", "[[target]]\n"),
            ("clippings/target", "# Target\n"),
        ],
    );
}

#[test]
fn wiki_link_full_path_shortened_to_bare_name_on_normalize() {
    compare_state_with_options(
        vec![
            ("notes/note", "[[target]]\n"),
            ("clippings/target", "# Target\n"),
        ],
        vec![
            ("notes/note", "[[clippings/target]]\n"),
            ("clippings/target", "# Target\n"),
        ],
        MarkdownOptions {
            wiki_link_path: WikiLinkPath::Short,
            ..Default::default()
        },
    );
}

#[test]
fn wiki_link_shortened_to_shortest_unique_suffix_on_normalize() {
    compare_state_with_options(
        vec![
            ("notes/note", "[[a/target]]\n"),
            ("x/a/target", "# A\n"),
            ("y/b/target", "# B\n"),
        ],
        vec![
            ("notes/note", "[[x/a/target]]\n"),
            ("x/a/target", "# A\n"),
            ("y/b/target", "# B\n"),
        ],
        MarkdownOptions {
            wiki_link_path: WikiLinkPath::Short,
            ..Default::default()
        },
    );
}

#[test]
fn wiki_link_full_path_kept_on_normalize() {
    compare_state_with_options(
        vec![
            ("notes/note", "[[clippings/target]]\n"),
            ("clippings/target", "# Target\n"),
        ],
        vec![
            ("notes/note", "[[clippings/target]]\n"),
            ("clippings/target", "# Target\n"),
        ],
        MarkdownOptions {
            wiki_link_path: WikiLinkPath::Full,
            ..Default::default()
        },
    );
}

#[test]
fn wiki_link_bare_name_expanded_to_full_path_on_normalize() {
    compare_state_with_options(
        vec![
            ("notes/note", "[[clippings/target]]\n"),
            ("clippings/target", "# Target\n"),
        ],
        vec![
            ("notes/note", "[[target]]\n"),
            ("clippings/target", "# Target\n"),
        ],
        MarkdownOptions {
            wiki_link_path: WikiLinkPath::Full,
            ..Default::default()
        },
    );
}

#[test]
fn wiki_link_shortest_suffix_expanded_to_full_path_on_normalize() {
    compare_state_with_options(
        vec![
            ("notes/note", "[[x/a/target]]\n"),
            ("x/a/target", "# A\n"),
            ("y/b/target", "# B\n"),
        ],
        vec![
            ("notes/note", "[[a/target]]\n"),
            ("x/a/target", "# A\n"),
            ("y/b/target", "# B\n"),
        ],
        MarkdownOptions {
            wiki_link_path: WikiLinkPath::Full,
            ..Default::default()
        },
    );
}

#[test]
fn wiki_link_preserved_keeps_bare_name_on_normalize() {
    compare_state_with_options(
        vec![
            ("notes/note", "[[target]]\n"),
            ("clippings/target", "# Target\n"),
        ],
        vec![
            ("notes/note", "[[target]]\n"),
            ("clippings/target", "# Target\n"),
        ],
        MarkdownOptions {
            wiki_link_path: WikiLinkPath::Preserve,
            ..Default::default()
        },
    );
}

#[test]
fn wiki_link_preserved_keeps_full_path_on_normalize() {
    compare_state_with_options(
        vec![
            ("notes/note", "[[clippings/target]]\n"),
            ("clippings/target", "# Target\n"),
        ],
        vec![
            ("notes/note", "[[clippings/target]]\n"),
            ("clippings/target", "# Target\n"),
        ],
        MarkdownOptions {
            wiki_link_path: WikiLinkPath::Preserve,
            ..Default::default()
        },
    );
}

#[test]
fn wiki_link_preserved_keeps_partial_suffix_on_normalize() {
    compare_state_with_options(
        vec![
            ("notes/note", "[[a/target]]\n"),
            ("x/a/target", "# A\n"),
            ("y/b/target", "# B\n"),
        ],
        vec![
            ("notes/note", "[[a/target]]\n"),
            ("x/a/target", "# A\n"),
            ("y/b/target", "# B\n"),
        ],
        MarkdownOptions {
            wiki_link_path: WikiLinkPath::Preserve,
            ..Default::default()
        },
    );
}

#[test]
fn wiki_link_across_directories_resolves_backlink() {
    setup();

    let state: State = vec![
        ("notes/note".to_string(), "[[target]]\n".to_string()),
        ("clippings/target".to_string(), "# Target\n".to_string()),
    ]
    .into_iter()
    .collect();

    let graph = Graph::import(
        &state,
        MarkdownOptions {
            refs_extension: String::default(),
            ..Default::default()
        },
        None,
    );

    assert_eq!(
        1,
        graph
            .get_inclusion_edges_to(&"clippings/target".into())
            .len()
    );
    assert_eq!(
        0,
        graph.get_inclusion_edges_to(&"notes/target".into()).len()
    );
}

#[test]
fn wiki_link_resolves_backlink_after_incremental_update() {
    setup();

    let state: State = vec![
        ("notes/note".to_string(), "# Note\n".to_string()),
        ("clippings/target".to_string(), "# Target\n".to_string()),
    ]
    .into_iter()
    .collect();

    let mut graph = Graph::import(
        &state,
        MarkdownOptions {
            refs_extension: String::default(),
            ..Default::default()
        },
        None,
    );

    graph.update_document("notes/note".into(), "[[target]]\n".to_string());

    assert_eq!(
        1,
        graph
            .get_inclusion_edges_to(&"clippings/target".into())
            .len()
    );
    assert_str_eq!("[[target]]\n", graph.to_markdown(&"notes/note".into()));
}

#[test]
fn normalization_of_refs_extensions() {
    setup();

    let mut graph = Graph::new_with_options(MarkdownOptions {
        refs_extension: ".md".to_string(),
        ..Default::default()
    });

    graph.from_markdown(
        "key".into(),
        "[link text](other-file.md)",
        MarkdownReader::new(),
    );

    let normalized = graph.to_markdown(&"key".into());

    assert_str_eq!("[link text](other-file.md)\n", normalized);
}

#[test]
fn normalization_preserves_fragment_anchor_with_refs_extension() {
    setup();

    let mut graph = Graph::new_with_options(MarkdownOptions {
        refs_extension: ".md".to_string(),
        ..Default::default()
    });

    graph.from_markdown(
        "key".into(),
        "[link text](other-file.md#section)",
        MarkdownReader::new(),
    );

    let normalized = graph.to_markdown(&"key".into());

    assert_str_eq!("[link text](other-file.md#section)\n", normalized);
}

#[test]
fn normalization_preserves_fragment_anchor_without_refs_extension() {
    setup();

    let mut graph = Graph::new_with_options(MarkdownOptions::default());

    graph.from_markdown(
        "key".into(),
        "[link text](other-file#section)",
        MarkdownReader::new(),
    );

    let normalized = graph.to_markdown(&"key".into());

    assert_str_eq!("[link text](other-file#section)\n", normalized);
}

#[test]
fn normalization_preserves_other_extensions() {
    setup();

    let mut graph = Graph::new_with_options(MarkdownOptions {
        refs_extension: ".md".to_string(),
        ..Default::default()
    });

    graph.from_markdown("key".into(), "[link text](file.txt)", MarkdownReader::new());

    let normalized = graph.to_markdown(&"key".into());

    assert_str_eq!("[link text](file.txt)\n", normalized);
}

#[test]
fn normalization_preserves_html_extension() {
    setup();

    let mut graph = Graph::new_with_options(MarkdownOptions {
        refs_extension: ".md".to_string(),
        ..Default::default()
    });

    graph.from_markdown("key".into(), "[link text](foo.html)", MarkdownReader::new());

    let normalized = graph.to_markdown(&"key".into());

    assert_str_eq!("[link text](foo.html)\n", normalized);
}

#[test]
fn normalization_preserves_pdf_extension() {
    setup();

    let mut graph = Graph::new_with_options(MarkdownOptions {
        refs_extension: ".md".to_string(),
        ..Default::default()
    });

    graph.from_markdown("key".into(), "[link text](foo.pdf)", MarkdownReader::new());

    let normalized = graph.to_markdown(&"key".into());

    assert_str_eq!("[link text](foo.pdf)\n", normalized);
}

#[test]
fn normalization_preserves_non_md_extension_with_fragment() {
    setup();

    let mut graph = Graph::new_with_options(MarkdownOptions {
        refs_extension: ".md".to_string(),
        ..Default::default()
    });

    graph.from_markdown(
        "key".into(),
        "[link text](foo.html#bar)",
        MarkdownReader::new(),
    );

    let normalized = graph.to_markdown(&"key".into());

    assert_str_eq!("[link text](foo.html#bar)\n", normalized);
}

#[test]
fn normalization_preserves_non_md_extension_in_subdir() {
    setup();

    let mut graph = Graph::new_with_options(MarkdownOptions {
        refs_extension: ".md".to_string(),
        ..Default::default()
    });

    graph.from_markdown(
        "key".into(),
        "[link text](bar/foo.html)",
        MarkdownReader::new(),
    );

    let normalized = graph.to_markdown(&"key".into());

    assert_str_eq!("[link text](bar/foo.html)\n", normalized);
}

#[test]
fn fragment_only_link_preserved() {
    setup();

    let mut graph = Graph::new_with_options(MarkdownOptions {
        refs_extension: ".md".to_string(),
        ..Default::default()
    });

    graph.from_markdown(
        "key".into(),
        "# title\n\n[text](#title)\n",
        MarkdownReader::new(),
    );

    let normalized = graph.to_markdown(&"key".into());

    assert_str_eq!("# title\n\n[text](#title)\n", normalized);
}

#[test]
fn root_absolute_inline_link_resolved_from_subdirectory() {
    compare_docs(
        dir(
            "a",
            indoc! {"
                # text
                _
                [text](1)
            "},
        ),
        dir(
            "a",
            indoc! {"
                # text
                _
                [old](/a/1)
            "},
        ),
    );
}

#[test]
fn root_absolute_inline_link_resolved_from_root() {
    compare_docs(
        dir(
            "",
            indoc! {"
                [text](2)
                _
                # text
            "},
        ),
        dir(
            "",
            indoc! {"
                [old](/2)
                _
                # text
            "},
        ),
    );
}

#[test]
fn root_absolute_inline_link_with_fragment_resolved_from_subdirectory() {
    compare_docs(
        dir(
            "a",
            indoc! {"
                # text
                _
                [text](1#section)
            "},
        ),
        dir(
            "a",
            indoc! {"
                # text
                _
                [old](/a/1#section)
            "},
        ),
    );
}

#[test]
fn relative_inline_link_with_fragment_resolved() {
    compare_docs(
        dir(
            "a",
            indoc! {"
                [text](2#section)
                _
                # text
            "},
        ),
        dir(
            "a",
            indoc! {"
                [old](2#section)
                _
                # text
            "},
        ),
    );
}

#[test]
fn refs_path_absolute_emits_root_absolute_link_on_normalize() {
    compare_docs_with_options(
        dir(
            "a",
            indoc! {"
                # text
                _
                [text](/a/1)
            "},
        ),
        dir(
            "a",
            indoc! {"
                # text
                _
                [old](1)
            "},
        ),
        MarkdownOptions {
            refs_path: RefsPath::Absolute,
            ..Default::default()
        },
    );
}

fn normalize(expected: &str, denormalized: &str) {
    setup();

    let graph = Graph::import(
        &from_indoc(denormalized),
        MarkdownOptions {
            refs_extension: String::default(),
            ..Default::default()
        },
        None,
    );

    let normalized = to_indoc(&graph.export());

    assert_str_eq!(expected, normalized);
}

pub type Documents = Vec<(&'static str, &'static str)>;

fn compare_state(exp: Documents, den: Documents) {
    compare_state_with_options(
        exp,
        den,
        MarkdownOptions {
            refs_extension: String::default(),
            ..Default::default()
        },
    );
}

fn compare_state_with_options(exp: Documents, den: Documents, options: MarkdownOptions) {
    setup();

    let expected: State = exp
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    let denormalized: State = den
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    let graph = Graph::import(
        &denormalized
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        options,
        None,
    );

    let normalized = &graph.export();

    assert_eq!(&expected, normalized);
}

fn dir(name: &str, docs: &str) -> State {
    docs.split("\n_\n")
        .enumerate()
        .map(|(index, text)| {
            let number = (index + 1).to_string();
            let key = if name.is_empty() {
                number
            } else {
                format!("{}/{}", name, number)
            };
            (key, format!("{}\n", text.trim()))
        })
        .collect()
}

fn merge(entries: impl IntoIterator<Item = State>) -> State {
    let mut docs = State::new();
    for entry in entries {
        docs.extend(entry);
    }
    docs
}

fn compare_docs(expected: State, denormalized: State) {
    compare_docs_with_options(
        expected,
        denormalized,
        MarkdownOptions {
            refs_extension: String::default(),
            ..Default::default()
        },
    );
}

fn compare_docs_with_options(expected: State, denormalized: State, options: MarkdownOptions) {
    setup();

    let graph = Graph::import(&denormalized, options, None);

    assert_eq!(expected, graph.export());
}

fn compare_with_extensions(expected: &str, denormalized: &str) {
    setup();

    let mut graph = Graph::new_with_options(MarkdownOptions {
        refs_extension: ".md".to_string(),
        ..Default::default()
    });

    graph.from_markdown("key".into(), denormalized, MarkdownReader::new());

    let normalized = graph.to_markdown(&"key".into());

    assert_str_eq!(expected, normalized);
}

static INIT: Once = Once::new();

fn setup() {
    INIT.call_once(|| {
        let _ = env_logger::builder().try_init();
    });
}
