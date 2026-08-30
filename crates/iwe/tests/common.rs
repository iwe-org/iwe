use std::env;
use std::path::PathBuf;

pub fn get_iwe_binary_path() -> PathBuf {
    let binary_name = format!("iwe{}", env::consts::EXE_SUFFIX);

    if let Ok(target_dir) = env::var("CARGO_TARGET_DIR") {
        let base = PathBuf::from(target_dir);
        if let Some(path) = ["debug", "release"]
            .into_iter()
            .map(|x| base.join(x).join(&binary_name))
            .find(|x| x.exists())
        {
            return path;
        }
    }

    let mut binary_path = env::current_dir().expect("Failed to get current directory");

    while !binary_path.join("Cargo.toml").exists() || !binary_path.join("crates").exists() {
        if !binary_path.pop() {
            panic!("Could not find workspace root");
        }
    }

    binary_path.push("target");

    ["debug", "release"]
        .into_iter()
        .map(|x| binary_path.join(x).join(&binary_name))
        .find(|x| x.exists())
        .unwrap_or_else(|| panic!("Could not find iwe binary"))
}

pub fn fenced_blocks(source: &str, language: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Option<String> = None;
    for line in source.lines() {
        match current.as_mut() {
            Some(block) => {
                if line.trim_end() == "```" {
                    blocks.push(current.take().unwrap());
                } else {
                    block.push_str(line);
                    block.push('\n');
                }
            }
            None => {
                let trimmed = line.trim_end();
                if trimmed == format!("```{}", language) || trimmed == format!("``` {}", language) {
                    current = Some(String::new());
                }
            }
        }
    }
    blocks
}
