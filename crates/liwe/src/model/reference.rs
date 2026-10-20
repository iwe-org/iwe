use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::model::document::LinkType;
use crate::model::inline::{text_to_inlines, to_plain_text};
use crate::model::{Inlines, Key};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
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

    pub fn is_wiki(&self) -> bool {
        matches!(self, ReferenceType::WikiLink | ReferenceType::WikiLinkPiped)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Target {
    Path {
        #[serde(
            serialize_with = "serialize_exact_key",
            deserialize_with = "deserialize_exact_key"
        )]
        key: Key,
        fragment: Option<String>,
    },
    Name(String),
}

fn serialize_exact_key<S: Serializer>(key: &Key, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(key.as_str())
}

fn deserialize_exact_key<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Key, D::Error> {
    let text = String::deserialize(deserializer)?;
    Ok(Key::from_stripped(&text))
}

impl Target {
    pub fn path(key: Key) -> Target {
        Target::Path {
            key,
            fragment: None,
        }
    }

    pub fn name(written: &str) -> Target {
        Target::Name(written.to_string())
    }

    pub fn for_key(key: &Key, reference_type: ReferenceType) -> Target {
        if reference_type.is_wiki() {
            Target::Name(key.to_workspace_url())
        } else {
            Target::path(key.clone())
        }
    }

    pub fn from_link_url(url: &str, relative_to: &str, reference_type: ReferenceType) -> Target {
        if reference_type.is_wiki() {
            return Target::name(url);
        }
        Target::Path {
            key: Key::from_rel_link_url(url, relative_to),
            fragment: url
                .split_once('#')
                .map(|(_, fragment)| fragment.to_string()),
        }
    }

    pub fn path_key(&self) -> Option<&Key> {
        match self {
            Target::Path { key, .. } => Some(key),
            Target::Name(_) => None,
        }
    }

    pub fn written_name(&self) -> Option<&str> {
        match self {
            Target::Name(written) => Some(written),
            Target::Path { .. } => None,
        }
    }

    pub fn fragment(&self) -> Option<&str> {
        match self {
            Target::Path { fragment, .. } => fragment.as_deref(),
            Target::Name(_) => None,
        }
    }

    pub fn unresolved_url(&self) -> String {
        match self {
            Target::Path { key, .. } => key.to_workspace_url(),
            Target::Name(written) => written.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Reference {
    pub target: Target,
    pub inlines: Inlines,
    pub reference_type: ReferenceType,
    pub display_url: Option<String>,
}

impl Reference {
    pub fn plain(target: Target, text: &str, reference_type: ReferenceType) -> Reference {
        Reference {
            target,
            inlines: text_to_inlines(text),
            reference_type,
            display_url: None,
        }
    }

    pub fn to_key(key: Key, text: &str) -> Reference {
        Reference::plain(Target::path(key), text, ReferenceType::Regular)
    }

    pub fn text(&self) -> String {
        to_plain_text(&self.inlines)
    }
}
