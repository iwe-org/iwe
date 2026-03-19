use std::fmt;

use crate::model::Key;

#[derive(Debug, Clone, Default)]
pub struct Changes {
    pub creates: Vec<(Key, String)>,
    pub updates: Vec<(Key, String)>,
    pub removes: Vec<Key>,
}

impl Changes {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create(mut self, key: Key, markdown: String) -> Self {
        self.creates.push((key, markdown));
        self
    }

    pub fn update(mut self, key: Key, markdown: String) -> Self {
        self.updates.push((key, markdown));
        self
    }

    pub fn remove(mut self, key: Key) -> Self {
        self.removes.push(key);
        self
    }

    pub fn add_create(&mut self, key: Key, markdown: String) {
        self.creates.push((key, markdown));
    }

    pub fn add_update(&mut self, key: Key, markdown: String) {
        self.updates.push((key, markdown));
    }

    pub fn add_remove(&mut self, key: Key) {
        self.removes.push(key);
    }

    pub fn is_empty(&self) -> bool {
        self.creates.is_empty() && self.updates.is_empty() && self.removes.is_empty()
    }

    pub fn affected_keys(&self) -> Vec<&Key> {
        let mut keys = Vec::new();
        for (key, _) in &self.creates {
            keys.push(key);
        }
        for (key, _) in &self.updates {
            keys.push(key);
        }
        for key in &self.removes {
            keys.push(key);
        }
        keys
    }
}

#[derive(Debug, Clone)]
pub enum OperationError {
    NotFound(Key),
    AlreadyExists(Key),
    InvalidTarget(String),
    NoParentSection,
    TargetNotFound(Key),
}

impl fmt::Display for OperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperationError::NotFound(key) => write!(f, "Document '{}' not found", key),
            OperationError::AlreadyExists(key) => write!(f, "Document '{}' already exists", key),
            OperationError::InvalidTarget(msg) => write!(f, "Invalid target: {}", msg),
            OperationError::NoParentSection => write!(f, "Cannot extract top-level document section"),
            OperationError::TargetNotFound(key) => write!(f, "Target document '{}' not found", key),
        }
    }
}

impl std::error::Error for OperationError {}
