pub const QUERY_SCHEMA_DRAFTS: [(&str, &str); 1] =
    [("2026-08", include_str!("schemas/draft-2026-08.json"))];

pub const CURRENT_QUERY_SCHEMA_DRAFT: &str = "2026-08";

pub fn query_schema_uri(draft: &str) -> String {
    format!("https://iwe.md/schemas/query/draft/{}/schema", draft)
}

pub fn query_schema(draft: &str) -> Option<&'static str> {
    QUERY_SCHEMA_DRAFTS
        .iter()
        .find(|(name, _)| *name == draft)
        .map(|(_, body)| *body)
}

pub fn current_query_schema() -> &'static str {
    query_schema(CURRENT_QUERY_SCHEMA_DRAFT).expect("the current draft is embedded")
}
