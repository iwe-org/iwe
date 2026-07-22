use std::io::{BufRead, Write};
use std::path::Path;

use crate::init::evidence::Evidence;
use crate::init::fit::measure;
use crate::init::report::summary_line;
use crate::init::settings::{to_configuration, Confidence, SettingId, Settings, ALL_SETTINGS};

pub enum Outcome {
    Write(Settings),
    Quit,
}

pub fn run(
    root: &Path,
    evidence: &Evidence,
    detected: &Settings,
    defaults: &Settings,
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> Outcome {
    let decisions: Vec<SettingId> = detected
        .differing(defaults)
        .into_iter()
        .filter(|id| detected.confidence(*id) != Confidence::Overridden)
        .collect();

    if decisions.is_empty() {
        let _ = writeln!(output, "{}", summary_line(evidence));
        let _ = writeln!(
            output,
            "detection matches the iwe defaults — nothing to choose"
        );
        return Outcome::Write(detected.clone());
    }

    draw(root, evidence, detected, defaults, &decisions, output);

    loop {
        let mut line = String::new();
        if input.read_line(&mut line).unwrap_or(0) == 0 {
            let _ = writeln!(output);
            return Outcome::Quit;
        }

        match line.trim() {
            "" | "y" => return Outcome::Write(detected.clone()),
            "n" => {
                let mut chosen = detected.clone();
                for id in &decisions {
                    chosen.adopt(*id, defaults);
                }
                return Outcome::Write(chosen);
            }
            "q" => return Outcome::Quit,
            _ => {
                let _ = writeln!(output, "answer y, n or q");
                prompt(output);
            }
        }
    }
}

fn draw(
    root: &Path,
    evidence: &Evidence,
    detected: &Settings,
    defaults: &Settings,
    decisions: &[SettingId],
    output: &mut impl Write,
) {
    let _ = writeln!(output);
    let _ = writeln!(output, "{}", summary_line(evidence));
    let _ = writeln!(output);
    let _ = writeln!(output, "    {:<18} {:<20} DEFAULT:", "", "DETECTED:");

    for id in ALL_SETTINGS {
        let on_offer = decisions.contains(&id);
        let mut detected_cell = detected.get(id).to_string();
        if detected.is_mixed(id) {
            detected_cell.push_str(" ?");
        }
        let default_cell = if on_offer {
            defaults.get(id).to_string()
        } else {
            String::new()
        };
        let _ = writeln!(
            output,
            "  {} {:<18} {:<20} {}",
            if on_offer { "❯" } else { " " },
            id.label(),
            detected_cell,
            default_cell
        );
    }

    let _ = writeln!(output);
    let _ = writeln!(
        output,
        "    {:<18} {:<20} {}",
        "normalize",
        measure(root, &to_configuration(detected)).render(),
        measure(root, &to_configuration(defaults)).render()
    );
    let _ = writeln!(output);
    prompt(output);
}

fn prompt(output: &mut impl Write) {
    let _ = writeln!(
        output,
        "write the detected settings? ⏎/y detected · n defaults · q quit"
    );
    let _ = write!(output, "> ");
    let _ = output.flush();
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::path::Path;

    use super::{run, Outcome};
    use crate::init::evidence::Evidence;
    use crate::init::settings::{defaults, Confidence, SettingId, Settings, Value};

    fn bundles() -> (Settings, Settings) {
        let base = defaults();
        let mut detected = base.clone();
        detected.set(
            SettingId::LibraryPath,
            Value::text("notes"),
            Confidence::Detected,
            "3 of 3 files live under notes/",
        );
        detected.set(
            SettingId::LinkFormat,
            Value::text("wiki"),
            Confidence::Detected,
            "3 wiki links vs 0 markdown links",
        );
        (detected, base)
    }

    fn drive(input: &str) -> (Option<Settings>, String) {
        let (detected, base) = bundles();
        drive_bundles(input, &detected, &base)
    }

    fn drive_bundles(
        input: &str,
        detected: &Settings,
        base: &Settings,
    ) -> (Option<Settings>, String) {
        let mut reader = Cursor::new(input.as_bytes().to_vec());
        let mut output = Vec::new();

        let outcome = run(
            Path::new("/iwe-nonexistent-corpus"),
            &Evidence::default(),
            detected,
            base,
            &mut reader,
            &mut output,
        );

        let selected = match outcome {
            Outcome::Write(settings) => Some(settings),
            Outcome::Quit => None,
        };

        (
            selected,
            String::from_utf8(output).expect("valid UTF-8 output"),
        )
    }

    fn rows_of(transcript: &str) -> Vec<String> {
        transcript
            .lines()
            .map(|line| {
                line.split_whitespace()
                    .filter(|token| *token != "❯")
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .filter(|line| {
                line.starts_with("library")
                    || line.starts_with("links")
                    || line.starts_with("format")
            })
            .collect()
    }

    #[test]
    fn empty_line_writes_the_detected_bundle() {
        let (selected, _) = drive("\n");
        let selected = selected.expect("screen writes on Enter");

        assert_eq!(Value::text("notes"), selected.get(SettingId::LibraryPath));
        assert_eq!(Value::text("wiki"), selected.get(SettingId::LinkFormat));
    }

    #[test]
    fn yes_writes_the_detected_bundle() {
        let (selected, _) = drive("y\n");
        let selected = selected.expect("screen writes on y");

        assert_eq!(Value::text("notes"), selected.get(SettingId::LibraryPath));
        assert_eq!(Value::text("wiki"), selected.get(SettingId::LinkFormat));
    }

    #[test]
    fn no_writes_the_default_bundle() {
        let (selected, _) = drive("n\n");
        let selected = selected.expect("screen writes on n");

        assert_eq!(Value::text(""), selected.get(SettingId::LibraryPath));
        assert_eq!(Value::text("markdown"), selected.get(SettingId::LinkFormat));
    }

    #[test]
    fn quit_writes_nothing() {
        let (selected, _) = drive("q\n");

        assert!(selected.is_none());
    }

    #[test]
    fn end_of_input_writes_nothing() {
        let (selected, _) = drive("");

        assert!(selected.is_none());
    }

    #[test]
    fn unknown_input_asks_again() {
        let (selected, transcript) = drive("x\ny\n");
        let selected = selected.expect("screen writes on y");
        let asked: Vec<&str> = transcript
            .lines()
            .filter(|line| *line == "> answer y, n or q")
            .collect();

        assert_eq!(vec!["> answer y, n or q"], asked);
        assert_eq!(Value::text("notes"), selected.get(SettingId::LibraryPath));
    }

    #[test]
    fn the_screen_lists_every_setting_and_offers_the_differing_ones() {
        let (_, transcript) = drive("\n");

        assert_eq!(
            vec![
                "library notes none".to_string(),
                "format markdown".to_string(),
                "links wiki markdown".to_string(),
            ],
            rows_of(&transcript)
        );
    }

    #[test]
    fn an_overridden_setting_keeps_its_value_when_defaults_are_chosen() {
        let base = defaults();
        let mut detected = base.clone();
        detected.set(
            SettingId::LibraryPath,
            Value::text("notes"),
            Confidence::Detected,
            "3 of 3 files live under notes/",
        );
        detected.set(
            SettingId::LinkFormat,
            Value::text("wiki"),
            Confidence::Overridden,
            "set on the command line",
        );

        let (selected, transcript) = drive_bundles("n\n", &detected, &base);
        let selected = selected.expect("screen writes on n");

        assert_eq!(
            vec![
                "library notes none".to_string(),
                "format markdown".to_string(),
                "links wiki".to_string(),
            ],
            rows_of(&transcript)
        );
        assert_eq!(Value::text(""), selected.get(SettingId::LibraryPath));
        assert_eq!(Value::text("wiki"), selected.get(SettingId::LinkFormat));
    }
}
