use indoc::indoc;
use std::fs::{create_dir_all, read_to_string, write};
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

const DUP_A: &str = indoc! {"
    # Ada and Kai

    Ada and Kai met in Vienna in the spring of 1998 while both were studying
    analog synthesizers. They collaborated on a modular sequencer and later
    co-founded a small workshop building custom filters for musicians.
"};

const DUP_B: &str = indoc! {"
    # Kai and Ada

    Kai and Ada met in Vienna during spring 1998 while studying analog
    synthesizers together. They built a modular sequencer, then started a
    small workshop making custom filters for working musicians.
"};

const UNRELATED: &str = indoc! {"
    # Tax Filing Checklist

    Gather receipts, confirm the standard deduction, review quarterly estimated
    payments, and submit the federal return before the April deadline.
"};

const ALPHA: &str = indoc! {"
    # Ada and Kai

    Ada and Kai met in Vienna in the spring of 1998 while both were studying
    analog synthesizers together. They collaborated on a modular sequencer and
    later co-founded a small workshop building custom filters for touring
    musicians across Europe and beyond.
"};

const BETA: &str = indoc! {"
    # Ada and Kai

    Ada and Kai met in Vienna in the summer of 1998 while both were studying
    analog synthesizers together. They collaborated on a modular sequencer and
    later co-founded a small workshop building custom filters for touring
    musicians across Europe and beyond.
"};

const PARAPHRASE: &str = indoc! {"
    # Kai and Ada

    Kai and Ada first crossed paths in Vienna during the spring season of 1998
    while the two of them were studying analog synthesizers. Together they built
    a modular sequencer, and afterwards launched a modest workshop crafting
    bespoke filters for gigging musicians touring the continent.
"};

const DISTINCT: &str = indoc! {"
    # Tax Filing Checklist

    Gather every receipt, confirm the standard deduction amount, review the
    quarterly estimated payments, reconcile the brokerage statements, and submit
    the completed federal return well before the April filing deadline to avoid
    any late penalties or accrued interest charges this year.
"};

#[test]
fn stats_reports_orphans_section() {
    let temp = TempDir::new().expect("tempdir");
    write_config(temp.path());
    write(temp.path().join("dup-a.md"), DUP_A).unwrap();
    write(temp.path().join("dup-b.md"), DUP_B).unwrap();
    write(temp.path().join("taxes.md"), UNRELATED).unwrap();

    let output = run(temp.path(), "stats", &["--format", "markdown"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let normalized = stdout
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    let expected = indoc! {"
        # Graph Statistics

        ## Overview

        - **Total documents:** 3
        - **Total nodes:** 9
        - **Total paths:** 3

        ## Document Statistics

        - **Total sections:** 3
        - **Average sections/doc:** 1.0

        ### Top Documents by Sections

        1. **Ada and Kai** (1 sections)
        2. **Kai and Ada** (1 sections)
        3. **Tax Filing Checklist** (1 sections)


        ## Reference Statistics

        - **Block references:**
        - **Inline references:**
        - **Total references:** 0
        - **Orphaned documents:** 3 (100.0%)
        - **Leaf documents:** 3 (100.0%)

        ## Lines Statistics

        - **Total lines:** 14
        - **Average lines/doc:** 4.67

        ### Top Documents by Lines

        1. **Ada and Kai** (5 lines)
        2. **Kai and Ada** (5 lines)
        3. **Tax Filing Checklist** (4 lines)


        ## Words Statistics

        - **Total words:** 95
        - **Average words/doc:** 31.67

        ### Top Documents by Words

        1. **Ada and Kai** (38 words)
        2. **Kai and Ada** (34 words)
        3. **Tax Filing Checklist** (23 words)


        ## Structure Statistics

        - **Root sections:** 3
        - **Maximum path depth:** 1
        - **Average path depth:** 1.0
        - **Bullet lists:** 0
        - **Ordered lists:** 0
        - **Code blocks:** 0
        - **Tables:** 0
        - **Quotes:** 0

        ## Orphans

        - dup-a
        - dup-b
        - taxes


        ## Network Analysis

        - **Average references/doc:** 0.0
    "};
    assert_eq!(normalized, expected.trim_end());
}

#[test]
fn stats_similarity_lists_each_pair_once() {
    let temp = TempDir::new().expect("tempdir");
    write_config(temp.path());
    write(temp.path().join("alpha.md"), ALPHA).unwrap();
    write(temp.path().join("beta.md"), BETA).unwrap();
    write(temp.path().join("distinct.md"), DISTINCT).unwrap();

    let output = run(temp.path(), "stats", &["similarity"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    assert_eq!(stdout, "alpha\tbeta\n");
}

#[test]
fn stats_similarity_threshold_controls_the_match_level() {
    let temp = TempDir::new().expect("tempdir");
    write_config(temp.path());
    write(temp.path().join("alpha.md"), ALPHA).unwrap();
    write(temp.path().join("paraphrase.md"), PARAPHRASE).unwrap();

    let strict = run(temp.path(), "stats", &["similarity"]);
    assert!(strict.status.success());
    assert_eq!(
        String::from_utf8(strict.stdout).expect("Valid UTF-8 output"),
        ""
    );

    let loose = run(temp.path(), "stats", &["similarity", "--threshold", "0.3"]);
    assert!(loose.status.success());
    assert_eq!(
        String::from_utf8(loose.stdout).expect("Valid UTF-8 output"),
        "alpha\tparaphrase\n"
    );
}

#[test]
fn stats_similarity_rejects_non_positive_threshold() {
    let temp = TempDir::new().expect("tempdir");
    write_config(temp.path());
    write(temp.path().join("alpha.md"), ALPHA).unwrap();

    let output = run(temp.path(), "stats", &["similarity", "--threshold", "0"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("Valid UTF-8 output");
    assert_eq!(
        stderr.lines().next(),
        Some("error: invalid value '0' for '--threshold <THRESHOLD>': threshold must be a positive number (typically between 0.5 and 1.0)")
    );
}

#[test]
fn update_strict_prints_stats_warnings_without_blocking() {
    let temp = TempDir::new().expect("tempdir");
    write_config(temp.path());
    write(temp.path().join("notes.md"), "# Notes\n").unwrap();

    let output = run(
        temp.path(),
        "update",
        &[
            "-k",
            "notes",
            "--content",
            "# Notes\n\nSee [missing](ghost).\n",
            "--strict",
        ],
    );

    assert!(output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("Valid UTF-8 output");
    assert_eq!(
        stderr,
        indoc! {"
            stats: notes › orphan: no page links here
            stats: notes › dangling-link: links to missing 'ghost'
        "}
    );
    assert_eq!(
        read_to_string(temp.path().join("notes.md")).unwrap(),
        "# Notes\n\nSee [missing](ghost).\n"
    );
}

#[test]
fn update_without_strict_stays_silent() {
    let temp = TempDir::new().expect("tempdir");
    write_config(temp.path());
    write(temp.path().join("notes.md"), "# Notes\n").unwrap();

    let output = run(
        temp.path(),
        "update",
        &[
            "-k",
            "notes",
            "--content",
            "# Notes\n\nSee [missing](ghost).\n",
        ],
    );

    assert!(output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("Valid UTF-8 output");
    assert_eq!(stderr, "");
}

fn write_config(path: &Path) {
    create_dir_all(path.join(".iwe")).unwrap();
    write(
        path.join(".iwe/config.toml"),
        "library.path = \"\"\nmarkdown.refs_extension = \"\"\n",
    )
    .unwrap();
}

fn run(work_dir: &Path, command: &str, args: &[&str]) -> Output {
    let mut cmd = Command::new(crate::common::get_iwe_binary_path());
    cmd.arg(command).current_dir(work_dir);
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("run iwe command")
}
