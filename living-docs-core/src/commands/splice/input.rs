//! The allow-lists every byte a splice writes passes: the Mermaid text, the
//! marker's command and the input globs. Each is checked against what is
//! allowed, never against known-bad shapes, because a Markdown renderer reads
//! line endings, fences and comments more loosely than this crate's parser.

/// Characters a glob may hold besides ASCII letters and digits: the path
/// separator, the wildcards, and the marks real file names carry (`-`, `_`,
/// `.`, and `@`, `+`, `~` from package and version names). Anything else, a
/// comma, a pipe, a space or a `>`, could end the marker comment or split the
/// stamped glob list.
const GLOB_MARKS: &str = "/*?-_.@+~";

const EMPTY: &str = "the Mermaid text is empty; pipe the diagram in on standard input";

/// Sequences that would end (`-->` and the HTML5 `--!>`) or reopen the marker
/// comment or its fields.
const MARKER_SEQUENCES: [&str; 5] = ["-->", "--!>", "<!--", " | inputs: ", " | sha256: "];

/// Whether `c` is a character no text written into a marker or a region may
/// hold: a control character, or a Unicode line or paragraph separator, which
/// some renderers read as a line ending.
fn is_unsafe(c: char) -> bool {
    c.is_control() || c == '\u{2028}' || c == '\u{2029}'
}

/// The Mermaid text as the region's fenced block: blank edge lines dropped.
///
/// A line ending is `\n` or `\r\n`; any other carriage return or control
/// character (a tab excepted) is refused, because CommonMark reads a bare
/// carriage return as a line ending the line scan does not see. The block may
/// carry its own fence only as a first line that is exactly a mermaid fence
/// and a last line that is exactly a closing fence, both or neither. No other
/// line may start, after leading spaces and tabs, with a backtick, a tilde or
/// `<`: Mermaid syntax never starts a line with them, and they are what a
/// fence, a comment or a marker line starts with. Each refusal names its line.
pub(super) fn fenced(text: &str) -> Result<String, String> {
    let text = text.replace("\r\n", "\n");
    let lines: Vec<&str> = text.lines().collect();
    refuse_control(&lines)?;
    let first = lines.iter().position(|line| !line.trim().is_empty());
    let last = lines.iter().rposition(|line| !line.trim().is_empty());
    let (Some(first), Some(last)) = (first, last) else {
        return Err(EMPTY.to_string());
    };
    let own_fence =
        first < last && lines[first].trim() == "```mermaid" && lines[last].trim() == "```";
    let inner = (first + usize::from(own_fence))..(last + 1 - usize::from(own_fence));
    for index in inner.clone() {
        let start = lines[index].trim_start_matches([' ', '\t']);
        if start.starts_with(['`', '~', '<']) {
            return Err(refused(
                index,
                "a line cannot start with a backtick, a tilde or `<`",
            ));
        }
    }
    let body = lines[inner].join("\n");
    match body.trim().is_empty() {
        true => Err(EMPTY.to_string()),
        false => Ok(format!("```mermaid\n{body}\n```")),
    }
}

/// Refuses the first character no line may hold, a tab excepted.
fn refuse_control(lines: &[&str]) -> Result<(), String> {
    for (index, line) in lines.iter().enumerate() {
        if let Some(bad) = line.chars().find(|c| is_unsafe(*c) && *c != '\t') {
            return Err(refused(
                index,
                &format!("the character {bad:?} is not allowed"),
            ));
        }
    }
    Ok(())
}

fn refused(index: usize, reason: &str) -> String {
    format!("line {}: {reason} in the Mermaid text", index + 1)
}

/// Refuses a command that is not one line of printable text, or that holds a
/// sequence ending or reopening the marker comment or its fields.
pub(super) fn check_command(command: &str) -> Result<(), String> {
    let sequence = MARKER_SEQUENCES.iter().find(|s| command.contains(**s));
    if command.is_empty() || command.chars().any(is_unsafe) || sequence.is_some() {
        return Err(format!(
            "the command must be one line of printable text with none of {}; the binary never runs it",
            MARKER_SEQUENCES.map(|s| format!("`{}`", s.trim())).join(", ")
        ));
    }
    Ok(())
}

/// Refuses a glob holding a character outside ASCII letters, digits and
/// [`GLOB_MARKS`], naming the glob.
pub(super) fn check_globs(globs: &[String]) -> Result<(), String> {
    for glob in globs {
        let allowed = |c: char| c.is_ascii_alphanumeric() || GLOB_MARKS.contains(c);
        if let Some(bad) = glob.chars().find(|c| !allowed(*c)) {
            return Err(format!(
                "glob `{glob}` holds {bad:?}; a glob may hold only letters, digits and `{GLOB_MARKS}`"
            ));
        }
    }
    Ok(())
}
