use crate::init::evidence::Evidence;
use crate::init::probe::Probes;
use crate::init::settings::{defaults, Confidence, SettingId, Settings, Value};

const MAJORITY: usize = 80;
const MIN_DATE_FILES: usize = 3;
const MIN_WRAPPED_LINES: usize = 20;
const STANDARD_COLUMNS: [usize; 5] = [72, 80, 88, 100, 120];

fn share(part: usize, total: usize) -> usize {
    if total == 0 {
        return 0;
    }
    part * 100 / total
}

pub fn detect(evidence: &Evidence, probes: &Probes) -> Settings {
    let mut settings = defaults();

    detect_library(&mut settings, evidence);
    detect_format(&mut settings, evidence);
    detect_link_format(&mut settings, evidence);
    detect_refs_extension(&mut settings, evidence);
    detect_refs_path(&mut settings, evidence);
    detect_wiki_link_path(&mut settings, evidence);
    detect_refs_text(&mut settings, evidence);
    detect_dates(&mut settings, evidence);
    detect_titles(&mut settings, evidence);
    detect_key_template(&mut settings, evidence);
    detect_language(&mut settings, evidence);
    detect_tokens(&mut settings, evidence);
    detect_wrapping(&mut settings, evidence);
    detect_agents(&mut settings, probes);

    settings
}

fn detect_library(settings: &mut Settings, evidence: &Evidence) {
    if evidence.library_path.is_empty() {
        return;
    }
    settings.set(
        SettingId::LibraryPath,
        Value::text(&evidence.library_path),
        Confidence::Detected,
        &format!(
            "{} of {} files live under {}/",
            evidence.scanned_files, evidence.files, evidence.library_path
        ),
    );
}

fn detect_format(settings: &mut Settings, evidence: &Evidence) {
    if evidence.djot_files > evidence.markdown_files {
        settings.set(
            SettingId::Format,
            Value::text("djot"),
            Confidence::Detected,
            &format!(
                "{} djot files vs {} markdown files",
                evidence.djot_files, evidence.markdown_files
            ),
        );
        if evidence.markdown_files > 0 {
            settings.mark_mixed(SettingId::Format);
        }
    } else if evidence.djot_files > 0 {
        settings.mark_mixed(SettingId::Format);
    }
}

fn detect_link_format(settings: &mut Settings, evidence: &Evidence) {
    let total = evidence.local_links();
    if total == 0 {
        return;
    }

    let wiki_share = share(evidence.wiki_links, total);
    let value = if evidence.wiki_links >= evidence.markdown_links {
        "wiki"
    } else {
        "markdown"
    };
    let note = format!(
        "{} wiki links vs {} markdown links",
        evidence.wiki_links, evidence.markdown_links
    );

    settings.set(
        SettingId::LinkFormat,
        Value::text(value),
        Confidence::Detected,
        &note,
    );

    let dominant_share = wiki_share.max(100 - wiki_share);
    if dominant_share < MAJORITY {
        settings.mark_mixed(SettingId::LinkFormat);
    }
}

fn detect_refs_extension(settings: &mut Settings, evidence: &Evidence) {
    let total = evidence.refs_with_extension + evidence.refs_bare;
    if total == 0 {
        return;
    }

    let with_share = share(evidence.refs_with_extension, total);
    let value = if evidence.refs_with_extension > evidence.refs_bare {
        ".md"
    } else {
        ""
    };
    settings.set(
        SettingId::RefsExtension,
        Value::text(value),
        Confidence::Detected,
        &format!(
            "{} links end in .md, {} have no extension",
            evidence.refs_with_extension, evidence.refs_bare
        ),
    );

    let dominant_share = with_share.max(100 - with_share);
    if dominant_share < MAJORITY {
        settings.mark_mixed(SettingId::RefsExtension);
    }
}

