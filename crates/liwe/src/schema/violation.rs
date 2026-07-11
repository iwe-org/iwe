use std::fmt;

use serde::ser::{Serialize, SerializeStruct, Serializer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Crumb {
    Frontmatter,
    Header(String),
    Position(usize),
    Field(String),
}

impl fmt::Display for Crumb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Crumb::Frontmatter => f.write_str("frontmatter"),
            Crumb::Header(text) => f.write_str(text),
            Crumb::Position(index) => write!(f, "sections[{index}]"),
            Crumb::Field(name) => f.write_str(name),
        }
    }
}

impl Serialize for Crumb {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub breadcrumb: Vec<Crumb>,
    pub message: String,
    pub hint: Option<String>,
    pub schema_pointer: String,
    pub keyword: String,
}

impl Violation {
    pub fn breadcrumb_text(&self) -> String {
        self.breadcrumb
            .iter()
            .map(|crumb| crumb.to_string())
            .collect::<Vec<_>>()
            .join(" › ")
    }
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let breadcrumb = self.breadcrumb_text();
        if breadcrumb.is_empty() {
            write!(f, "{}", self.message)?;
        } else {
            write!(f, "{breadcrumb}: {}", self.message)?;
        }
        if let Some(hint) = &self.hint {
            write!(f, "\nhint: {hint}")?;
        }
        Ok(())
    }
}

impl Serialize for Violation {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("Violation", 5)?;
        state.serialize_field("breadcrumb", &self.breadcrumb)?;
        state.serialize_field("message", &self.message)?;
        state.serialize_field("hint", &self.hint)?;
        state.serialize_field("schemaPath", &self.schema_pointer)?;
        state.serialize_field("keyword", &self.keyword)?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Violation {
        Violation {
            breadcrumb: vec![
                Crumb::Header("Session Notes".to_string()),
                Crumb::Header("Tasks".to_string()),
            ],
            message: "header is 18 tokens (limit 12)".to_string(),
            hint: Some("keep section headers short".to_string()),
            schema_pointer: "/sections/0/sections/1/header/maxTokens".to_string(),
            keyword: "maxTokens".to_string(),
        }
    }

    #[test]
    fn text_render_joins_breadcrumb_and_hint() {
        assert_eq!(
            sample().to_string(),
            "Session Notes › Tasks: header is 18 tokens (limit 12)\nhint: keep section headers short"
        );
    }

    #[test]
    fn text_render_without_breadcrumb_or_hint() {
        let violation = Violation {
            breadcrumb: vec![],
            message: "body is 2000 tokens (limit 1200)".to_string(),
            hint: None,
            schema_pointer: "/maxTokens".to_string(),
            keyword: "maxTokens".to_string(),
        };
        assert_eq!(violation.to_string(), "body is 2000 tokens (limit 1200)");
    }

    #[test]
    fn frontmatter_breadcrumb_renders_fields() {
        let violation = Violation {
            breadcrumb: vec![Crumb::Frontmatter, Crumb::Field("status".to_string())],
            message: "not one of 'draft', 'published'".to_string(),
            hint: None,
            schema_pointer: "/frontmatter/properties/status/enum".to_string(),
            keyword: "enum".to_string(),
        };
        assert_eq!(
            violation.to_string(),
            "frontmatter › status: not one of 'draft', 'published'"
        );
    }

    #[test]
    fn json_render_uses_schema_path_and_string_crumbs() {
        assert_eq!(
            serde_json::to_string(&sample()).unwrap(),
            "{\"breadcrumb\":[\"Session Notes\",\"Tasks\"],\"message\":\"header is 18 tokens (limit 12)\",\"hint\":\"keep section headers short\",\"schemaPath\":\"/sections/0/sections/1/header/maxTokens\",\"keyword\":\"maxTokens\"}"
        );
    }
}
