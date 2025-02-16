use indoc::indoc;

#[test]
fn toc_one_level() {
    toc(
        indoc! {"
             # file 1 title

             [file 2 title](2)

             text 1
             _
             # file 2 title
             "},
        indoc! {"
             - [file 2 title](2)
             "},
    );
}

#[test]
fn toc_one_level_two_items() {
    toc(
        indoc! {"
             # file 1 title

             [file 2 title](2)

             text

             [file 2 title](2)

             text
             _
             # file 2 title
             "},
        indoc! {"
             - [file 2 title](2)
             - [file 2 title](2)
             "},
    );
}

#[test]
fn toc_one_recursive() {
    toc(
        indoc! {"
             # file 1 title

             [file 2 title](2)

             text 1
             _
             # file 2 title

             [file 1 title](1)

             "},
        indoc! {"
             - [file 2 title](2)
               - [file 1 title](1)
                 - [file 2 title](2)
             "},
    );
}

#[test]
fn toc_level_two() {
    toc(
        indoc! {"
             # file 1 title

             [file 2 title](2)

             text 1
             _
             # file 2 title

             [file 3 title](3)

             text 2
             _
             # file 3 title

             text 3
             "},
        indoc! {"
             - [file 2 title](2)
               - [file 3 title](3)
             "},
    );
}

#[test]
fn toc_multiple_items_on_level_two() {
    toc(
        indoc! {"
             # file 1 title

             [file 2 title](2)

             text 1
             _
             # file 2 title

             [file 3 title](3)

             [file 4 title](4)

             [file 3 title](3)

             text 2
             _
             # file 3 title

             text 3
             _
             # file 4 title

             text 4
             "},
        indoc! {"
                  - [file 2 title](2)
                    - [file 3 title](3)
                    - [file 4 title](4)
                    - [file 3 title](3)
             "},
    );
}

fn toc(_: &str, _: &str) {
    // todo: restore this?
}
