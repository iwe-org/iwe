use std::io::{BufRead, Write};
use std::path::Path;

use crate::init::evidence::Evidence;
use crate::init::fit::measure;
use crate::init::probe::Probes;
use crate::init::report::summary_line;
use crate::init::settings::{
    to_configuration, Confidence, SettingId, Settings, Value, ALL_SETTINGS,
};

pub enum Outcome {
    Write { settings: Settings, memory: bool },
    Quit,
}

pub fn run(
    root: &Path,
    evidence: &Evidence,
    detected: &Settings,
    defaults: &Settings,
    probes: &Probes,
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> Outcome {
    let decisions: Vec<SettingId> = detected
        .differing(defaults)
        .into_iter()
        .filter(|id| *id != SettingId::Agents)
        .filter(|id| detected.confidence(*id) != Confidence::Overridden)
        .collect();

    let mut chosen = detected.clone();

    if decisions.is_empty() {
        let _ = writeln!(output, "{}", summary_line(evidence));
        let _ = writeln!(
            output,
            "detection matches the iwe defaults — nothing to choose"
        );
    } else {
        draw(root, evidence, detected, defaults, &decisions, output);

        loop {
            let mut line = String::new();
            if input.read_line(&mut line).unwrap_or(0) == 0 {
                let _ = writeln!(output);
                return Outcome::Quit;
            }

            match line.trim() {
                "" | "y" => break,
                "n" => {
                    for id in &decisions {
                        chosen.adopt(*id, defaults);
                    }
                    break;
                }
                "q" => return Outcome::Quit,
                _ => {
                    let _ = writeln!(output, "answer y, n or q");
                    prompt(output);
                }
            }
        }
    }

    if probes.has_agent_surface() {
        let _ = writeln!(output);
        let _ = writeln!(output, "{}", probes.agent_surface_note());
        let agents = match confirm(
            "add agent instructions to AGENTS.md and the MCP server to .mcp.json?",
            input,
            output,
        ) {
            Some(answer) => answer,
            None => return Outcome::Quit,
        };
        chosen.set(
            SettingId::Agents,
            Value::Bool(agents),
            Confidence::Asked,
            "",
        );
    }

    let mut memory = false;
    if probes.claude_dir {
        memory = match confirm(
            "enable claude memory (writes the MEMORY.md policy)?",
            input,
            output,
        ) {
            Some(answer) => answer,
            None => return Outcome::Quit,
        };
    }

    Outcome::Write {
        settings: chosen,
        memory,
    }
}

fn confirm(question: &str, input: &mut impl BufRead, output: &mut impl Write) -> Option<bool> {
    let _ = writeln!(output, "{} y yes · ⏎/n no · q quit", question);
    let _ = write!(output, "> ");
    let _ = output.flush();

    loop {
        let mut line = String::new();
        if input.read_line(&mut line).unwrap_or(0) == 0 {
            let _ = writeln!(output);
            return None;
        }

        match line.trim() {
            "" | "n" => return Some(false),
            "y" => return Some(true),
            "q" => return None,
            _ => {
                let _ = writeln!(output, "answer y, n or q");
                let _ = write!(output, "> ");
                let _ = output.flush();
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
        if id == SettingId::Agents {
            continue;
        }
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
    use crate::init::probe::Probes;
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

    fn claude_probes() -> Probes {
        let mut probes = Probes::default();
        probes.claude_dir = true;
        probes
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
        let (selected, _, transcript) = drive_probes(input, detected, base, &Probes::default());
        (selected, transcript)
    }

    fn drive_probes(
        input: &str,
        detected: &Settings,
        base: &Settings,
        probes: &Probes,
    ) -> (Option<Settings>, bool, String) {
        let mut reader = Cursor::new(input.as_bytes().to_vec());
        let mut output = Vec::new();

        let outcome = run(
            Path::new("/iwe-nonexistent-corpus"),
            &Evidence::default(),
            detected,
            base,
            probes,
            &mut reader,
            &mut output,
        );

        let (selected, memory) = match outcome {
            Outcome::Write { settings, memory } => (Some(settings), memory),
            Outcome::Quit => (None, false),
        };

        (
            selected,
            memory,
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

    fn questions_of(transcript: &str) -> Vec<String> {
        transcript
            .lines()
            .map(|line| line.trim_start_matches("> "))
            .filter(|line| line.ends_with("y yes · ⏎/n no · q quit"))
            .map(|line| line.to_string())
            .collect()
    }

    #[test]
    fn the_agent_and_memory_questions_default_to_no() {
        let (detected, base) = bundles();
        let (selected, memory, transcript) =
            drive_probes("\n\n\n", &detected, &base, &claude_probes());
        let selected = selected.expect("screen writes on Enter");

        assert_eq!(Value::Bool(false), selected.get(SettingId::Agents));
        assert_eq!(false, memory);
        assert_eq!(
            vec![
                "add agent instructions to AGENTS.md and the MCP server to .mcp.json? y yes · ⏎/n no · q quit".to_string(),
                "enable claude memory (writes the MEMORY.md policy)? y yes · ⏎/n no · q quit".to_string(),
            ],
            questions_of(&transcript)
        );
    }

    #[test]
    fn yes_answers_enable_agent_files_and_memory() {
        let (detected, base) = bundles();
        let (selected, memory, _) = drive_probes("\ny\ny\n", &detected, &base, &claude_probes());
        let selected = selected.expect("screen writes on Enter");

        assert_eq!(Value::Bool(true), selected.get(SettingId::Agents));
        assert_eq!(true, memory);
    }

    #[test]
    fn the_memory_question_needs_a_claude_directory() {
        let (detected, base) = bundles();
        let mut probes = Probes::default();
        probes.mcp_config = true;

        let (selected, memory, transcript) = drive_probes("\ny\n", &detected, &base, &probes);
        let selected = selected.expect("screen writes on Enter");

        assert_eq!(Value::Bool(true), selected.get(SettingId::Agents));
        assert_eq!(false, memory);
        assert_eq!(
            vec![
                "add agent instructions to AGENTS.md and the MCP server to .mcp.json? y yes · ⏎/n no · q quit".to_string(),
            ],
            questions_of(&transcript)
        );
    }

    #[test]
    fn no_questions_without_an_agent_surface() {
        let (detected, base) = bundles();
        let (selected, memory, transcript) =
            drive_probes("\n", &detected, &base, &Probes::default());

        assert!(selected.is_some());
        assert_eq!(false, memory);
        assert_eq!(Vec::<String>::new(), questions_of(&transcript));
    }

    #[test]
    fn quit_at_a_question_writes_nothing() {
        let (detected, base) = bundles();
        let (selected, memory, _) = drive_probes("\nq\n", &detected, &base, &claude_probes());

        assert!(selected.is_none());
        assert_eq!(false, memory);
    }

    #[test]
    fn choosing_defaults_still_asks_the_questions() {
        let (detected, base) = bundles();
        let (selected, memory, _) = drive_probes("n\ny\ny\n", &detected, &base, &claude_probes());
        let selected = selected.expect("screen writes on n");

        assert_eq!(Value::text(""), selected.get(SettingId::LibraryPath));
        assert_eq!(Value::Bool(true), selected.get(SettingId::Agents));
        assert_eq!(true, memory);
    }
}
