use itertools::Itertools;

use crate::model::State;

pub fn to_indoc(state: &State) -> String {
    state
        .iter()
        .map(|(key, file)| (key, file))
        .sorted_by_key(|a| a.0)
        .map(|file| file.1.to_string())
        .collect::<Vec<String>>()
        .join("_\n")
}

pub fn from_indoc(indoc: &str) -> State {
    indoc
        .split("\n_\n")
        .enumerate()
        .map(|(index, text)| ((index + 1).to_string(), text.trim().to_string()))
        .collect()
}

pub fn from_indoc_sub(indoc: &str) -> State {
    indoc
        .split("\n_\n")
        .enumerate()
        .map(|(index, text)| {
            let name = if index == 0 {
                "1".to_string()
            } else {
                format!("d/{}", index + 1)
            };
            (name.to_string(), text.trim().to_string())
        })
        .collect()
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
        .map(|(name, content)| (name.to_string(), content.to_string()))
        .collect()
}

#[test]
fn test_store_new_form_indoc() {
    use std::collections::HashMap;
    let store: HashMap<String, String> = {
        let indoc: &str = indoc::indoc! {"
            a
            _
            b
            _
            c
            "};
        from_indoc(indoc)
    };
    assert_eq!(store[&"1".to_string()], "a");
    assert_eq!(store[&"2".to_string()], "b");
    assert_eq!(store[&"3".to_string()], "c");
}