fn detect_refs_path(settings: &mut Settings, evidence: &Evidence) {
    let votes = evidence.refs_relative_votes + evidence.refs_absolute_votes;
    if votes == 0 {
        return;
    }

    let value = if evidence.refs_absolute_votes > evidence.refs_relative_votes {
        "absolute"
    } else {
        "relative"
    };
    settings.set(
        SettingId::RefsPath,
        Value::text(value),
        Confidence::Detected,
        &format!(
            "{} links resolve relative to their own directory, {} from the library root",
            evidence.refs_relative_votes, evidence.refs_absolute_votes
        ),
    );

    let dominant = evidence
        .refs_relative_votes
        .max(evidence.refs_absolute_votes);
    if share(dominant, votes) < MAJORITY {
        settings.mark_mixed(SettingId::RefsPath);
    }
}

fn detect_wiki_link_path(settings: &mut Settings, evidence: &Evidence) {
    let total = evidence.wiki_pathed + evidence.wiki_bare;
    if total == 0 {
        return;
    }

    let pathed_share = share(evidence.wiki_pathed, total);
    let value = if pathed_share >= MAJORITY {
        "full"
    } else if pathed_share <= 100 - MAJORITY {
        "short"
    } else {
        "preserve"
    };

    let confidence = if value == "preserve" {
        Confidence::Assumed
    } else {
        Confidence::Detected
    };

    let note = format!(
        "{} wiki links carry a path, {} are bare names",
        evidence.wiki_pathed, evidence.wiki_bare
    );

    settings.set(
        SettingId::WikiLinkPath,
        Value::text(value),
        confidence,
        &note,
    );

    if value == "preserve" {
        settings.mark_mixed(SettingId::WikiLinkPath);
    }
}

fn detect_refs_text(settings: &mut Settings, evidence: &Evidence) {
    let total = evidence.refs_text_matching + evidence.refs_text_diverging;
    if total == 0 {
        return;
    }

    let value = if evidence.refs_text_diverging == 0 {
        "normalize"
    } else {
        "preserve"
    };
    settings.set(
        SettingId::RefsText,
        Value::text(value),
        Confidence::Detected,
        &format!(
            "{} link texts match their target title, {} diverge",
            evidence.refs_text_matching, evidence.refs_text_diverging
        ),
    );
}

fn detect_dates(settings: &mut Settings, evidence: &Evidence) {
    if let Some((pattern, count)) = evidence.key_date_matches.dominant() {
        if count >= MIN_DATE_FILES {
            settings.set(
                SettingId::KeyDateFormat,
                Value::text(&pattern),
                Confidence::Detected,
                &format!("{} filenames parse as {}", count, pattern),
            );
        }
    }

    if let Some((pattern, count)) = evidence.title_date_matches.dominant() {
        if count >= MIN_DATE_FILES {
            settings.set(
                SettingId::DisplayDateFormat,
                Value::text(&pattern),
                Confidence::Detected,
                &format!("{} date-keyed titles parse as {}", count, pattern),
            );
        }
    }
}

fn detect_titles(settings: &mut Settings, evidence: &Evidence) {
    if evidence.scanned_files == 0 {
        return;
    }
    if evidence.frontmatter_title_without_header * 2 > evidence.scanned_files {
        settings.set(
            SettingId::FrontmatterTitle,
            Value::text("title"),
            Confidence::Detected,
            &format!(
                "{} of {} files carry a frontmatter title and no header",
                evidence.frontmatter_title_without_header, evidence.scanned_files
            ),
        );
    }
}

fn detect_key_template(settings: &mut Settings, evidence: &Evidence) {
    let styles = &evidence.key_styles;
    let total = styles.total();
    if total == 0 {
        return;
    }

    let named: Vec<(String, usize)> = styles
        .entries()
        .into_iter()
        .filter(|(style, _)| style != "date")
        .collect();
    let named_total: usize = named.iter().map(|(_, count)| count).sum();
    if named_total == 0 {
        return;
    }

    let dominant = named
        .iter()
        .max_by_key(|(_, count)| *count)
        .expect("non-empty style tally");

    if share(dominant.1, named_total) < MAJORITY {
        settings.mark_mixed(SettingId::KeyTemplate);
        return;
    }

    let template = match dominant.0.as_str() {
        "title" => "{{title}}",
        "id" => "{{id}}",
        _ => "{{slug}}",
    };

    settings.set(
        SettingId::KeyTemplate,
        Value::text(template),
        Confidence::Detected,
        &format!(
            "{} of {} filenames use {} style",
            dominant.1, named_total, dominant.0
        ),
    );
}

