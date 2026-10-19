//! Reads record prose handed to `new --body` and `set --from`: the one place
//! the CLI touches a file or standard input for text, so core only ever
//! receives a `&str`.

use std::fs;
use std::io::{self, IsTerminal, Read};

/// The text of `spec`: standard input for `-`, otherwise the file at that
/// path, with CRLF line endings normalized to LF. Standard input on a
/// terminal is refused rather than left to block on a keyboard.
pub(crate) fn read_source(spec: &str) -> Result<String, String> {
    let stdin = io::stdin();
    let terminal = stdin.is_terminal();
    read_source_with(spec, terminal, &mut stdin.lock())
}

/// A record body: non-empty, not opening with a frontmatter fence, ending
/// with exactly one newline.
pub(crate) fn read_body(spec: &str) -> Result<String, String> {
    body_from(&read_source(spec)?)
}

/// A field value: the source text trimmed of surrounding whitespace.
pub(crate) fn read_value(spec: &str) -> Result<String, String> {
    Ok(read_source(spec)?.trim().to_string())
}

fn read_source_with(spec: &str, terminal: bool, stdin: &mut dyn Read) -> Result<String, String> {
    let mut text = String::new();
    if spec == "-" {
        if terminal {
            return Err(
                "standard input is a terminal; pipe the text in (`... | living-docs ... -`) or pass a file path"
                    .to_string(),
            );
        }
        stdin
            .read_to_string(&mut text)
            .map_err(|e| format!("cannot read standard input: {e}"))?;
    } else {
        text = fs::read_to_string(spec).map_err(|e| format!("cannot read {spec}: {e}"))?;
    }
    Ok(text.replace("\r\n", "\n"))
}

fn body_from(text: &str) -> Result<String, String> {
    if text.trim().is_empty() {
        return Err("the body is empty; nothing to write below the heading".to_string());
    }
    if text
        .lines()
        .next()
        .is_some_and(|line| line.trim_end() == "---")
    {
        return Err(
            "the body opens with `---`; frontmatter is CLI-owned, give only the text below the heading"
                .to_string(),
        );
    }
    Ok(format!("{}\n", text.trim_end()))
}

#[cfg(test)]
mod tests;
