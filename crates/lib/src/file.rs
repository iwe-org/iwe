use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct File {
    pub name: String,
    pub content: String,
    pub created: Option<SystemTime>,
    pub modified: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
}

impl File {
    pub fn new(name: &str, content: &str) -> Self {
        Self {
            name: name.into(),
            content: content.into(),
            created: None,
            modified: None,
            accessed: None,
        }
    }
}
