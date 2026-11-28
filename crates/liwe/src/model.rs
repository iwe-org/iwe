use std::{collections::HashMap, fmt::Display, ops::Range, path::Path, sync::Arc};

use percent_encoding::percent_decode_str;
use relative_path::RelativePath;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use config::RefsPath;

pub mod config;
pub mod document;
pub mod frontmatter;
pub mod ids;
pub mod inline;
pub mod key_index;
pub mod node;
pub mod node_iter;
pub mod node_pointer;
pub mod projector;
pub mod reference;
pub mod tree;
pub mod tree_iter;
pub mod writer;

pub use document::{
    Attributes, BlockQuote, BulletList, Code, CodeBlock, Div, Document, DocumentBlock,
    DocumentBlocks, DocumentInline, DocumentInlines, Emph, Format, Header, HorizontalRule, Image,
    LineBlock, LineBreak, Link, LinkType, Math, MathType, OrderedList, Para, Plain, RawBlock,
    RawInline, SmallCaps, SoftBreak, Space, Strikeout, Strong, Subscript, Superscript, Target,
    Underline,
};
pub use frontmatter::{prepend_frontmatter, split_raw_frontmatter};
pub use inline::{inlines_to_markdown, to_graph_inlines, to_plain_text, Inline, Inlines};
pub use node::{ColumnAlignment, Node, Reference, ReferenceType};
pub use node_iter::NodeIter;
pub use node_pointer::NodePointer;
pub use projector::Projector;
pub use tree::Tree;
pub use tree_iter::TreeIter;
pub use writer::{blocks_to_markdown, Block};

use writer::frontmatter_to_yaml;

pub type Frontmatter = serde_yaml::Mapping;

pub fn frontmatter_from_str(raw: &str) -> Option<Frontmatter> {
    if raw.trim().is_empty() {
        return Some(Frontmatter::new());
    }
    match serde_yaml::from_str::<serde_yaml::Value>(raw.trim()).ok()? {
        serde_yaml::Value::Mapping(mapping) => Some(mapping),
        _ => None,
    }
}

pub fn frontmatter_to_string(frontmatter: &Frontmatter) -> String {
    if frontmatter.is_empty() {
        return String::new();
    }
    frontmatter_to_yaml(frontmatter)
        .trim_start_matches("---\n")
        .trim_end()
        .to_string()
}

pub type Markdown = String;

pub type MaybeKey = Option<Key>;

pub type Content = String;
pub type State = HashMap<String, String>;

pub type NodeId = i64;
pub type MaybeNodeId = Option<NodeId>;

pub type LineId = i64;
pub type MaybeLineId = Option<LineId>;

pub type StrId = usize;
pub type MaybeStrId = Option<StrId>;

pub type LineNumber = usize;
pub type LineRange = Range<LineNumber>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key {
    text: Arc<str>,
}

impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Key {
    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn parent(&self) -> String {
        RelativePath::new(self.as_str())
            .parent()
            .map(|p| p.to_string())
            .unwrap_or_default()
    }

    pub fn source(&self) -> String {
        RelativePath::new(self.as_str())
            .file_name()
            .unwrap_or("")
            .to_string()
    }

    pub fn path(&self) -> Option<String> {
        RelativePath::new(self.as_str())
            .parent()
            .map(|p| p.to_string())
    }

    pub fn combine(parent: &str, id: &str) -> Key {
        let path = RelativePath::new(parent).join(id).to_string();
        Key {
            text: Arc::from(path),
        }
    }

    pub fn name(name: &str) -> Self {
        Key {
            text: Arc::from(strip_doc_extension(name)),
        }
    }

    pub fn has_hidden_or_empty_segment(&self) -> bool {
        self.text
            .split('/')
            .any(|segment| segment.is_empty() || segment.starts_with('.'))
    }

    pub fn from_stripped(key: &str) -> Self {
        Key {
            text: Arc::from(key),
        }
    }

    pub fn from_rel_link_url(url: &str, relative_to: &str) -> Self {
        let without_fragment = match url.split_once('#') {
            Some((path, _)) => path,
            None => url,
        };
        let decoded = percent_decode_str(without_fragment)
            .decode_utf8_lossy()
            .into_owned();
        Self::from_rel_link_path(strip_doc_extension(&decoded), relative_to)
    }

    pub fn from_rel_document_link(path: &str, relative_to: &str) -> Self {
        let without_fragment = match path.split_once('#') {
            Some((path, _)) => path,
            None => path,
        };
        let decoded = percent_decode_str(without_fragment)
            .decode_utf8_lossy()
            .into_owned();
        Self::from_rel_link_path(&decoded, relative_to)
    }

    fn from_rel_link_path(key: &str, relative_to: &str) -> Self {
        let base = if key.starts_with('/') {
            ""
        } else {
            relative_to
        };
        let path = RelativePath::new(base).join_normalized(key).to_string();
        Key {
            text: Arc::from(path),
        }
    }

    pub fn to_rel_link_url(&self, relative_to: &str) -> String {
        let target = RelativePath::new(self.as_str());
        let Some(file_name) = target.file_name() else {
            return RelativePath::new(relative_to)
                .relative(self.as_str())
                .to_string();
        };
        let target_parent = target.parent().map(|p| p.to_string()).unwrap_or_default();
        let prefix = RelativePath::new(relative_to)
            .relative(&target_parent)
            .to_string();
        if prefix.is_empty() {
            file_name.to_string()
        } else {
            format!("{prefix}/{file_name}")
        }
    }

    pub fn to_workspace_url(&self) -> String {
        self.as_str().to_string()
    }

    #[deprecated(since = "0.25.0", note = "renamed to `to_workspace_url`")]
    pub fn to_library_url(&self) -> String {
        self.to_workspace_url()
    }

    pub fn link_url(&self, relative_to: &str, refs_path: RefsPath) -> String {
        let url = match refs_path {
            RefsPath::Relative => self.to_rel_link_url(relative_to),
            RefsPath::Absolute => format!("/{}", self.to_workspace_url()),
        };
        url.replace('#', "%23")
    }

    pub fn document_link_url(
        &self,
        relative_to: &str,
        refs_path: RefsPath,
        extension: &str,
        format: config::Format,
    ) -> String {
        document_url(&self.link_url(relative_to, refs_path), extension, format)
    }

    pub fn last_url_segment(&self) -> String {
        self.as_str().to_string()
    }

    pub fn to_path(&self, format: config::Format) -> String {
        format!("{}.{}", self.as_str(), format.extension())
    }

    pub fn from_path(path: &Path) -> Option<Key> {
        let name = path.file_name()?.to_string_lossy().to_string();
        Some(Key::name(&name))
    }
}

impl Default for Key {
    fn default() -> Self {
        Key::name("")
    }
}

impl Serialize for Key {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Key {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Key::name(&s))
    }
}

pub fn strip_doc_extension(name: &str) -> &str {
    name.strip_suffix(".md")
        .or_else(|| name.strip_suffix(".dj"))
        .unwrap_or(name)
}

impl From<&str> for Key {
    fn from(value: &str) -> Self {
        Key::name(value)
    }
}

impl From<String> for Key {
    fn from(value: String) -> Self {
        Key::name(&value)
    }
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Default, Hash)]
pub struct Position {
    pub line: usize,
    pub character: usize,
}

impl From<(usize, usize)> for Position {
    fn from(value: (usize, usize)) -> Self {
        Position {
            line: value.0,
            character: value.1,
        }
    }
}

pub type InlineRange = Range<Position>;

pub type NodesMap = Vec<(NodeId, LineRange)>;
pub type DocumentNodesMap = (Key, NodesMap);

pub type Lang = String;
pub type WorkspaceUrl = String;

#[deprecated(since = "0.25.0", note = "renamed to `WorkspaceUrl`")]
pub type LibraryUrl = String;

pub type Level = u8;
pub type Title = String;

pub trait InlinesContext: Copy {
    fn get_ref_title(&self, key: &Key) -> Option<String>;
    fn wiki_display(&self, target: &reference::Target) -> String;
    fn normalize_ref_text(&self) -> bool;
}

pub fn is_ref_url(url: &str) -> bool {
    !has_uri_scheme(url)
}

