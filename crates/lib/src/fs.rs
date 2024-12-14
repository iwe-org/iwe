use std::fs;
use std::path::PathBuf;
use std::thread::panicking;

use rayon::iter::IntoParallelRefIterator;
use rayon::prelude::*;

use crate::file::File;
use crate::model::{Content, Document, Key, State};

pub fn write_file(key: &Key, content: &Content, to: &PathBuf) -> std::io::Result<()> {
    fs::write(to.clone().join(key.as_str()), content.as_str())
}

pub fn new_for_path(path: &PathBuf) -> State {
    if !path.exists() {
        eprintln!("path donsn't exist");
        return State::new();
    }

    let files: State = fs::read_dir(path)
        .unwrap()
        .into_iter()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<PathBuf>>()
        .par_iter()
        .flat_map(|path| read_file(path))
        .collect::<Vec<Document>>()
        .into_iter()
        .collect();

    files
}

pub fn write_store_at_path(store: &State, to: &PathBuf) -> std::io::Result<()> {
    for (key, content) in store.iter() {
        write_file(key, content, &to)?;
    }
    Ok(())
}

fn read_file(path: &PathBuf) -> Option<Document> {
    if !path.is_file() {
        return None;
    }

    if !path.exists() {
        return None;
    }

    fs::read_to_string(path).ok().map(|content| {
        (
            path.file_name().unwrap().to_string_lossy().to_string(),
            content,
        )
    })
}