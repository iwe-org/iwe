use std::fmt::{self, Debug, Formatter};

use crate::model::inline::{decode_inlines, encode_inlines, to_plain_text, Inlines};
use crate::model::reference::Target;
use crate::model::InlinesContext;

#[derive(Clone, PartialEq)]
pub struct Line {
    bytes: Box<[u8]>,
}

impl Line {
    pub fn new(inlines: Inlines) -> Line {
        Line {
            bytes: encode_inlines(&inlines),
        }
    }

    pub fn to_plain_text(&self) -> String {
        to_plain_text(&self.inlines())
    }

    pub fn inlines(&self) -> Inlines {
        decode_inlines(&self.bytes)
    }

    pub fn targets(&self) -> Vec<Target> {
        self.inlines().iter().flat_map(|i| i.targets()).collect()
    }

    pub fn normalize(&self, context: impl InlinesContext) -> Inlines {
        self.inlines()
            .iter()
            .map(|i| i.normalize(context))
            .collect()
    }
}

impl Debug for Line {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Line")
            .field("inlines", &self.inlines())
            .finish()
    }
}

#[cfg(test)]
mod test {
    use super::Line;
    use crate::model::inline::{prepend_checkbox, to_plain_text, Inline, Inlines};
    use crate::model::reference::{Reference, ReferenceType, Target};
    use crate::model::Key;

    fn str(text: &str) -> Inline {
        Inline::Str(text.to_string())
    }

    fn reference() -> Inline {
        Inline::Reference(Reference {
            target: Target::path(Key::name("key")),
            inlines: vec![str("title")],
            reference_type: ReferenceType::Regular,
            display_url: None,
        })
    }

    fn cases() -> Vec<Inlines> {
        vec![
            vec![
                str("first"),
                Inline::Space,
                str("second"),
                Inline::Space,
                str("third"),
            ],
            vec![
                str("first"),
                Inline::Space,
                reference(),
                Inline::Space,
                str("second"),
            ],
            prepend_checkbox(Some(true), vec![str("first"), Inline::Space, str("second")]),
            Inline::from_string("first second"),
            vec![str("[ "), str("first"), Inline::Space, str("second")],
            vec![str("")],
            vec![str("first"), Inline::Space, Inline::Space, str("second")],
            vec![str("first"), Inline::SoftBreak, str("second")],
            Inlines::new(),
        ]
    }

    #[test]
    fn inlines_round_trip() {
        for inlines in cases() {
            assert_eq!(Line::new(inlines.clone()).inlines(), inlines);
        }
    }

    #[test]
    fn plain_text_and_targets_match_original() {
        for inlines in cases() {
            let line = Line::new(inlines.clone());
            let targets: Vec<Target> = inlines.iter().flat_map(|i| i.targets()).collect();
            assert_eq!(
                (line.to_plain_text(), line.targets()),
                (to_plain_text(&inlines), targets)
            );
        }
    }

    #[test]
    fn debug_prints_decoded_inlines() {
        assert_eq!(
            format!(
                "{:?}",
                Line::new(vec![str("first"), Inline::Space, str("second")])
            ),
            r#"Line { inlines: [Str("first"), Space, Str("second")] }"#
        );
    }

    #[test]
    fn empty_line_is_one_byte() {
        let line = Line::new(Inlines::new());
        assert_eq!(line.bytes.as_ref(), &[0u8]);
        assert_eq!(line.inlines(), Inlines::new());
    }
}
