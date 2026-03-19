use std::env;
use std::path::PathBuf;

pub fn get_iwe_binary_path() -> PathBuf {
    let mut binary_path = env::current_dir().expect("Failed to get current directory");

    while !binary_path.join("Cargo.toml").exists() || !binary_path.join("crates").exists() {
        if !binary_path.pop() {
            panic!("Could not find workspace root");
        }
    }

    binary_path.push("target");

    ["debug", "release"]
        .into_iter()
        .map(|x| binary_path.join(x).join("iwe"))
        .find(|x| x.exists())
        .unwrap_or_else(|| panic!("Could not find iwe binary"))
}
