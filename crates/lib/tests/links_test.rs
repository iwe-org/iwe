use indoc::indoc;
use pretty_assertions::assert_str_eq;

use lib::{
    graph::Graph,
    model::graph::MarkdownOptions,
    state::{new_form_pairs, to_debug_string},
};

#[test]
#[ignore]
fn links_text_updated_from_header() {
    compare(
        vec![
            "file1.md",
            indoc! {"
            link: [file 2 title](file2)
            "},
            "file2.md",
            indoc! {"
            # file 2 title
            "},
        ],
        vec![
            "file1.md",
            indoc! {"
            link: [another title](file2)

            "},
            "file2.md",
            indoc! {"
            # file 2 title
            "},
        ],
    );
}

#[test]
#[ignore]
fn links_updated_two_way() {
    compare(
        vec![
            "file1.md",
            indoc! {"
            # file 1 title

            link: [file 2 title](file2)
            "},
            "file2.md",
            indoc! {"
            # file 2 title

            link: [file 1 title](file1)
        "},
        ],
        vec![
            "file1.md",
            indoc! {"
            # file 1 title

            link: [another title](file2)

            "},
            "file2.md",
            indoc! {"
            # file 2 title

            link: [another title](file1)
            "},
        ],
    );
}

#[test]
#[ignore]
fn ref_links_updated_one_way() {
    compare(
        vec![
            "file1.md",
            indoc! {"
            [file 2 title](file2)
            "},
            "file2.md",
            indoc! {"
            # file 2 title
        "},
        ],
        vec![
            "file1.md",
            indoc! {"
            [another title](file2)
            "},
            "file2.md",
            indoc! {"
            # file 2 title
            "},
        ],
    );
}

#[test]
fn ref_links_updated_two_way() {
    compare(
        vec![
            "file1.md",
            indoc! {"
            # file 1 title

            [file 2 title](file2)
            "},
            "file2.md",
            indoc! {"
            # file 2 title

            [file 1 title](file1)
        "},
        ],
        vec![
            "file1.md",
            indoc! {"
            # file 1 title

            [old title](file2)

            "},
            "file2.md",
            indoc! {"
            # file 2 title

            [old title](file1)
            "},
        ],
    );
}

#[test]
#[ignore]
fn drop_not_found_links_as_is() {
    compare(
        vec![
            "file1.md",
            indoc! {"
            # file 1 title

            link: file 2 title
            "},
        ],
        vec![
            "file1.md",
            indoc! {"
            # file 1 title

            link: [file 2 title](file2)
            "},
        ],
    );
}

fn compare(left: Vec<&str>, right: Vec<&str>) {
    let l = new_form_pairs(left.clone());
    let r = new_form_pairs(right.clone());

    let lstr = to_debug_string(&l);
    let graph = &Graph::import(&r, MarkdownOptions::default());

    println!("{:#?}", graph);

    let rstr = to_debug_string(&graph.export());
    assert_str_eq!(&lstr, &rstr,);
}
