use std::str::FromStr;

use lsp_types::*;
use percent_encoding::percent_decode_str;
use url::Url;

use liwe::model::Key;

pub struct BasePath {
    url: Url,
}

impl BasePath {
    pub fn new(base_path: String) -> Self {
        let url = Url::parse(&base_path).expect("valid base URL");
        Self {
            url: canonical(url),
        }
    }

    pub fn from_path(path: &str) -> Self {
        let url = Url::from_directory_path(path).expect("valid base path");
        Self {
            url: canonical(url),
        }
    }

    pub fn key_to_url(&self, key: &Key) -> Uri {
        self.build_url(&key.relative_path)
    }

    pub fn relative_to_full_path(&self, path: &str) -> Uri {
        self.build_url(path)
    }

    pub fn name_to_url(&self, name: &str) -> Uri {
        self.build_url(name)
    }

    pub fn url_to_key(&self, uri: &Uri) -> Key {
        let url = canonical(Url::parse(&uri.to_string()).expect("valid URI"));
        let base = self.url.to_file_path().expect("base is a file URL");
        let target = url.to_file_path().expect("URI is a file URL");
        let relative = target.strip_prefix(&base).unwrap_or(&target);

        let joined = relative
            .components()
            .filter_map(|c| match c {
                std::path::Component::Normal(os) => Some(os.to_string_lossy().to_string()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("/");

        Key::name(&joined)
    }

    pub fn resolve_relative_url(&self, link: &str, relative_to: &str) -> Uri {
        let mut source = self.url.clone();
        {
            let mut segs = source.path_segments_mut().expect("path-based URL");
            segs.pop_if_empty();
            for s in relative_to.split('/').filter(|s| !s.is_empty()) {
                segs.push(s);
            }
            segs.push("");
        }

        let mut resolved = source.join(link).expect("valid link");

        let last = resolved
            .path_segments()
            .and_then(|s| s.last())
            .unwrap_or("");
        if !last.is_empty() && !last.ends_with(".md") {
            let decoded = percent_decode_str(last).decode_utf8_lossy().into_owned();
            resolved
                .path_segments_mut()
                .expect("path-based URL")
                .pop()
                .push(&format!("{}.md", decoded));
        }

        Uri::from_str(resolved.as_str()).expect("valid URI")
    }

    fn build_url(&self, key_or_path: &str) -> Uri {
        let trimmed = key_or_path.trim_end_matches(".md");
        let mut url = self.url.clone();
        {
            let mut segs = url.path_segments_mut().expect("path-based URL");
            segs.pop_if_empty();
            let parts: Vec<&str> = trimmed.split('/').filter(|s| !s.is_empty()).collect();
            if let Some((last, rest)) = parts.split_last() {
                segs.extend(rest);
                segs.push(&format!("{}.md", last));
            }
        }
        Uri::from_str(url.as_str()).expect("valid URI")
    }
}

fn canonical(url: Url) -> Url {
    if url.scheme() != "file" {
        return url;
    }
    let was_directory = url.path().ends_with('/');
    let Ok(path) = url.to_file_path() else {
        return lowercase_url_drive(url);
    };
    let result = if was_directory {
        Url::from_directory_path(&path)
    } else {
        Url::from_file_path(&path)
    };
    lowercase_url_drive(result.unwrap_or(url))
}

fn lowercase_url_drive(mut url: Url) -> Url {
    let path = url.path();
    let bytes = path.as_bytes();
    if bytes.len() >= 3 && bytes[0] == b'/' && bytes[1].is_ascii_uppercase() && bytes[2] == b':' {
        let lowered = format!("/{}{}", (bytes[1] as char).to_ascii_lowercase(), &path[2..]);
        url.set_path(&lowered);
    }
    url
}

#[cfg(test)]
mod tests {
    use super::*;

    fn abs_path(rel: &str) -> String {
        if cfg!(windows) {
            format!("C:/{rel}")
        } else {
            format!("/{rel}")
        }
    }

    fn file_url(rel: &str) -> String {
        if cfg!(windows) {
            format!("file:///c:/{rel}")
        } else {
            format!("file:///{rel}")
        }
    }

    #[test]
    fn test_url_to_key_with_danish_characters() {
        let base_path = BasePath::new(file_url("basepath/"));
        let uri = Uri::from_str(&file_url("basepath/t%C3%B8j.md")).unwrap();
        let key = base_path.url_to_key(&uri);
        assert_eq!(key.to_string(), "tøj");
    }

    #[test]
    fn test_url_to_key_with_regular_characters() {
        let base_path = BasePath::new(file_url("basepath/"));

        let uri = Uri::from_str(&file_url("basepath/regular.md")).unwrap();
        let key = base_path.url_to_key(&uri);
        assert_eq!(key.to_string(), "regular");
    }

    #[test]
    fn test_url_to_key_with_spaces_in_base_path() {
        let base_path = BasePath::from_path(&abs_path("path with spaces/docs"));
        let uri = Uri::from_str(&file_url("path%20with%20spaces/docs/test.md")).unwrap();
        let key = base_path.url_to_key(&uri);
        assert_eq!(key.to_string(), "test");
    }

    #[test]
    fn test_key_to_url_with_spaces_in_base_path() {
        let base_path = BasePath::from_path(&abs_path("path with spaces/docs"));
        let key = liwe::model::Key::name("test.md");
        let url = base_path.key_to_url(&key);
        assert_eq!(
            url.to_string(),
            file_url("path%20with%20spaces/docs/test.md")
        );
    }

    #[test]
    fn test_url_to_key_with_spaces_in_key() {
        let base_path = BasePath::from_path(&abs_path("basepath"));
        let uri = Uri::from_str(&file_url("basepath/my%20document.md")).unwrap();
        let key = base_path.url_to_key(&uri);
        assert_eq!(key.to_string(), "my document");
    }

    #[test]
    fn test_key_to_url_with_spaces_in_key() {
        let base_path = BasePath::from_path(&abs_path("basepath"));
        let key = liwe::model::Key::name("my document.md");
        let url = base_path.key_to_url(&key);
        assert_eq!(url.to_string(), file_url("basepath/my%20document.md"));
    }

    #[test]
    fn canonical_lowercases_drive_letter_and_preserves_other_segments() {
        let url = Url::parse("file:///C:/Base/Note.md").unwrap();
        assert_eq!(canonical(url).as_str(), "file:///c:/Base/Note.md");
    }

    #[test]
    fn test_url_to_key_windows_case_and_percent_encoding_mismatch() {
        let base_path = BasePath::new("file:///C:/base/".to_string());
        let uri = Uri::from_str("file:///c%3A/base/one.md").unwrap();
        let key = base_path.url_to_key(&uri);
        assert_eq!(key.to_string(), "one");
    }

    #[test]
    fn test_resolve_relative_url_doubles_md_suffix_when_link_has_md_extension() {
        let base_path = BasePath::from_path(&abs_path("basepath"));
        let url = base_path.resolve_relative_url("one.md", "");
        assert_eq!(url.to_string(), file_url("basepath/one.md"));
    }

    #[test]
    fn test_name_to_url_with_md_suffix_in_name() {
        let base_path = BasePath::from_path(&abs_path("basepath"));
        let url = base_path.name_to_url("one.md");
        assert_eq!(url.to_string(), file_url("basepath/one.md"));
    }

    #[test]
    fn test_resolve_relative_url_with_anchor_fragment() {
        let base_path = BasePath::from_path(&abs_path("basepath"));
        let url = base_path.resolve_relative_url("one#section", "");
        assert_eq!(url.to_string(), file_url("basepath/one.md#section"));
    }

    #[test]
    fn test_from_rel_link_url_decodes_percent_encoded_spaces() {
        let key = liwe::model::Key::from_rel_link_url("a%20b.md", "");
        assert_eq!(key.to_string(), "a b");
    }

    #[test]
    fn test_url_to_key_matches_parsed_reference_key() {
        let base_path = BasePath::new("file:///C:/base/".to_string());

        let source_uri = Uri::from_str("file:///c%3A/base/one.md").unwrap();
        let target_uri = Uri::from_str("file:///c%3A/base/two.md").unwrap();

        let source_key = base_path.url_to_key(&source_uri);
        let target_key = base_path.url_to_key(&target_uri);

        let parsed_target_key = liwe::model::Key::from_rel_link_url("two", &source_key.parent());

        assert_eq!(parsed_target_key.to_string(), target_key.to_string());
    }
}
