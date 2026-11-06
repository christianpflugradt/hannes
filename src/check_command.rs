//! One executable with arguments: POSIX quoting, without shell evaluation.
use crate::limits::{MAX_ARGUMENT_BYTES, MAX_COMMAND_ARGS};
use anyhow::{Context, Result, bail, ensure};

const COMPOSITION_HINT: &str = "Hannes supports one executable per check. Use an npm script, Gradle task, Makefile or dedicated script to combine multiple commands.";

pub fn parse(source: &str) -> Result<Vec<String>> {
    reject_shell_syntax(source)?;
    let arguments = shell_words::split(source).context("Unclosed quote in command")?;
    ensure!(
        !arguments.is_empty() && !arguments[0].trim().is_empty(),
        "Command must contain a nonempty executable name"
    );
    ensure!(
        arguments.len() <= MAX_COMMAND_ARGS,
        "Command must have 1..={MAX_COMMAND_ARGS} executable/argument elements"
    );
    ensure!(
        arguments
            .iter()
            .all(|argument| !argument.contains('\0') && argument.len() <= MAX_ARGUMENT_BYTES),
        "Command arguments contain NUL or exceed {MAX_ARGUMENT_BYTES} bytes"
    );
    Ok(arguments)
}

// Inspect original spelling before quote removal. The tokenizer deliberately
// treats operators/expansions as text, so it cannot perform this validation.
fn reject_shell_syntax(source: &str) -> Result<()> {
    let mut quote = None;
    let mut word_start = true;
    let mut glob_bracket = false;
    let mut characters = source.chars().peekable();
    while let Some(character) = characters.next() {
        if quote == Some('\'') {
            if character == '\'' {
                quote = None;
            }
            continue;
        }
        if character == '\\' {
            ensure!(
                characters.next().is_some(),
                "Invalid trailing escape in command"
            );
            word_start = false;
            continue;
        }
        if Some(character) == quote {
            quote = None;
            continue;
        }
        if quote.is_none() && matches!(character, '\'' | '"') {
            quote = Some(character);
            word_start = false;
            continue;
        }
        if character == '`' {
            bail!("Unsupported backtick command substitution. {COMPOSITION_HINT}");
        }
        if character == '$'
            && characters
                .peek()
                .is_some_and(|next| next.is_ascii_alphanumeric() || "_({@*#?$!-".contains(*next))
        {
            bail!(
                "Unsupported shell expansion beginning with '$'. Quote literal text with single quotes or escape '$'. {COMPOSITION_HINT}"
            );
        }
        if quote.is_none() {
            if matches!(
                character,
                '&' | '|' | ';' | '<' | '>' | '(' | ')' | '\n' | '\r'
            ) {
                let mut operator = character.to_string();
                if matches!(character, '&' | '|' | '<' | '>')
                    && characters.peek() == Some(&character)
                {
                    operator.push(characters.next().unwrap());
                }
                bail!("Unsupported shell operator {operator:?}. {COMPOSITION_HINT}");
            }
            if matches!(character, '*' | '?') || (character == ']' && glob_bracket) {
                bail!(
                    "Unsupported glob pattern. Quote or escape literal wildcard characters. {COMPOSITION_HINT}"
                );
            }
            if character == '[' {
                glob_bracket = true;
            } else if character.is_ascii_whitespace() {
                glob_bracket = false;
            }
            if word_start && matches!(character, '#' | '~') {
                bail!(
                    "Unsupported shell comment or tilde expansion. Quote or escape literal text. {COMPOSITION_HINT}"
                );
            }
            word_start = character.is_ascii_whitespace();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoting_and_escaping_preserve_exact_arguments() {
        for (source, expected) in [
            ("cargo test --locked", vec!["cargo", "test", "--locked"]),
            (
                "git log -1 --format='%h %s'",
                vec!["git", "log", "-1", "--format=%h %s"],
            ),
            (
                r#"git commit -m "Fix broken tests""#,
                vec!["git", "commit", "-m", "Fix broken tests"],
            ),
            (
                r"tool directory\ with\ spaces/file.txt",
                vec!["tool", "directory with spaces/file.txt"],
            ),
            (
                r#"tool \"hello\" 'it'\''s' "say \"hi\"""#,
                vec!["tool", "\"hello\"", "it's", "say \"hi\""],
            ),
            (r#""tools/my tool" --flag"#, vec!["tools/my tool", "--flag"]),
            (
                r#"tool "Hello; world" "a&&b|c<>d&" '*?[ab]' '$HOME $(date) `date`'"#,
                vec![
                    "tool",
                    "Hello; world",
                    "a&&b|c<>d&",
                    "*?[ab]",
                    "$HOME $(date) `date`",
                ],
            ),
            (
                r#"tool \; \& \| \< \> \$HOME \* \? \`date\` "\$HOME" "\`date\`""#,
                vec![
                    "tool", ";", "&", "|", "<", ">", "$HOME", "*", "?", "`date`", "$HOME", "`date`",
                ],
            ),
            (
                r#"tool '' "" pre"fix" '$5' "cost $" foo#bar \#tag \~user [a\] [a"]""#,
                vec![
                    "tool", "", "", "prefix", "$5", "cost $", "foo#bar", "#tag", "~user", "[a]",
                    "[a]",
                ],
            ),
        ] {
            assert_eq!(parse(source).unwrap(), expected, "{source}");
        }
    }

    #[test]
    fn invalid_syntax_and_recognizable_shell_features_are_rejected() {
        for source in [
            "",
            " \t ",
            "'' --flag",
            "\" \" arg",
            "tool 'open",
            "tool \"open",
            "tool \\",
            "tool \"trailing\\",
            "tool && next",
            "tool||next",
            "tool; next",
            "tool | tee out",
            "tool > out",
            "tool 2>>out",
            "tool < in",
            "tool &",
            "tool $(date)",
            "tool `date`",
            "tool $HOME",
            "tool ${HOME}",
            "tool $1",
            "tool \"$HOME\"",
            "tool \"$(date)\"",
            "tool \"`date`\"",
            "ls *.rs",
            "ls file?.rs",
            "ls [ab].rs",
            "tool\nnext",
            "tool # comment",
            "tool ~/file",
        ] {
            assert!(parse(source).is_err(), "accepted {source:?}");
        }
    }

    #[test]
    fn argument_limits_apply_after_tokenization() {
        assert!(parse(&format!("tool {}", "'' ".repeat(MAX_COMMAND_ARGS - 1))).is_ok());
        assert!(parse(&format!("tool {}", "'' ".repeat(MAX_COMMAND_ARGS))).is_err());
        assert!(parse(&format!("tool '{}'", "a".repeat(MAX_ARGUMENT_BYTES))).is_ok());
        assert!(parse(&format!("tool '{}'", "a".repeat(MAX_ARGUMENT_BYTES + 1))).is_err());
        assert!(parse("tool '\0'").is_err());
    }
}
