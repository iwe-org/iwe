use std::str::FromStr;

use lsp_types::*;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use relative_path::RelativePath;
use url::Url;

use liwe::model::Key;

pub struct BasePath {
    base_path: String,
}

impl BasePath {
    pub fn new(base_path: String) -> Self {
        Self { base_path }
    }

    pub fn key_to_url(&self, key: &Key) -> Uri {
        let base_url = Url::parse(&self.base_path).expect("valid base URL");
        let mut url = base_url.clone();
        url.set_path(&format!("{}{}", base_url.path(), key.to_path()));
        Uri::from_str(url.as_str()).expect("to work")
    }

    pub fn relative_to_full_path(&self, url: &str) -> Uri {
        let mut base_url = Url::parse(&self.base_path).expect("valid base URL");
        let filename = format!("{}.md", url.trim_end_matches(".md"));
        base_url.set_path(&format!("{}{}", base_url.path(), filename));
        Uri::from_str(base_url.as_str()).expect("to work")
    }

    pub fn name_to_url(&self, key: &str) -> Uri {
        let mut base_url = Url::parse(&self.base_path).expect("valid base URL");
        base_url.set_path(&format!("{}{}.md", base_url.path(), key));
        Uri::from_str(base_url.as_str()).expect("to work")
    }

    pub fn url_to_key(&self, url: &Uri) -> Key {
        Key::name(
            &url.to_string()
                .trim_start_matches(&self.base_path)
                .to_string(),
        )
    }

    pub fn resolve_relative_url(&self, url: &str, relative_to: &str) -> Uri {
        // URL-encode the path to handle spaces and special characters
        let encoded_url = utf8_percent_encode(url, NON_ALPHANUMERIC).to_string();
        let relative_url = RelativePath::new(relative_to).join(encoded_url).to_string();
        self.relative_to_full_path(&relative_url)
    }
}