fn detect_language(settings: &mut Settings, evidence: &Evidence) {
    let dominant = match evidence.language_scores.dominant() {
        Some(dominant) => dominant,
        None => return,
    };
    let runner_up = evidence
        .language_scores
        .runner_up()
        .map(|(_, count)| count)
        .unwrap_or(0);

    if dominant.1 >= runner_up * 3 / 2 && dominant.1 > 0 {
        settings.set(
            SettingId::SearchLanguage,
            Value::text(&dominant.0),
            Confidence::Detected,
            &format!(
                "{} stopword hits for {}, {} for the next language",
                dominant.1, dominant.0, runner_up
            ),
        );
        return;
    }

    if let Some(language) = &evidence.locale_language {
        settings.set(
            SettingId::SearchLanguage,
            Value::text(language),
            Confidence::Assumed,
            "stopword counts were inconclusive; used the shell locale",
        );
    }
}

fn detect_tokens(settings: &mut Settings, evidence: &Evidence) {
    dominant_text(
        settings,
        SettingId::ListToken,
        &evidence.list_tokens.entries(),
        "list items",
    );
    dominant_text(
        settings,
        SettingId::EmphasisToken,
        &evidence.emphasis_tokens.entries(),
        "emphasis spans",
    );
    dominant_text(
        settings,
        SettingId::StrongToken,
        &evidence.strong_tokens.entries(),
        "strong spans",
    );
    dominant_text(
        settings,
        SettingId::OrderedListToken,
        &evidence.ordered_tokens.entries(),
        "ordered list items",
    );
    dominant_text(
        settings,
        SettingId::CodeBlockToken,
        &evidence.code_fence_tokens.entries(),
        "code fences",
    );
    dominant_text(
        settings,
        SettingId::RuleToken,
        &evidence.rule_tokens.entries(),
        "thematic breaks",
    );

    dominant_number(
        settings,
        SettingId::CodeBlockTokenCount,
        &evidence.code_fence_lengths.entries(),
        "code fences",
        3,
    );
    dominant_number(
        settings,
        SettingId::RuleTokenCount,
        &evidence.rule_lengths.entries(),
        "thematic breaks",
        3,
    );

    detect_indent(
        settings,
        SettingId::BulletListContentIndent,
        &evidence.bullet_indents.entries(),
        "bullet list items",
    );
    detect_indent(
        settings,
        SettingId::OrderedListContentIndent,
        &evidence.ordered_indents.entries(),
        "ordered list items",
    );

    let runs = evidence.ordered_incrementing + evidence.ordered_flat;
    if runs > 0 && evidence.ordered_flat > evidence.ordered_incrementing {
        settings.set(
            SettingId::IncrementOrderedListBullets,
            Value::Bool(false),
            Confidence::Detected,
            &format!(
                "{} ordered lists repeat 1., {} count up",
                evidence.ordered_flat, evidence.ordered_incrementing
            ),
        );
    }
}

fn dominant_text(
    settings: &mut Settings,
    id: SettingId,
    entries: &[(String, usize)],
    subject: &str,
) {
    let total: usize = entries.iter().map(|(_, count)| count).sum();
    if total == 0 {
        return;
    }
    let dominant = entries
        .iter()
        .max_by_key(|(_, count)| *count)
        .expect("non-empty tally");

    settings.set(
        id,
        Value::text(&dominant.0),
        Confidence::Detected,
        &format!("{} of {} {} use {}", dominant.1, total, subject, dominant.0),
    );

    if share(dominant.1, total) < MAJORITY {
        settings.mark_mixed(id);
    }
}