pub fn is_document_url(url: &str) -> bool {
    let path = url.split_once('#').map(|(path, _)| path).unwrap_or(url);
    let last = path.rsplit('/').next().unwrap_or(path);
    is_ref_url(url)
        && !path.is_empty()
        && !path.ends_with('/')
        && last != "."
        && last != ".."
        && !strip_doc_extension(last).is_empty()
        && (strip_doc_extension(last) != last || !has_file_extension(last))
}

pub(crate) fn has_file_extension(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|ext| ext.chars().any(|c| c.is_ascii_alphabetic()))
        .unwrap_or(false)
}

fn has_uri_scheme(url: &str) -> bool {
    let Some((scheme, _)) = url.split_once(':') else {
        return false;
    };
    scheme.len() >= 2
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
}

const MAILTO_SCHEME: &str = "mailto:";

pub(crate) fn email_autolink_url(address: &str) -> String {
    format!("{}{}", MAILTO_SCHEME, address)
}

pub(crate) fn autolink_body<'a>(url: &'a str, text: &str) -> Option<&'a str> {
    if is_ref_url(url) {
        return None;
    }
    if text.eq_ignore_ascii_case(url) {
        return Some(url);
    }
    url.strip_prefix(MAILTO_SCHEME)
        .filter(|address| text.eq_ignore_ascii_case(address))
}

pub fn document_link_path(url: &str, extension: &str) -> Option<String> {
    if !is_ref_url(url) {
        return None;
    }
    let (path, fragment) = match url.split_once('#') {
        Some((path, fragment)) => (path, Some(fragment)),
        None => (url, None),
    };
    let stripped = path
        .strip_suffix(extension)
        .filter(|stripped| {
            !extension.is_empty() && !stripped.is_empty() && !stripped.ends_with('/')
        })
        .or_else(|| is_document_url(url).then(|| strip_doc_extension(path)))?;
    Some(match fragment {
        Some(fragment) => format!("{}#{}", stripped, fragment),
        None => stripped.to_string(),
    })
}

pub fn document_url(url: &str, extension: &str, format: config::Format) -> String {
    let (path, fragment) = match url.split_once('#') {
        Some((path, fragment)) => (path, Some(fragment)),
        None => (url, None),
    };
    let path = if !extension.is_empty() {
        format!("{}{}", path, extension)
    } else if document_link_path(path, extension).as_deref() == Some(path) {
        path.to_string()
    } else {
        format!("{}.{}", path, format.extension())
    };
    match fragment {
        Some(fragment) => format!("{}#{}", path, fragment),
        None => path,
    }
}

#[cfg(test)]
mod test {
    use super::{
        config, document_link_path, document_url, is_document_url, is_ref_url, Key, RefsPath,
    };

    #[test]
    fn tel_url_is_external() {
        assert!(!is_ref_url("tel:+15551234"));
    }

    #[test]
    fn app_and_transport_schemes_are_external() {
        assert!(!is_ref_url("ftp://example.org/file"));
        assert!(!is_ref_url("file:///home/user/notes.md"));
        assert!(!is_ref_url("obsidian://open?vault=notes"));
        assert!(!is_ref_url("zotero://select/items/1_ABCDEFGH"));
    }

    #[test]
    fn web_schemes_stay_external_regardless_of_case() {
        assert!(!is_ref_url("http://example.org"));
        assert!(!is_ref_url("https://example.org"));
        assert!(!is_ref_url("mailto:someone@example.org"));
        assert!(!is_ref_url("HTTPS://example.org"));
        assert!(!is_ref_url("MailTo:someone@example.org"));
    }

    #[test]
    fn windows_drive_letters_are_internal() {
        assert!(is_ref_url("C:/Users/notes/file.md"));
        assert!(is_ref_url("D:\\notes\\file.md"));
    }

    #[test]
    fn leading_digit_is_not_a_scheme() {
        assert!(is_ref_url("2024: review.md"));
    }

    #[test]
    fn markdown_and_extensionless_urls_are_documents() {
        assert!(is_document_url("file"));
        assert!(is_document_url("file.md"));
        assert!(is_document_url("file.dj"));
        assert!(is_document_url("../dir/file.md#section"));
        assert!(is_document_url("2024.10.08"));
    }

    #[test]
    fn non_document_files_are_not_documents() {
        assert!(!is_document_url("file.yaml"));
        assert!(!is_document_url("../dir/file.pdf"));
        assert!(!is_document_url("image.png#fragment"));
        assert!(!is_document_url("#fragment"));
    }

