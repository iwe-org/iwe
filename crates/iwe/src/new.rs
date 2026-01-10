use std::io::{self, Read};
use std::path::PathBuf;

use chrono::Local;
use minijinja::{context, Environment};
use rand::distr::Alphanumeric;
use rand::Rng;

use liwe::model::config::{Configuration, NoteTemplate, DEFAULT_KEY_DATE_FORMAT};
use liwe::model::Key;

#[derive(Debug, Clone, Default, clap::ValueEnum)]
pub enum IfExists {
    #[default]
    Suffix,
    Override,
    Skip,
}

fn get_default_template() -> NoteTemplate {
    NoteTemplate {
        key_template: "{{slug}}".to_string(),
        document_template: "# {{title}}\n\n{{content}}".to_string(),
    }
}

pub struct DocumentCreator<'a> {
    config: &'a Configuration,
    library_path: PathBuf,
}

pub struct CreateOptions {
    pub title: String,
    pub template_name: Option<String>,
    pub content: Option<String>,
    pub if_exists: IfExists,
}

pub struct CreatedDocument {
    pub path: PathBuf,
}

impl<'a> DocumentCreator<'a> {
    pub fn new(config: &'a Configuration, library_path: PathBuf) -> Self {
        Self {
            config,
            library_path,
        }
    }

    fn find_available_key(&self, base_key: &Key) -> Key {
        let mut candidate_key = base_key.clone();
        let mut counter = 1;

        while self.library_path.join(candidate_key.to_path()).exists() {
            let suffixed_name = format!("{}-{}", base_key, counter);
            candidate_key = Key::name(&suffixed_name);
            counter += 1;
        }

        candidate_key
    }

    pub fn create(&self, options: CreateOptions) -> Result<Option<CreatedDocument>, String> {
        let template_name = options
            .template_name
            .or_else(|| self.config.library.default_template.clone())
            .unwrap_or_else(|| "default".to_string());

        let fallback_template = get_default_template();
        let template = self
            .config
            .templates
            .get(&template_name)
            .or_else(|| {
                if template_name == "default" {
                    Some(&fallback_template)
                } else {
                    None
                }
            })
            .ok_or_else(|| format!("Template '{}' not found in configuration", template_name))?;

        let content = options.content.unwrap_or_default();

        let key_date_format = self
            .config
            .library
            .date_format
            .clone()
            .unwrap_or_else(|| DEFAULT_KEY_DATE_FORMAT.to_string());

        let markdown_date_format = self
            .config
            .markdown
            .date_format
            .clone()
            .unwrap_or_else(|| "%b %d, %Y".to_string());

        let date = Local::now().date_naive();
        let key_today = date.format(&key_date_format).to_string();
        let markdown_today = date.format(&markdown_date_format).to_string();

        let slug = string_to_slug(&options.title);
        let id = generate_random_id();

        let relative_key = render_template(
            &template.key_template,
            &options.title,
            &slug,
            &key_today,
            &id,
            &content,
        )?;

        let document_content = render_template(
            &template.document_template,
            &options.title,
            &slug,
            &markdown_today,
            &id,
            &content,
        )?;

        let base_key = Key::name(&relative_key);
        let file_path = self.library_path.join(base_key.to_path());
        let file_exists = file_path.exists();

        let final_key = match options.if_exists {
            IfExists::Skip if file_exists => return Ok(None),
            IfExists::Suffix => self.find_available_key(&base_key),
            IfExists::Override | IfExists::Skip => base_key,
        };

        let file_path = self.library_path.join(final_key.to_path());

        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent directories: {}", e))?;
            }
        }

        std::fs::write(&file_path, document_content)
            .map_err(|e| format!("Failed to write file: {}", e))?;

        let absolute_path = file_path.canonicalize().unwrap_or(file_path);

        Ok(Some(CreatedDocument {
            path: absolute_path,
        }))
    }
}

pub fn read_stdin_if_available() -> String {
    use std::io::IsTerminal;

    if std::io::stdin().is_terminal() {
        return String::new();
    }

    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer).unwrap_or_default();
    buffer
}

fn render_template(
    template_str: &str,
    title: &str,
    slug: &str,
    today: &str,
    id: &str,
    content: &str,
) -> Result<String, String> {
    Environment::new()
        .template_from_str(template_str)
        .map_err(|e| format!("Invalid template syntax: {}", e))?
        .render(context! {
            title => title,
            slug => slug,
            today => today,
            id => id,
            content => content,
        })
        .map_err(|e| format!("Template rendering failed: {}", e))
}

fn generate_random_id() -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect::<String>()
        .to_lowercase()
}

fn string_to_slug(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
