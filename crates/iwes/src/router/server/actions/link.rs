use chrono::Local;
use liwe::model::config::LinkType as ConfigLinkType;
use liwe::model::Key;
use minijinja::{context, Environment};
use sanitize_filename::sanitize;

use super::{
    string_to_slug, Action, ActionContext, ActionProvider, Change, Changes, Create, Update,
};

pub struct LinkAction {
    pub title: String,
    pub identifier: String,
    pub link_type: Option<ConfigLinkType>,
    pub key_template: String,
    pub key_date_format: String,
}

impl LinkAction {
    fn format_target_key(
        &self,
        context: &impl ActionContext,
        id: &str,
        _parent_key: &str,
        word: &str,
    ) -> Key {
        let date = Local::now().date_naive();
        let formatted = date.format(&self.key_date_format).to_string();
        let slug = string_to_slug(word);

        let relative_key = Environment::new()
            .template_from_str(&self.key_template)
            .expect("correct template")
            .render(context! {
                today => formatted,
                id => id.to_string(),
                title => sanitize(word),
                slug => slug,
            })
            .expect("template to work");

        let base_key = Key::name(&relative_key);

        std::iter::successors(Some((base_key.clone(), 1)), |(key, counter)| {
            context.key_exists(key).then(|| {
                let suffixed_name = format!("{}-{}", base_key.to_string(), counter);
                (Key::name(&suffixed_name), counter + 1)
            })
        })
        .last()
        .map(|(key, _)| key)
        .unwrap_or(base_key)
    }

    fn extract_word_at_cursor(line_text: &str, character_pos: u32) -> Option<(String, u32, u32)> {
        let char_pos = character_pos as usize;

        (char_pos <= line_text.len()).then(|| {
            let bytes = line_text.as_bytes();

            let start = (0..char_pos)
                .rev()
                .take_while(|&i| Self::is_word_char(bytes[i]))
                .last()
                .unwrap_or(char_pos);

            let end = (char_pos..bytes.len())
                .take_while(|&i| Self::is_word_char(bytes[i]))
                .last()
                .map(|i| i + 1)
                .unwrap_or(char_pos);

            (start != end)
                .then(|| line_text[start..end].to_string())
                .filter(|word| !word.trim().is_empty())
                .map(|word| (word, start as u32, end as u32))
        })?
    }

    fn is_word_char(c: u8) -> bool {
        c.is_ascii_alphanumeric() || c == b'_' || c == b'-' || (c >= 128)
    }

    fn replace_word_with_link(
        line_text: &str,
        word: &str,
        start_pos: u32,
        end_pos: u32,
        new_key: &Key,
        link_type: Option<&ConfigLinkType>,
    ) -> String {
        let (start, end) = (start_pos as usize, end_pos as usize);

        let link_text = match link_type {
            Some(ConfigLinkType::WikiLink) => format!("[[{}]]", new_key),
            Some(ConfigLinkType::Markdown) | None => format!("[{}]({})", word, new_key),
        };

        format!("{}{}{}", &line_text[..start], link_text, &line_text[end..])
    }
}

impl ActionProvider for LinkAction {
    fn identifier(&self) -> String {
        format!("custom.{}", self.identifier)
    }

    fn action(
        &self,
        key: super::Key,
        selection: super::TextRange,
        context: impl ActionContext,
    ) -> Option<Action> {
        (selection.start.line == selection.end.line).then(|| {
            let document = context.get_document_markdown(&key)?;
            let lines: Vec<&str> = document.lines().collect();
            let target_line = selection.start.line as usize;

            let line_text = lines.get(target_line)?;

            Self::extract_word_at_cursor(line_text, selection.start.character)?;

            Some(Action {
                title: self.title.clone(),
                identifier: self.identifier(),
                key: key.clone(),
                range: selection.clone(),
            })
        })?
    }

    fn changes(
        &self,
        key: super::Key,
        selection: super::TextRange,
        context: impl ActionContext,
    ) -> Option<Changes> {
        (selection.start.line == selection.end.line).then(|| {
            let document = context.get_document_markdown(&key)?;
            let lines: Vec<&str> = document.lines().collect();
            let target_line = selection.start.line as usize;

            let line_text = lines.get(target_line)?;

            let (word, start, end) =
                Self::extract_word_at_cursor(line_text, selection.start.character)?;

            let id = context
                .unique_ids(&key.parent(), 1)
                .first()
                .expect("to have one")
                .to_string();
            let new_key = self.format_target_key(&context, &id, &key.parent(), &word);

            let new_markdown = format!("# {}\n", word);

            let updated_line = Self::replace_word_with_link(
                line_text,
                &word,
                start,
                end,
                &new_key,
                self.link_type.as_ref(),
            );

            let updated_markdown = lines
                .iter()
                .enumerate()
                .map(|(i, &line)| {
                    if i == target_line {
                        updated_line.as_str()
                    } else {
                        line
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
                + "\n";

            Some(vec![
                Change::Create(Create {
                    key: new_key.clone(),
                }),
                Change::Update(Update {
                    key: new_key,
                    markdown: new_markdown,
                }),
                Change::Update(Update {
                    key: key,
                    markdown: updated_markdown,
                }),
            ])
        })?
    }
}
