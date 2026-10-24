//! The hint for a `set` whose record reference was typed as two words, a type
//! then a number (`set <type> <number> <key> …`). The CLI never runs that
//! form; the hint shows the same command with the pair joined as
//! `type/number`, built from the process argv and quoted for a POSIX shell.

use std::ffi::OsString;

/// Global options that take their value as the next argv token.
const VALUE_FLAGS: [&str; 2] = ["--docs-dir", "--color"];

/// The one-line hint for a split `set` reference, or `None` unless the verb
/// is `set`, the first word after it is a registered numbered type (its token
/// or directory name) and the second is all digits. Global options before the
/// verb are kept.
pub(crate) fn hint(argv: &[OsString]) -> Option<String> {
    let words: Vec<&str> = argv
        .iter()
        .skip(1)
        .map(|a| a.to_str())
        .collect::<Option<_>>()?;
    let verb = first_positional(&words)?;
    if words[verb] != "set" {
        return None;
    }
    let (token, number) = (*words.get(verb + 1)?, *words.get(verb + 2)?);
    let numeric = !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit());
    if !numeric || living_docs_core::paths::dir_for(token).is_none() {
        return None;
    }
    let joined = format!("{token}/{number}");
    let mut fixed = words.clone();
    fixed.splice(verb + 1..verb + 3, [joined.as_str()]);
    let command: Vec<String> = fixed.iter().map(|word| shell_quote(word)).collect();
    Some(format!(
        "hint: write the reference as one token: living-docs {}",
        command.join(" ")
    ))
}

/// The word as one POSIX shell word: bare when it is made only of safe
/// characters, otherwise single-quoted with each `'` written as `'\''`.
fn shell_quote(word: &str) -> String {
    let safe = |c: char| c.is_ascii_alphanumeric() || "_./:=@%+,-".contains(c);
    if !word.is_empty() && word.chars().all(safe) {
        word.to_string()
    } else {
        format!("'{}'", word.replace('\'', r"'\''"))
    }
}

/// Index of the first word that is neither a global option nor its value.
fn first_positional(words: &[&str]) -> Option<usize> {
    let mut index = 0;
    while let Some(&word) = words.get(index) {
        match word {
            "--" => return None,
            _ if VALUE_FLAGS.contains(&word) => index += 2,
            _ if word.starts_with('-') => index += 1,
            _ => return Some(index),
        }
    }
    None
}
