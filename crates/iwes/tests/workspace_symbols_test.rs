use indoc::indoc;

mod fixture;
use crate::fixture::*;

#[test]
#[allow(deprecated)]
fn one_file() {
    Fixture::with(indoc! {"
            # test
            "})
    .workspace_symbols(
        workspace_symbol_params(""),
        workspace_symbol_response(vec![uri(1).to_symbol_info(
            "test",
            lsp_types::SymbolKind::NAMESPACE,
            0,
            1,
        )]),
    );
}

#[test]
#[allow(deprecated)]
fn fuzzy_one_file() {
    Fixture::with(indoc! {"
            # test
            "})
    .workspace_symbols(
        workspace_symbol_params("tst"),
        workspace_symbol_response(vec![uri(1).to_symbol_info(
            "test",
            lsp_types::SymbolKind::NAMESPACE,
            0,
            1,
        )]),
    );
}

#[test]
#[allow(deprecated)]
fn fuzzy_two_files() {
    Fixture::with(indoc! {"
            # similar
            _
            # not really
            "})
    .workspace_symbols(
        workspace_symbol_params("liar"),
        workspace_symbol_response(vec![
            uri(1).to_symbol_info("similar", lsp_types::SymbolKind::NAMESPACE, 0, 1),
            uri(2).to_symbol_info("not really", lsp_types::SymbolKind::NAMESPACE, 0, 1),
        ]),
    )
    .workspace_symbols(
        workspace_symbol_params("rel"),
        workspace_symbol_response(vec![
            uri(2).to_symbol_info("not really", lsp_types::SymbolKind::NAMESPACE, 0, 1),
            uri(1).to_symbol_info("similar", lsp_types::SymbolKind::NAMESPACE, 0, 1),
        ]),
    );
}

#[test]
#[allow(deprecated)]
fn one_file_two_headers() {
    Fixture::with(indoc! {"
            # test

            ## test 2
            "})
    .workspace_symbols(
        workspace_symbol_params(""),
        workspace_symbol_response(vec![
            uri(1).to_symbol_info("test", lsp_types::SymbolKind::NAMESPACE, 0, 1),
            uri(1).to_symbol_info("test • test 2", lsp_types::SymbolKind::OBJECT, 2, 3),
        ]),
    );
}

#[test]
#[allow(deprecated)]
fn one_file_two_headers_same_level() {
    Fixture::with(indoc! {"
            # test

            # test 2
            "})
    .workspace_symbols(
        workspace_symbol_params(""),
        workspace_symbol_response(vec![
            uri(1).to_symbol_info("test", lsp_types::SymbolKind::NAMESPACE, 0, 1),
            uri(1).to_symbol_info("test 2", lsp_types::SymbolKind::NAMESPACE, 2, 3),
        ]),
    );
}

#[test]
#[allow(deprecated)]
fn two_files() {
    Fixture::with(indoc! {"
            # test 1
            _
            # test 2
            "})
    .workspace_symbols(
        workspace_symbol_params(""),
        workspace_symbol_response(vec![
            uri(1).to_symbol_info("test 1", lsp_types::SymbolKind::NAMESPACE, 0, 1),
            uri(2).to_symbol_info("test 2", lsp_types::SymbolKind::NAMESPACE, 0, 1),
        ]),
    );
}

#[test]
#[allow(deprecated)]
fn two_nested_files() {
    Fixture::with(indoc! {"
            # test 1
            _
            # test 2

            [test 1](1)
            "})
    .workspace_symbols(
        workspace_symbol_params(""),
        workspace_symbol_response(vec![
            uri(1).to_symbol_info("test 2 • test 1", lsp_types::SymbolKind::OBJECT, 0, 1),
            uri(2).to_symbol_info("test 2", lsp_types::SymbolKind::NAMESPACE, 0, 1),
        ]),
    );
}

#[test]
#[allow(deprecated)]
fn page_rank_applied_after_fuzzy_score() {
    Fixture::with(indoc! {"
            # test rank
            _
            # test rank
            _
            # another page

            link to [test 1](1)

            link to [test 2](2)

            link to [test 2](2)
            "})
    .workspace_symbols(
        workspace_symbol_params("test"),
        workspace_symbol_response(vec![
            uri(2).to_symbol_info("test rank", lsp_types::SymbolKind::NAMESPACE, 0, 1),
            uri(1).to_symbol_info("test rank", lsp_types::SymbolKind::NAMESPACE, 0, 1),
            uri(3).to_symbol_info("another page", lsp_types::SymbolKind::NAMESPACE, 0, 1),
        ]),
    );
}

#[test]
#[allow(deprecated)]
fn dual_nested_files() {
    Fixture::with(indoc! {"
            # test 1
            _
            # test 2

            [test 1](1)
            _
            # test 3

            [test 2](2)
            "})
    .workspace_symbols(
        workspace_symbol_params(""),
        workspace_symbol_response(vec![
            uri(2).to_symbol_info("test 3 • test 2", lsp_types::SymbolKind::OBJECT, 0, 1),
            uri(1).to_symbol_info(
                "test 3 • test 2 • test 1",
                lsp_types::SymbolKind::OBJECT,
                0,
                1,
            ),
            uri(3).to_symbol_info("test 3", lsp_types::SymbolKind::NAMESPACE, 0, 1),
        ]),
    );
}

#[test]
#[allow(deprecated)]
fn sub_one_file() {
    Fixture::with_documents(vec![("d/1", "# test")]).workspace_symbols(
        workspace_symbol_params(""),
        workspace_symbol_response(vec![uri_from("d/1").to_symbol_info(
            "test",
            lsp_types::SymbolKind::NAMESPACE,
            0,
            1,
        )]),
    );
}
