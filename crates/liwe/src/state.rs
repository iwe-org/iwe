use std::collections::HashMap;

use itertools::Itertools;

use crate::model::{Key, State};

pub fn to_indoc(state: &State) -> String {
    state
        .iter()
        .map(|(key, file)| (key, file))
        .sorted_by_key(|a| a.0)
        .map(|file| file.1.to_string())
        .collect::<Vec<String>>()
        .join("_\n")
}

pub fn new_form_pairs(files: Vec<&str>) -> State {
    let pairs = files
        .iter()
        .enumerate()
        .filter(|(i, _)| i % 2 == 0)
        .map(|(_, name)| name)
        .zip(
            files
                .iter()
                .enumerate()
                .filter(|(i, _)| i % 2 == 1)
                .map(|(_, content)| content),
        );

    pairs
        .map(|(name, content)| (Key::from_file_name(name), content.to_string()))
        .collect()
}

#[test]
fn test_store_new_form_indoc() {
    let store: HashMap<Key, String> = {
        let indoc: &str = indoc::indoc! {"
            a
            _
            b
            _
            c
            "};
        indoc
            .split("\n_\n")
            .enumerate()
            .map(|(index, text)| {
                (
                    Key::from_file_name(&(index + 1).to_string()),
                    text.trim().to_string(),
                )
            })
            .collect()
    };
    assert_eq!(store[&"1.md".into()], "a");
    assert_eq!(store[&"2.md".into()], "b");
    assert_eq!(store[&"3.md".into()], "c");
}
