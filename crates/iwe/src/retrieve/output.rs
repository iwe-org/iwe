use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ParentDocumentInfo {
    pub key: String,
    pub title: String,
    pub section_path: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BacklinkInfo {
    pub key: String,
    pub title: String,
    pub section_path: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChildDocumentInfo {
    pub key: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocumentOutput {
    pub key: String,
    pub title: String,
    pub content: String,
    pub parent_documents: Vec<ParentDocumentInfo>,
    pub child_documents: Vec<ChildDocumentInfo>,
    pub backlinks: Vec<BacklinkInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RetrieveOutput {
    pub documents: Vec<DocumentOutput>,
}
