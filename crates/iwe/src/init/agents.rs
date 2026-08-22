use std::path::Path;

use serde_json::{json, Map, Value as JsonValue};

pub const SECTION_START: &str = "<!-- iwe -->";
pub const SECTION_END: &str = "<!-- /iwe -->";

pub fn agent_instructions() -> String {
    format!(
        "{start}\n\
        ## Notes in this repository\n\
        \n\
        These notes are an [IWE](https://iwe.md) knowledge graph — markdown documents the `iwe` CLI reads, searches and edits as one structure.\n\
        \n\
        Key principles:\n\
        \n\
        - Every document is a node in the graph, addressed by key — its path without the extension.\n\
        - A document starts with a single `#` header; that header is its title.\n\
        - Links between documents are the graph's edges; edit through `iwe` so they stay valid.\n\
        - `iwe normalize` rewrites every document in canonical form — run it after bulk edits.\n\
        \n\
        Learn the tool from the binary itself: `iwe help` lists the commands, `iwe docs` prints the reference for the installed version.\n\
        \n\
        As the graph forms, extend this section with the `iwe` queries that answer this repository's recurring questions.\n\
        {end}\n",
        start = SECTION_START,
        end = SECTION_END,
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

pub fn write_agent_instructions(root: &Path) -> Result<String, String> {
    let target = root.join("AGENTS.md");
    let section = agent_instructions();

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

    use super::{agent_instructions, register_mcp, write_agent_instructions};

    #[test]
    fn appends_an_iwe_section_to_an_existing_agents_file() {
        let temp = TempDir::new().expect("Should create temp directory");
        write(
            temp.path().join("AGENTS.md"),
            "# House rules\n\nBe careful.\n",
        )
        .expect("Should write AGENTS.md");

        let message = write_agent_instructions(temp.path()).expect("Should write section");

        let agents = read_to_string(temp.path().join("AGENTS.md")).expect("Should read AGENTS.md");
        assert_eq!("appended an iwe section to AGENTS.md", message);
        assert_eq!(
            format!("# House rules\n\nBe careful.\n\n{}", agent_instructions()),
            agents
        );
    }

    #[test]
    fn leaves_an_existing_iwe_section_untouched() {
        let temp = TempDir::new().expect("Should create temp directory");
        let existing = format!("# House rules\n\n{}", agent_instructions());
        write(temp.path().join("AGENTS.md"), &existing).expect("Should write AGENTS.md");

        let message = write_agent_instructions(temp.path()).expect("Should report");

        let agents = read_to_string(temp.path().join("AGENTS.md")).expect("Should read AGENTS.md");
        assert_eq!(
            "AGENTS.md already carries an iwe section — left untouched",
            message
        );
        assert_eq!(existing, agents);
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
