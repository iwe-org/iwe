use clap::{Arg, Command};

pub const PROMPTS: [(&str, &str); 3] = [
    (
        "init",
        include_str!("../../../templates/claude/prompts/init.md"),
    ),
    (
        "distill",
        include_str!("../../../templates/claude/prompts/distill.md"),
    ),
    (
        "reflect",
        include_str!("../../../templates/claude/prompts/reflect.md"),
    ),
];

pub fn prompt_body(name: &str) -> Option<&'static str> {
    PROMPTS
        .iter()
        .find(|(prompt, _)| *prompt == name)
        .map(|(_, body)| *body)
}

pub fn invocations(body: &str) -> Vec<String> {
    let mut found = Vec::new();
    for line in joined_lines(body) {
        let mut from = 0;
        while let Some(offset) = line[from..].find("iwe ") {
            let at = from + offset;
            from = at + 4;
            if !starts_invocation(&line[..at]) {
                continue;
            }
            let rest = &line[at..];
            let end = ["`", "|", ";", "&", "<<", " #"]
                .iter()
                .filter_map(|delimiter| rest.find(delimiter))
                .min()
                .unwrap_or(rest.len());
            let invocation = rest[..end].trim();
            if invocation.split_whitespace().count() > 1 {
                found.push(invocation.to_string());
            }
        }
    }
    found
}

pub fn unknown_invocations(app: &Command, body: &str) -> Vec<String> {
    invocations(body)
        .into_iter()
        .filter_map(|invocation| {
            check_invocation(app, &invocation).map(|problem| format!("`{invocation}`: {problem}"))
        })
        .collect()
}

fn joined_lines(body: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut pending = String::new();
    for line in body.lines() {
        match line.strip_suffix('\\') {
            Some(continued) => {
                pending.push_str(continued);
                pending.push(' ');
            }
            None => {
                pending.push_str(line);
                lines.push(std::mem::take(&mut pending));
            }
        }
    }
    if !pending.is_empty() {
        lines.push(pending);
    }
    lines
}

fn starts_invocation(before: &str) -> bool {
    before.trim().is_empty() || before.ends_with('`')
}

fn check_invocation(app: &Command, invocation: &str) -> Option<String> {
    let mut path: Vec<&Command> = vec![app];
    let mut tokens = invocation.split_whitespace().skip(1).peekable();
    while let Some(token) = tokens.next() {
        if token == "--" {
            return None;
        }
        if let Some(long) = token.strip_prefix("--") {
            let (name, inline_value) = match long.split_once('=') {
                Some((name, value)) => (name, Some(value)),
                None => (long, None),
            };
            let Some(arg) = find_arg(&path, |arg| {
                arg.get_long() == Some(name)
                    || arg
                        .get_all_aliases()
                        .map(|aliases| aliases.contains(&name))
                        .unwrap_or(false)
            }) else {
                return Some(format!("unknown flag --{name}"));
            };
            if inline_value.is_none() && arg.get_action().takes_values() {
                tokens.next();
            }
            continue;
        }
        if token.len() > 1
            && token.starts_with('-')
            && !token[1..].starts_with(|c: char| c.is_ascii_digit())
        {
            let short = token.chars().nth(1).unwrap_or('-');
            let Some(arg) = find_arg(&path, |arg| {
                arg.get_short() == Some(short)
                    || arg
                        .get_all_short_aliases()
                        .map(|aliases| aliases.contains(&short))
                        .unwrap_or(false)
            }) else {
                return Some(format!("unknown flag -{short}"));
            };
            if token.len() == 2 && arg.get_action().takes_values() {
                tokens.next();
            }
            continue;
        }
        let node = path.last().copied().unwrap_or(app);
        if node.has_subcommands() {
            if token.starts_with('<') {
                return None;
            }
            match node.get_subcommands().find(|sub| {
                sub.get_name() == token || sub.get_all_aliases().any(|alias| alias == token)
            }) {
                Some(sub) => path.push(sub),
                None => return Some(format!("unknown subcommand {token}")),
            }
        }
    }
    None
}

fn find_arg<'a>(path: &[&'a Command], matches: impl Fn(&Arg) -> bool) -> Option<&'a Arg> {
    path.iter().enumerate().find_map(|(depth, command)| {
        let current = depth + 1 == path.len();
        command
            .get_arguments()
            .find(|arg| (current || arg.is_global_set()) && matches(arg))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::ArgAction;

    fn app() -> Command {
        let mut app = Command::new("iwe")
            .arg(
                Arg::new("verbose")
                    .long("verbose")
                    .short('v')
                    .global(true)
                    .action(ArgAction::Count),
            )
            .subcommand(Command::new("find").arg(Arg::new("lexical").long("lexical")))
            .subcommand(
                Command::new("stats")
                    .arg(Arg::new("format").long("format"))
                    .subcommand(Command::new("similarity").arg(Arg::new("threshold").short('t'))),
            )
            .subcommand(
                Command::new("internal").subcommand(
                    Command::new("claude")
                        .subcommand(Command::new("session").subcommand(Command::new("brief")))
                        .subcommand(
                            Command::new("enable")
                                .arg(Arg::new("typed").long("typed").action(ArgAction::SetTrue)),
                        ),
                ),
            );
        app.build();
        app
    }

    #[test]
    fn finds_backticked_and_block_invocations() {
        let body = "Run `iwe find --lexical x` first.\n```bash\niwe internal claude session brief | head\niwe schema   # only if bound\n```\nThe liwe crate.";
        assert_eq!(
            invocations(body),
            vec![
                "iwe find --lexical x",
                "iwe internal claude session brief",
                "iwe schema"
            ]
        );
    }

    #[test]
    fn joins_continued_lines() {
        let body = "```bash\niwe find --lexical x \\\n  --limit 5\n```";
        assert_eq!(invocations(body), vec!["iwe find --lexical x    --limit 5"]);
    }

    #[test]
    fn accepts_known_paths_flags_and_placeholders() {
        let body = "`iwe internal claude session brief`, `iwe internal claude enable --typed`, `iwe find --lexical \"<terms>\" -v`, `iwe <command> --help`, `iwe stats --format json`, `iwe stats similarity -t 0.85`, `iwe stats --format=csv`";
        assert!(unknown_invocations(&app(), body).is_empty());
    }

    #[test]
    fn rejects_unknown_subcommands_and_flags() {
        let problems = unknown_invocations(
            &app(),
            "`iwe internal claude session brif` then `iwe find --lexcal x` then `iwe memory on`",
        );
        assert_eq!(problems.len(), 3);
        assert!(problems[0].contains("unknown subcommand brif"));
        assert!(problems[1].contains("unknown flag --lexcal"));
        assert!(problems[2].contains("unknown subcommand memory"));
    }

    #[test]
    fn serves_every_prompt() {
        for (name, body) in PROMPTS {
            assert_eq!(prompt_body(name), Some(body));
            assert!(
                body.starts_with("# "),
                "{name} body must open with its title"
            );
        }
        assert_eq!(prompt_body("nope"), None);
    }
}
