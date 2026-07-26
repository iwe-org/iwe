use crate::fixture::*;

#[test]
fn picks_up_externally_modified_file() {
    let fixture = Fixture::with_workspace(vec![("note", "# Original Title\n")]);
    fixture.wait_for_symbols("", &["Original Title"]);

    fixture.write_doc("note", "# Updated Title\n");

    fixture.wait_for_symbols("", &["Updated Title"]);
}

#[test]
fn picks_up_externally_created_file() {
    let fixture = Fixture::with_workspace(vec![("note", "# First\n")]);
    fixture.wait_for_symbols("", &["First"]);

    fixture.write_doc("second", "# Second\n");

    fixture.wait_for_symbols("", &["First", "Second"]);
}

#[test]
fn picks_up_externally_deleted_file() {
    let fixture = Fixture::with_workspace(vec![("kept", "# Kept\n"), ("gone", "# Gone\n")]);
    fixture.wait_for_symbols("", &["Gone", "Kept"]);

    fixture.remove_doc("gone");

    fixture.wait_for_symbols("", &["Kept"]);
}