fn dominant_number(
    settings: &mut Settings,
    id: SettingId,
    entries: &[(usize, usize)],
    subject: &str,
    minimum: usize,
) {
    let total: usize = entries.iter().map(|(_, count)| count).sum();
    if total == 0 {
        return;
    }
    let dominant = entries
        .iter()
        .max_by_key(|(_, count)| *count)
        .expect("non-empty tally");
    if dominant.0 < minimum {
        return;
    }

    settings.set(
        id,
        Value::Number(dominant.0),
        Confidence::Detected,
        &format!(
            "{} of {} {} are {} characters long",
            dominant.1, total, subject, dominant.0
        ),
    );
}

fn detect_indent(
    settings: &mut Settings,
    id: SettingId,
    entries: &[(usize, usize)],
    subject: &str,
) {
    let candidates: Vec<&(usize, usize)> = entries
        .iter()
        .filter(|(width, _)| (2..=4).contains(width))
        .collect();
    let total: usize = candidates.iter().map(|(_, count)| count).sum();
    if total == 0 {
        return;
    }
    let dominant = candidates
        .iter()
        .max_by_key(|(_, count)| *count)
        .expect("non-empty tally");
    if dominant.0 == 2 {
        return;
    }

    settings.set(
        id,
        Value::Number(dominant.0),
        Confidence::Detected,
        &format!(
            "{} of {} {} indent content to column {}",
            dominant.1, total, subject, dominant.0
        ),
    );
}

fn detect_wrapping(settings: &mut Settings, evidence: &Evidence) {
    detect_wrap_column(settings, evidence);

    let breaks = evidence.hard_break_backslash + evidence.hard_break_spaces;
    if breaks >= 3 {
        settings.set(
            SettingId::PreserveLineBreaks,
            Value::Bool(true),
            Confidence::Detected,
            &format!(
                "{} hard line breaks ({} backslash, {} trailing spaces)",
                breaks, evidence.hard_break_backslash, evidence.hard_break_spaces
            ),
        );
        if evidence.hard_break_spaces > evidence.hard_break_backslash {
            settings.set(
                SettingId::LineBreakStyleId,
                Value::text("spaces"),
                Confidence::Detected,
                &format!(
                    "{} breaks use trailing spaces vs {} backslash",
                    evidence.hard_break_spaces, evidence.hard_break_backslash
                ),
            );
        }
    }

    if evidence.scanned_files > 0 && evidence.blank_run_files * 5 >= evidence.scanned_files {
        settings.set(
            SettingId::PreserveNewlines,
            Value::Bool(true),
            Confidence::Detected,
            &format!(
                "{} of {} files contain runs of blank lines",
                evidence.blank_run_files, evidence.scanned_files
            ),
        );
    }
}

fn detect_wrap_column(settings: &mut Settings, evidence: &Evidence) {
    let entries = evidence.wrapped_line_lengths.entries();
    let total: usize = entries.iter().map(|(_, count)| count).sum();
    if total < MIN_WRAPPED_LINES {
        return;
    }
    if evidence.multiline_paragraphs * 4 < evidence.single_line_paragraphs {
        return;
    }

    let longest = entries
        .iter()
        .map(|(length, _)| *length)
        .max()
        .unwrap_or_default();
    if !(40..=120).contains(&longest) {
        return;
    }

    let column = STANDARD_COLUMNS
        .iter()
        .find(|candidate| **candidate >= longest)
        .copied()
        .unwrap_or(longest);

    settings.set(
        SettingId::WrapColumn,
        Value::Number(column),
        Confidence::Detected,
        &format!("{} wrapped lines, longest is {} characters", total, longest),
    );
}

fn detect_agents(settings: &mut Settings, probes: &Probes) {
    if !probes.has_agent_surface() {
        return;
    }
    settings.set(
        SettingId::Agents,
        Value::Bool(true),
        Confidence::Detected,
        &probes.agent_surface_note(),
    );
}
