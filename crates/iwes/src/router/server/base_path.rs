use std::str::FromStr;

use lsp_types::*;
use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};
use relative_path::RelativePath;
use url::Url;

use liwe::model::Key;

pub struct BasePath {
    url: Url,
}

impl BasePath {
    pub fn new(base_path: String) -> Self {
        Self {
            url: Url::parse(&base_path).expect("valid base URL"),
        }
    }

    pub fn from_path(path: &str) -> Self {
        Self {
            url: Url::from_directory_path(path).expect("valid base path"),
        }
    }

    pub fn key_to_url(&self, key: &Key) -> Uri {
        self.join(&key.to_path())
    }

    pub fn relative_to_full_path(&self, path: &str) -> Uri {
        let filename = path.trim_end_matches(".md");
        self.join(&[filename, ".md"].concat())
    }

    pub fn name_to_url(&self, key: &str) -> Uri {
        self.join(&[key, ".md"].concat())
    }

    pub fn url_to_key(&self, url: &Uri) -> Key {
        let base = self.url.as_str();
        let relative_path = url.to_string().trim_start_matches(base).to_string();
        let decoded_path = percent_decode_str(&relative_path)
            .decode_utf8()
            .unwrap_or_else(|_| relative_path.as_str().into())
            .to_string();
        Key::name(&decoded_path)
    }

    pub fn resolve_relative_url(&self, url: &str, relative_to: &str) -> Uri {
        let encoded_url = utf8_percent_encode(url, NON_ALPHANUMERIC).to_string();
        let relative_url = RelativePath::new(relative_to).join(encoded_url).to_string();
        self.relative_to_full_path(&relative_url)
    }

    fn join(&self, path: &str) -> Uri {
        let url = self.url.join(path).expect("valid path");
        Uri::from_str(url.as_str()).expect("valid URI")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_to_key_with_danish_characters() {
        let base_path = BasePath::new("file:///basepath/".to_string());
        let uri = Uri::from_str("file:///basepath/t%C3%B8j.md").unwrap();
        let key = base_path.url_to_key(&uri);
        assert_eq!(key.to_string(), "tøj");
    }

    #[test]
    fn test_url_to_key_with_regular_characters() {
        let base_path = BasePath::new("file:///basepath/".to_string());

        let uri = Uri::from_str("file:///basepath/regular.md").unwrap();
        let key = base_path.url_to_key(&uri);
        assert_eq!(key.to_string(), "regular");
    }

    #[test]
    fn test_url_to_key_with_spaces_in_base_path() {
        let base_path = BasePath::from_path("/path with spaces/docs");
        let uri = Uri::from_str("file:///path%20with%20spaces/docs/test.md").unwrap();
        let key = base_path.url_to_key(&uri);
        assert_eq!(key.to_string(), "test");
    }

    #[test]
    fn test_key_to_url_with_spaces_in_base_path() {
        let base_path = BasePath::from_path("/path with spaces/docs");
        let key = liwe::model::Key::name("test.md");
        let url = base_path.key_to_url(&key);
        assert_eq!(
            url.to_string(),
            "file:///path%20with%20spaces/docs/test.md"
        );
    }

    #[test]
    fn test_url_to_key_with_spaces_in_key() {
        let base_path = BasePath::from_path("/basepath");
        let uri = Uri::from_str("file:///basepath/my%20document.md").unwrap();
        let key = base_path.url_to_key(&uri);
        assert_eq!(key.to_string(), "my document");
    }

    #[test]
    fn test_key_to_url_with_spaces_in_key() {
        let base_path = BasePath::from_path("/basepath");
        let key = liwe::model::Key::name("my document.md");
        let url = base_path.key_to_url(&key);
        assert_eq!(
            url.to_string(),
            "file:///basepath/my%20document.md"
        );
    }
}
