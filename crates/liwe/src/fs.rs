use std::path::PathBuf;
use std::{collections::HashMap, fs};

use log::error;
use rayon::iter::IntoParallelRefIterator;
use rayon::prelude::*;

use crate::model::{Content, State};

pub fn write_file(key: &String, content: &Content, to: &PathBuf) -> std::io::Result<()> {
    fs::write(to.clone().join(format!("{}.md", key)), content.as_str())
}

pub fn new_for_path(base_path: &PathBuf) -> State {
    new_for_path_rec(base_path, vec![])
}

pub fn new_from_hashmap(map: HashMap<String, String>) -> State {
    map.into_iter()
        .map(|(key, content)| (key, content))
        .collect()
}

pub fn new_for_path_rec(base_path: &PathBuf, sub_path: Vec<String>) -> State {
    if !base_path.exists() {
        error!("path donsn't exist");
        return State::new();
    }

    let mut files: State = fs::read_dir(base_path)
        .unwrap()
        .into_iter()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().map_or(false, |ex| ex.eq("md")))
        .collect::<Vec<PathBuf>>()
        .par_iter()
        .flat_map(|path| read_file(path, &sub_path))
        .collect::<Vec<(String, Content)>>()
        .into_iter()
        .collect();

    let subs: State = fs::read_dir(base_path)
        .unwrap()
        .into_iter()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .flat_map(|path| {
            let mut sub = sub_path.clone();
            sub.push(path.file_name().unwrap().to_str().unwrap().to_string());
            new_for_path_rec(&path, sub)
        })
        .collect();

    files.extend(subs);

    files
}

pub fn write_store_at_path(store: &State, to: &PathBuf) -> std::io::Result<()> {
    for (key, content) in store.iter() {
        write_file(key, content, &to)?;
    }
    Ok(())
}

fn read_file(path: &PathBuf, sub: &Vec<String>) -> Option<(String, Content)> {
    if !path.is_file() {
        return None;
    }

    if !path.exists() {
        return None;
    }

    let sub_path = sub.join("/");

    if sub_path.is_empty() {
        fs::read_to_string(path)
            .ok()
            .map(|content| (format!("{}", to_file_name(path)), content))
    } else {
        fs::read_to_string(path)
            .ok()
            .map(|content| (format!("{}/{}", sub_path, to_file_name(path)), content))
    }
}

fn to_file_name(path: &PathBuf) -> String {
    let name = path.file_name().unwrap().to_string_lossy().to_string();
    name.trim_end_matches(".md").to_string()
}
