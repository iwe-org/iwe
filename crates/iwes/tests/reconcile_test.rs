use crate::fixture::*;

#[test]
fn external_edit_to_open_document_applies_on_close() {
    let fixture = Fixture::with_workspace(vec![("note", "# Title One\n")]);
    fixture.wait_for_symbols("", &["Title One"]);

    fixture.open_doc("note", "# Title One\n");
    fixture.wait_for_symbols("", &["Title One"]);
    fixture.write_doc("note", "# Title Two\n");

    fixture.write_doc("marker", "# Marker One\n");
    fixture.wait_for_symbols("", &["Marker One", "Title One"]);
    fixture.write_doc("marker", "# Marker Two\n");
    fixture.wait_for_symbols("", &["Marker Two", "Title One"]);

    fixture.close_doc("note");
    fixture.wait_for_symbols("", &["Marker Two", "Title Two"]);
}

#[test]
fn unsaved_buffer_edits_survive_external_write_until_close() {
    let fixture = Fixture::with_workspace(vec![("note", "# Title One\n")]);
    fixture.wait_for_symbols("", &["Title One"]);

    fixture.open_doc("note", "# Title One\n");
    fixture.change_doc("note", "# Title Two\n");
    fixture.wait_for_symbols("", &["Title Two"]);
    fixture.write_doc("note", "# Title Three\n");

    fixture.write_doc("marker", "# Marker One\n");
    fixture.wait_for_symbols("", &["Marker One", "Title Two"]);
    fixture.write_doc("marker", "# Marker Two\n");
    fixture.wait_for_symbols("", &["Marker Two", "Title Two"]);

    fixture.close_doc("note");
    fixture.wait_for_symbols("", &["Marker Two", "Title Three"]);
}

#[test]
fn external_delete_of_open_document_applies_on_close() {
    let fixture =
        Fixture::with_workspace(vec![("note", "# Title One\n"), ("other", "# Title Two\n")]);
    fixture.wait_for_symbols("", &["Title One", "Title Two"]);

    fixture.open_doc("note", "# Title One\n");
    fixture.wait_for_symbols("", &["Title One", "Title Two"]);
    fixture.remove_doc("note");

    fixture.write_doc("marker", "# Marker One\n");
    fixture.wait_for_symbols("", &["Marker One", "Title One", "Title Two"]);
    fixture.write_doc("marker", "# Marker Two\n");
    fixture.wait_for_symbols("", &["Marker Two", "Title One", "Title Two"]);

    fixture.close_doc("note");
    fixture.wait_for_symbols("", &["Marker Two", "Title Two"]);
}

#[test]
fn open_buffer_content_wins_over_disk() {
    let fixture = Fixture::with_workspace(vec![("note", "# Title One\n")]);
    fixture.wait_for_symbols("", &["Title One"]);

    fixture.open_doc("note", "# Title Two\n");
    fixture.wait_for_symbols("", &["Title Two"]);
}

#[test]
fn unsaved_new_document_is_dropped_on_close() {
    let fixture = Fixture::with_workspace(vec![("note", "# Title One\n")]);
    fixture.wait_for_symbols("", &["Title One"]);

    fixture.open_doc("draft", "# Title Two\n");
    fixture.wait_for_symbols("", &["Title One", "Title Two"]);

    fixture.close_doc("draft");
    fixture.wait_for_symbols("", &["Title One"]);
}
