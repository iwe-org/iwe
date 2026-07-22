use std::path::Path;

use serde_json::{json, Map, Value as JsonValue};

use crate::init::settings::{SettingId, Settings};

pub const SECTION_START: &str = "<!-- iwe -->";
pub const SECTION_END: &str = "<!-- /iwe -->";

pub fn instructions(settings: &Settings) -> String {
    let library = match settings.get(SettingId::LibraryPath).as_text() {
        Some(path) if !path.is_empty() => format!("{}/", path),
        _ => "the repository root".to_string(),
    };

    let wiki = settings.get(SettingId::LinkFormat).as_text() == Some("wiki");
    let extension = settings
        .get(SettingId::RefsExtension)
        .as_text()
        .unwrap_or_default()
        .to_string();

    let link_example = if wiki {
        if settings.get(SettingId::WikiLinkPath).as_text() == Some("short") {
            "[[roadmap]]".to_string()
        } else {
            "[[projects/roadmap]]".to_string()
        }
    } else {
        format!("[Roadmap](projects/roadmap{})", extension)
    };
    let link_rule = if wiki {
        "Link between documents with wiki links"
    } else {
        "Link between documents with regular markdown links"
    };

    let key_naming = match settings.get(SettingId::KeyTemplate).as_text() {
        Some("{{title}}") => "Filenames mirror the document title, spaces included.",
        Some("{{id}}") => "Filenames are numeric identifiers.",
        _ => "Filenames are lowercase slugs with hyphens between words.",
    };

    format!(
        "{start}\n\
        ## Notes in this repository\n\
        \n\
        These notes form an IWE graph. Keep the following conventions when editing them.\n\
        \n\
        - Documents live under {library} and are addressed by key — the path without the extension.\n\
        - Every document starts with a single `#` header; that header is its title.\n\
        - {link_rule}, for example `{link_example}`.\n\
        - {key_naming}\n\
        - Run `iwe normalize` after bulk edits so formatting stays consistent.\n\
        \n\
        Useful commands:\n\
        \n\
        - `iwe find <query>` — full-text search across the graph\n\
        - `iwe retrieve <key>` — read a document and the documents it includes\n\
        - `iwe new <title>` — create a document from the default template\n\
        - `iwe update <key>` — apply a structured edit to a document\n\
        - `iwe normalize` — rewrite every document in canonical form\n\
        {end}\n",
        start = SECTION_START,
        end = SECTION_END,
        library = library,
        link_rule = link_rule,
        link_example = link_example,
        key_naming = key_naming,
    )
}

pub fn mcp_snippet() -> String {
    serde_json::to_string_pretty(&json!({
        "mcpServers": {
            "iwe": {
                "command": "iwec",
                "args": []
            }
        }
    }))
    .expect("snippet serializes")
        + "\n"
}

pub fn write_instructions(root: &Path, settings: &Settings) -> Result<String, String> {
    let target = root.join("AGENTS.md");
    let section = instructions(settings);

    let existing = std::fs::read_to_string(&target).unwrap_or_default();
    if existing.contains(SECTION_START) {
        return Ok("AGENTS.md already carries an iwe section — left untouched".to_string());
    }

    let updated = if existing.trim().is_empty() {
        section
    } else {
        format!("{}\n\n{}", existing.trim_end(), section)
    };

    std::fs::write(&target, updated)
        .map_err(|error| format!("failed to write AGENTS.md: {}", error))?;

    Ok(if existing.is_empty() {
        "created AGENTS.md".to_string()
    } else {
        "appended an iwe section to AGENTS.md".to_string()
    })
}

