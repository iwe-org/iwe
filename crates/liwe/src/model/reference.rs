use crate::model::document::LinkType;
use crate::model::inline::{text_to_inlines, to_plain_text};
use crate::model::{Inlines, Key};

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
    pub inlines: Inlines,
    pub reference_type: ReferenceType,
    pub url: String,
    pub display_url: Option<String>,
}

impl Reference {
    pub fn plain(key: Key, text: &str, reference_type: ReferenceType, url: String) -> Reference {
        Reference {
            key,
            inlines: text_to_inlines(text),
            reference_type,
            url,
            display_url: None,
        }
    }

    pub fn text(&self) -> String {
        to_plain_text(&self.inlines)
    }
}
