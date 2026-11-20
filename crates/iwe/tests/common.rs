use std::path::PathBuf;

pub fn get_iwe_binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_iwe"))
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