pub fn register_mcp(root: &Path) -> Result<String, String> {
    let target = root.join(".mcp.json");
    let existing = std::fs::read_to_string(&target).unwrap_or_default();

    let mut document: JsonValue = if existing.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(&existing)
            .map_err(|error| format!("failed to parse .mcp.json: {}", error))?
    };

    let root_object = document
        .as_object_mut()
        .ok_or_else(|| ".mcp.json is not a JSON object".to_string())?;

    let servers = root_object
        .entry("mcpServers")
        .or_insert_with(|| JsonValue::Object(Map::new()));
    let servers = servers
        .as_object_mut()
        .ok_or_else(|| "mcpServers in .mcp.json is not an object".to_string())?;

    if servers.contains_key("iwe") {
        return Ok(".mcp.json already registers iwe — left untouched".to_string());
    }

    servers.insert("iwe".to_string(), json!({ "command": "iwec", "args": [] }));

    let rendered = serde_json::to_string_pretty(&document)
        .map_err(|error| format!("failed to render .mcp.json: {}", error))?;
    std::fs::write(&target, rendered + "\n")
        .map_err(|error| format!("failed to write .mcp.json: {}", error))?;

    Ok(if existing.trim().is_empty() {
        "created .mcp.json with the iwec server".to_string()
    } else {
        "registered the iwec server in .mcp.json".to_string()
    })
}

#[cfg(test)]
mod tests {
    use std::fs::{read_to_string, write};

    use tempfile::TempDir;

    use super::{instructions, register_mcp, write_instructions};
    use crate::init::settings::{defaults, Confidence, SettingId, Settings, Value};

    fn wiki_settings() -> Settings {
        let mut settings = defaults();
        settings.set(
            SettingId::LinkFormat,
            Value::text("wiki"),
            Confidence::Detected,
            "3 wiki links vs 0 markdown links",
        );
        settings
    }

    #[test]
    fn appends_an_iwe_section_to_an_existing_agents_file() {
        let temp = TempDir::new().expect("Should create temp directory");
        write(
            temp.path().join("AGENTS.md"),
            "# House rules\n\nBe careful.\n",
        )
        .expect("Should write AGENTS.md");
        let settings = wiki_settings();

        let message = write_instructions(temp.path(), &settings).expect("Should write section");

        let agents = read_to_string(temp.path().join("AGENTS.md")).expect("Should read AGENTS.md");
        assert_eq!("appended an iwe section to AGENTS.md", message);
        assert_eq!(
            format!(
                "# House rules\n\nBe careful.\n\n{}",
                instructions(&settings)
            ),
            agents
        );
    }

    #[test]
    fn leaves_an_existing_iwe_section_untouched() {
        let temp = TempDir::new().expect("Should create temp directory");
        let settings = wiki_settings();
        let existing = format!("# House rules\n\n{}", instructions(&settings));
        write(temp.path().join("AGENTS.md"), &existing).expect("Should write AGENTS.md");

        let message = write_instructions(temp.path(), &settings).expect("Should report");

        let agents = read_to_string(temp.path().join("AGENTS.md")).expect("Should read AGENTS.md");
        assert_eq!(
            "AGENTS.md already carries an iwe section — left untouched",
            message
        );
        assert_eq!(existing, agents);
    }

    #[test]
    fn short_wiki_paths_use_a_bare_link_example() {
        let mut settings = wiki_settings();
        settings.set(
            SettingId::WikiLinkPath,
            Value::text("short"),
            Confidence::Detected,
            "0 wiki links carry a path, 3 are bare names",
        );

        let section = instructions(&settings);
        let bare: Vec<&str> = section
            .lines()
            .filter(|line| line.starts_with("- Link between"))
            .map(|line| line.trim_end())
            .collect();

        assert_eq!(
            vec!["- Link between documents with wiki links, for example `[[roadmap]]`."],
            bare
        );
    }

    #[test]
    fn registers_the_mcp_server_alongside_existing_servers() {
        let temp = TempDir::new().expect("Should create temp directory");
        write(
            temp.path().join(".mcp.json"),
            "{\n  \"mcpServers\": {\n    \"other\": { \"command\": \"other\" }\n  }\n}\n",
        )
        .expect("Should write .mcp.json");

        let message = register_mcp(temp.path()).expect("Should register");

        let text = read_to_string(temp.path().join(".mcp.json")).expect("Should read .mcp.json");
        let document: serde_json::Value =
            serde_json::from_str(&text).expect(".mcp.json is valid JSON");
        assert_eq!("registered the iwec server in .mcp.json", message);
        assert_eq!(
            serde_json::json!({
                "mcpServers": {
                    "iwe": { "command": "iwec", "args": [] },
                    "other": { "command": "other" }
                }
            }),
            document
        );
    }
}
