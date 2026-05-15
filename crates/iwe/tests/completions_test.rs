use std::process::Command;

fn run_completions(shell: &str) -> std::process::Output {
    Command::new(crate::common::get_iwe_binary_path())
        .arg("completions")
        .arg(shell)
        .output()
        .expect("Failed to execute iwe completions")
}

#[test]
fn test_completions_bash() {
    let output = run_completions("bash");
    assert!(output.status.success());
    assert!(!output.stdout.is_empty());
}

#[test]
fn test_completions_elvish() {
    let output = run_completions("elvish");
    assert!(output.status.success());
    assert!(!output.stdout.is_empty());
}

#[test]
fn test_completions_fish() {
    let output = run_completions("fish");
    assert!(output.status.success());
    assert!(!output.stdout.is_empty());
}

#[test]
fn test_completions_nushell() {
    let output = run_completions("nushell");
    assert!(output.status.success());
    assert!(!output.stdout.is_empty());
}

#[test]
fn test_completions_powershell() {
    let output = run_completions("powershell");
    assert!(output.status.success());
    assert!(!output.stdout.is_empty());
}

#[test]
fn test_completions_zsh() {
    let output = run_completions("zsh");
    assert!(output.status.success());
    assert!(!output.stdout.is_empty());
}

#[test]
fn test_completions_rejects_unknown_shell() {
    let output = Command::new(crate::common::get_iwe_binary_path())
        .arg("completions")
        .arg("tcsh")
        .output()
        .expect("Failed to execute iwe completions");
    assert!(!output.status.success());
}
