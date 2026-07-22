use std::path::Path;
use std::process::Command;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Editor {
    VsCode,
    Helix,
    Zed,
    Neovim,
}

impl Editor {
    pub fn hint(&self) -> &'static str {
        match self {
            Editor::VsCode => "install the IWE extension from the VSCode marketplace",
            Editor::Helix => "add iwes as a markdown language server in languages.toml",
            Editor::Zed => "install the IWE extension from the Zed extension gallery",
            Editor::Neovim => "register iwes as a markdown LSP client",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Probes {
    pub agents_file: Option<String>,
    pub mcp_config: bool,
    pub claude_dir: bool,
    pub editors: Vec<Editor>,
    pub git_repository: bool,
    pub git_clean: bool,
    pub nested_library: Option<String>,
}

impl Probes {
    pub fn has_agent_surface(&self) -> bool {
        self.agents_file.is_some() || self.mcp_config || self.claude_dir
    }

    pub fn agent_surface_note(&self) -> String {
        let mut found = Vec::new();
        if let Some(file) = &self.agents_file {
            found.push(file.clone());
        }
        if self.mcp_config {
            found.push(".mcp.json".to_string());
        }
        if self.claude_dir {
            found.push(".claude/".to_string());
        }
        format!("found {}", found.join(", "))
    }
}

pub fn probe(root: &Path) -> Probes {
    let mut probes = Probes::default();

    for candidate in ["AGENTS.md", "CLAUDE.md"] {
        if root.join(candidate).is_file() {
            probes.agents_file = Some(candidate.to_string());
            break;
        }
    }

    probes.mcp_config = root.join(".mcp.json").is_file();
    probes.claude_dir = root.join(".claude").is_dir();

    if root.join(".vscode").is_dir() {
        probes.editors.push(Editor::VsCode);
    }
    if root.join(".helix").is_dir() {
        probes.editors.push(Editor::Helix);
    }
    if root.join(".zed").is_dir() {
        probes.editors.push(Editor::Zed);
    }
    if root.join(".nvim.lua").is_file() || root.join(".nvimrc").is_file() {
        probes.editors.push(Editor::Neovim);
    }

    probes.git_repository = root.join(".git").exists();
    if probes.git_repository {
        probes.git_clean = git_is_clean(root);
    }

    probes.nested_library = find_parent_library(root);

    probes
}

fn git_is_clean(root: &Path) -> bool {
    Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .current_dir(root)
        .output()
        .map(|output| output.status.success() && output.stdout.is_empty())
        .unwrap_or(false)
}

fn find_parent_library(root: &Path) -> Option<String> {
    let mut current = root.parent();
    while let Some(directory) = current {
        if directory.join(".iwe").is_dir() {
            return Some(directory.display().to_string());
        }
        current = directory.parent();
    }
    None
}
