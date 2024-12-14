use std::thread::panicking;

use crate::model::{self, Key};

pub fn without_extension(key: &str) -> String {
    if !key.ends_with(".md") {
        return key.to_string();
    }
    key.trim_end_matches(".md").to_string()
}

pub fn with_extension(link: &str) -> model::Key {
    if link.ends_with(".md") {
        return link.to_string();
    }
    (&format!("{}.md", link)).to_string()
}
