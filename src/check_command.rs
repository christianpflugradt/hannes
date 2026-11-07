//! Decode string commands once; legacy arrays and execution keep their existing contract.
use serde::{Deserialize, Deserializer, de::Error};

pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Command {
        String(String),
        Array(Vec<String>),
    }
    match Option::<Command>::deserialize(deserializer)? {
        Some(Command::String(command)) => parse(&command).map(Some).map_err(D::Error::custom),
        Some(Command::Array(arguments)) => Ok(Some(arguments)),
        None => Ok(None),
    }
}

fn parse(command: &str) -> Result<Vec<String>, &'static str> {
    let mut arguments = Vec::new();
    let mut argument = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut started = false;
    for c in command.chars() {
        if c.is_control() && c != '\t' {
            return Err("check.command: control characters other than tab are unsupported");
        }
        if escaped {
            argument.push(c);
            escaped = false;
        } else if c == '\\' && quote != Some('\'') {
            escaped = true;
            started = true;
        } else if quote == Some(c) {
            quote = None;
        } else if quote != Some('\'') && matches!(c, '$' | '`') {
            return Err(
                "check.command: variable expansion and substitution are unsupported; quote with single quotes or escape literal characters",
            );
        } else if quote.is_some() {
            argument.push(c);
        } else if matches!(c, '\'' | '"') {
            quote = Some(c);
            started = true;
        } else if matches!(c, ' ' | '\t') {
            if started {
                arguments.push(std::mem::take(&mut argument));
                started = false;
            }
        } else if matches!(c, '|' | '&' | ';' | '<' | '>' | '(' | ')' | '#') {
            return Err(
                "check.command: shell operators and comments are unsupported; quote or escape literal characters",
            );
        } else {
            argument.push(c);
            started = true;
        }
    }
    if escaped {
        return Err("check.command: trailing escape");
    }
    if quote.is_some() {
        return Err("check.command: unclosed quote");
    }
    if started {
        arguments.push(argument);
    }
    Ok(arguments)
}
