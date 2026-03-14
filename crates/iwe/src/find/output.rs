use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ParentDocumentInfo {
    pub key: String,
    pub title: String,
    pub section_path: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FindResult {
    pub key: String,
    pub title: String,
    pub is_root: bool,
    pub incoming_refs: usize,
    pub outgoing_refs: usize,
    pub parent_documents: Vec<ParentDocumentInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FindOutput {
    pub query: Option<String>,
    pub limit: Option<usize>,
    pub total: usize,
    pub results: Vec<FindResult>,
}
