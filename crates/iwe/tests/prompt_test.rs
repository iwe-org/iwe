use std::process::Command;

fn run_prompt(args: &[&str]) -> std::process::Output {
    Command::new(crate::common::get_iwe_binary_path())
        .args(["internal", "claude", "prompt"])
        .args(args)
        .output()
        .expect("Failed to execute iwe internal claude prompt")
}

#[test]
fn prints_each_prompt_body_verbatim() {
    for (name, body) in iwe::internal::claude::prompt::PROMPTS {
        let output = run_prompt(&[name]);
        assert!(output.status.success(), "{name} should print");
        assert_eq!(String::from_utf8_lossy(&output.stdout), body, "{name} body");
    }
}

#[test]
fn works_outside_any_workspace() {
    let dir = tempfile::tempdir().unwrap();
    let output = Command::new(crate::common::get_iwe_binary_path())
        .current_dir(dir.path())
        .args(["internal", "claude", "prompt", "distill"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("# Distill"));
}

#[test]
fn rejects_unknown_names() {
    let output = run_prompt(&["remember"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("distill-agent"));
}