    #[test]
    fn link_url_encodes_hash_in_key() {
        assert_eq!(
            "C%23 notes",
            Key::name("C# notes").link_url("", RefsPath::Relative)
        );
        assert_eq!(
            "/dir/C%23",
            Key::name("dir/C#").link_url("", RefsPath::Absolute)
        );
    }

    #[test]
    fn fragment_only_url_is_internal() {
        assert!(is_ref_url("#anchor"));
    }

    #[test]
    fn paths_are_internal() {
        assert!(is_ref_url("./file.md"));
        assert!(is_ref_url("file.md"));
        assert!(is_ref_url("/abs/file.md"));
        assert!(is_ref_url("../up/file.md"));
    }

    #[test]
    fn scheme_urls_are_not_document_links() {
        assert_eq!(None, document_link_path("tel:+15551234", ".md"));
    }

    #[test]
    fn refs_extension_is_stripped_from_document_links() {
        assert_eq!(
            Some("folder/file".to_string()),
            document_link_path("folder/file.md", ".md")
        );
        assert_eq!(Some("a.b".to_string()), document_link_path("a.b.md", ".md"));
        assert_eq!(
            Some("a.b#part".to_string()),
            document_link_path("a.b.md#part", ".md")
        );
        assert_eq!(
            Some("a.b".to_string()),
            document_link_path("a.b.html", ".html")
        );
    }

    #[test]
    fn links_without_refs_extension_are_read_by_their_own_extension() {
        assert_eq!(Some("a.b".to_string()), document_link_path("a.b.md", ""));
        assert_eq!(
            Some("a.b#part".to_string()),
            document_link_path("a.b.md#part", "")
        );
        assert_eq!(Some("a".to_string()), document_link_path("a", ".md"));
        assert_eq!(Some("v1.2".to_string()), document_link_path("v1.2", ".md"));
        assert_eq!(None, document_link_path("a.b", ".md"));
        assert_eq!(None, document_link_path("image.png", ""));
        assert_eq!(None, document_link_path("#part", ".md"));
        assert_eq!(None, document_link_path(".md", ".md"));
        assert_eq!(None, document_link_path("dir/.md", ""));
    }

    #[test]
    fn document_url_puts_extension_before_fragment() {
        assert_eq!(
            "a.b.dj#part",
            document_url("a.b#part", "", config::Format::Djot)
        );
        assert_eq!(
            "notes.html#part",
            document_url("notes#part", ".html", config::Format::Markdown)
        );
        assert_eq!(
            "notes#part",
            document_url("notes#part", "", config::Format::Markdown)
        );
    }

    #[test]
    fn document_link_round_trips_for_every_key_and_extension() {
        let keys = [
            "notes",
            "a.b",
            "v1.2",
            "a.b.c",
            "dir/a.b",
            "my page",
            "C#",
            "dir/sub/notes",
            "a.md",
            "dir/a.dj",
        ];
        for format in [config::Format::Markdown, config::Format::Djot] {
            for extension in ["", ".md", ".dj", ".html"] {
                for refs_path in [RefsPath::Relative, RefsPath::Absolute] {
                    for key in keys {
                        let key = Key::from_stripped(key);
                        let url = key.document_link_url("dir", refs_path, extension, format);
                        let path = document_link_path(&url, extension).unwrap_or_default();
                        assert_eq!(
                            (key.clone(), true),
                            (Key::from_rel_document_link(&path, "dir"), !path.is_empty()),
                            "{url} with refs_extension {extension:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn document_link_url_reads_back_as_a_document() {
        let key = Key::name("dir/a.b");
        assert_eq!(
            "a.b.md",
            key.document_link_url("dir", RefsPath::Relative, "", config::Format::Markdown)
        );
        assert_eq!(
            "/dir/a.b.dj",
            key.document_link_url("", RefsPath::Absolute, "", config::Format::Djot)
        );
        assert_eq!(
            "a.b.html",
            key.document_link_url("dir", RefsPath::Relative, ".html", config::Format::Markdown)
        );
        assert_eq!(
            "plain",
            Key::name("plain").document_link_url(
                "",
                RefsPath::Relative,
                "",
                config::Format::Markdown
            )
        );
    }
}
