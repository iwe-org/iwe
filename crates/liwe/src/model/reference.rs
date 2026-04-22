use crate::model::Key;
use crate::model::document::LinkType;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ReferenceType {
    Regular,
    WikiLink,
    WikiLinkPiped,
}

impl ReferenceType {
    pub fn to_link_type(&self) -> LinkType {
        match self {
            ReferenceType::Regular => LinkType::Markdown,
            ReferenceType::WikiLink => LinkType::WikiLink,
            ReferenceType::WikiLinkPiped => LinkType::WikiLinkPiped,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Reference {
    pub key: Key,
    pub text: String,
    pub reference_type: ReferenceType,
}
