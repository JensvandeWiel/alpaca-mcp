//! `cargo xtask lint-commits`: enforces Conventional Commits, see CONTRIBUTING.md.
//! Used by the git hook in .githooks/ and by .github/workflows/pr.yml.

use std::{fs, path::PathBuf, process::Command};

use anyhow::{Context, Result, bail};

use crate::workspace;

const TYPES: [&str; 11] = [
    "feat", "fix", "docs", "style", "refactor", "perf", "test", "build", "ci", "chore", "revert",
];

const MAX_HEADER: usize = 100;

#[derive(clap::Args)]
#[group(required = true, multiple = false)]
pub struct Args {
    /// A single message, for example a pull request title
    #[arg(long)]
    message: Option<String>,

    /// A commit message file as passed to the `commit-msg` hook
    #[arg(long, value_name = "PATH")]
    message_file: Option<PathBuf>,

    /// Every non-merge commit in a git range, for example `origin/main..HEAD`
    #[arg(long)]
    range: Option<String>,
}

pub fn run(args: Args) -> Result<()> {
    let messages = if let Some(message) = args.message {
        vec![message]
    } else if let Some(path) = args.message_file {
        let content =
            fs::read_to_string(&path).with_context(|| format!("cannot read {}", path.display()))?;
        let message = strip_comments(&content);
        // Git creates these itself; fixups disappear again when squashing.
        if ["Merge ", "fixup! ", "squash! ", "amend! "]
            .iter()
            .any(|prefix| message.starts_with(prefix))
        {
            return Ok(());
        }
        vec![message]
    } else {
        commits(&args.range.unwrap_or_default())?
    };

    let mut failures = 0;
    for message in &messages {
        if let Err(reason) = check(message) {
            failures += 1;
            eprintln!(
                "invalid: {}\n  {reason}",
                message.lines().next().unwrap_or_default()
            );
        }
    }
    if failures > 0 {
        bail!(
            "{failures} message(s) do not follow Conventional Commits.\n\
             Format: <type>(<scope>)!: <description>\n\
             Types:  {}\n\
             See CONTRIBUTING.md",
            TYPES.join(", ")
        );
    }
    Ok(())
}

fn commits(range: &str) -> Result<Vec<String>> {
    let output = workspace::capture(
        Command::new("git")
            .args(["log", "--no-merges", "--format=%B%x00", range, "--"])
            .current_dir(workspace::root()),
    )?;
    Ok(output
        .split('\0')
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map(str::to_owned)
        .collect())
}

/// Removes what git itself would remove from an edited commit message.
fn strip_comments(content: &str) -> String {
    content
        .lines()
        .take_while(|line| !(line.starts_with('#') && line.contains(">8")))
        .filter(|line| !line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
}

fn check(message: &str) -> Result<(), String> {
    let mut lines = message.lines();
    let header = lines.next().unwrap_or_default();

    let (prefix, description) = header
        .split_once(": ")
        .ok_or("expected `<type>: <description>`")?;
    // `!` marks a breaking change and goes after the scope.
    let prefix = prefix.strip_suffix('!').unwrap_or(prefix);
    let (kind, scope) = match prefix.split_once('(') {
        Some((kind, rest)) => {
            let scope = rest.strip_suffix(')').ok_or("unclosed scope")?;
            (kind, Some(scope))
        }
        None => (prefix, None),
    };

    if !TYPES.contains(&kind) {
        return Err(format!("unknown type `{kind}`"));
    }
    if let Some(scope) = scope {
        let valid = !scope.is_empty()
            && scope
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !valid {
            return Err(format!(
                "scope `{scope}` must be lowercase letters, digits or dashes"
            ));
        }
    }
    if description.trim().is_empty() || description.starts_with(' ') {
        return Err("the description is empty or starts with a space".to_owned());
    }
    if description.ends_with('.') {
        return Err("the description must not end with a period".to_owned());
    }
    if header.chars().count() > MAX_HEADER {
        return Err(format!(
            "the first line is longer than {MAX_HEADER} characters"
        ));
    }
    if lines.next().is_some_and(|line| !line.trim().is_empty()) {
        return Err("leave a blank line between the first line and the body".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_conventional_messages() {
        for message in [
            "feat: add weather server",
            "fix(mcp-core): keep logs off stdout",
            "feat(hello)!: rename the greet tool",
            "refactor!: drop the old config format",
            "chore(deps): bump rmcp\n\nBREAKING CHANGE: new protocol version",
        ] {
            assert_eq!(check(message), Ok(()), "{message}");
        }
    }

    #[test]
    fn rejects_other_messages() {
        let long = format!("feat: {}", "x".repeat(MAX_HEADER));
        for message in [
            "",
            "update stuff",
            "feature: add weather server",
            "Feat: add weather server",
            "feat:add weather server",
            "feat: ",
            "feat:  two spaces",
            "feat(): empty scope",
            "feat(Core): uppercase scope",
            "feat(core: unclosed scope",
            "feat!(core): misplaced marker",
            "fix: trailing period.",
            "fix: no blank line\nbody",
            long.as_str(),
        ] {
            assert!(check(message).is_err(), "{message}");
        }
    }

    #[test]
    fn ignores_comments_and_the_verbose_diff() {
        let content = "# comment\nfix: something\n# more\n# ------ >8 ------\ndiff --git a b\n";
        assert_eq!(strip_comments(content), "fix: something");
    }
}
