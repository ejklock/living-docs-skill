//! Output-mode and color resolution shared by every data verb (ADR 0060):
//! JSON off a TTY, colored plain text on one, honoring `--json`/`--plain`,
//! `--color`/`NO_COLOR`, and `--quiet` for informational stderr lines.

use clap::ValueEnum;
use std::fmt;
use std::io::{self, IsTerminal, Write};
use std::process;

/// Whether a data verb's success output is minified JSON or human text.
/// `--json`/`--plain` are explicit overrides that always win; with neither
/// given, a TTY defaults to text and anything else (piped, an agent) to
/// JSON. Errors are unaffected by this mode — they always print plain text
/// to stderr.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OutputMode {
    Text,
    Json,
}

impl OutputMode {
    /// `color_always` is an explicit `--color=always`, which implies text
    /// mode the same way `--plain` does — a piped `check --color=always`
    /// prints colored text, not JSON (ADR 0060).
    pub(crate) fn resolve(
        json: bool,
        plain: bool,
        color_always: bool,
        stdout_is_tty: bool,
    ) -> Self {
        if json {
            return Self::Json;
        }
        if plain || color_always || stdout_is_tty {
            Self::Text
        } else {
            Self::Json
        }
    }

    pub(crate) fn from_flags(json: bool, plain: bool, color: ColorChoice) -> Self {
        Self::resolve(
            json,
            plain,
            color == ColorChoice::Always,
            std::io::stdout().is_terminal(),
        )
    }

    pub(crate) fn is_json(self) -> bool {
        self == Self::Json
    }
}

/// The `--color` flag's vocabulary (clig.dev convention).
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub(crate) enum ColorChoice {
    Auto,
    Always,
    Never,
}

/// Whether ANSI color escapes are emitted. `Auto` follows the TTY and
/// `NO_COLOR` (any non-empty value disables it, per the NO_COLOR convention);
/// `Always`/`Never` override both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ColorMode {
    On,
    Off,
}

impl ColorMode {
    pub(crate) fn resolve(choice: ColorChoice, no_color_env: bool, stdout_is_tty: bool) -> Self {
        match choice {
            ColorChoice::Always => Self::On,
            ColorChoice::Never => Self::Off,
            ColorChoice::Auto if no_color_env || !stdout_is_tty => Self::Off,
            ColorChoice::Auto => Self::On,
        }
    }

    pub(crate) fn from_choice(choice: ColorChoice) -> Self {
        let no_color = std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
        Self::resolve(choice, no_color, std::io::stdout().is_terminal())
    }
}

/// The palette `paint` supports — just enough for `check`'s report (red
/// violations, yellow advisories, a green or red verdict line).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Style {
    Red,
    Yellow,
    Green,
}

/// Wraps `text` in `style`'s ANSI escape under [`ColorMode::On`]; returns
/// `text` unchanged under [`ColorMode::Off`]. Plain ANSI, no color crate.
pub(crate) fn paint(mode: ColorMode, style: Style, text: &str) -> String {
    if mode == ColorMode::Off {
        return text.to_owned();
    }
    let code = match style {
        Style::Red => "31",
        Style::Yellow => "33",
        Style::Green => "32",
    };
    format!("\x1b[{code}m{text}\x1b[0m")
}

/// Serializes `value` to minified JSON, matching every embedded-corpus JSON
/// helper's shape. Falls back to a JSON error envelope on a serialization
/// bug rather than panicking — `value`'s fields are always plain owned data,
/// so this only fires if a future field stops being serializable.
pub(crate) fn to_json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|err| format!("{{\"error\":\"failed to serialize JSON: {err}\"}}"))
}

/// How the process ends after a stdout write failed.
#[derive(Debug, PartialEq, Eq)]
struct Exit {
    code: i32,
    message: Option<String>,
}

/// The one place that decides what a stdout write result means: a reader that
/// closed the pipe chose to stop, so nothing failed; any other error exits 1
/// naming its cause. `None` means the write succeeded.
fn exit_for(result: &io::Result<()>) -> Option<Exit> {
    let err = result.as_ref().err()?;
    Some(if err.kind() == io::ErrorKind::BrokenPipe {
        Exit {
            code: 0,
            message: None,
        }
    } else {
        Exit {
            code: 1,
            message: Some(format!("failed writing to stdout: {err}")),
        }
    })
}

fn finish(result: io::Result<()>) {
    if let Some(exit) = exit_for(&result) {
        if let Some(message) = exit.message {
            eprintln!("{message}");
        }
        process::exit(exit.code);
    }
}

/// Runs `write` on `out` then flushes, so a failure surfaces here instead of
/// at the exit-time flush, where std swallows it.
fn emit<W: Write>(out: &mut W, write: impl FnOnce(&mut W) -> io::Result<()>) -> io::Result<()> {
    write(out)?;
    out.flush()
}

/// The single stdout writer; a closed pipe exits 0 silently, any other error 1.
pub(crate) fn write_stdout(args: fmt::Arguments<'_>) {
    finish(emit(&mut io::stdout().lock(), |out| out.write_fmt(args)));
}

/// Writes raw bytes through the same decision as [`write_stdout`].
pub(crate) fn write_stdout_bytes(bytes: &[u8]) {
    finish(emit(&mut io::stdout().lock(), |out| out.write_all(bytes)));
}

/// Stdout counterpart of the std println macro, routed through [`write_stdout`].
macro_rules! outln {
    () => { $crate::output::write_stdout(format_args!("\n")) };
    ($($arg:tt)*) => {
        $crate::output::write_stdout(format_args!("{}\n", format_args!($($arg)*)))
    };
}

pub(crate) use outln;

/// Prints an informational line to stderr unless `--quiet` was given —
/// warnings and progress notes, never errors (which always print).
pub(crate) fn note(quiet: bool, message: &str) {
    if !quiet {
        eprintln!("{message}");
    }
}

#[cfg(test)]
#[path = "output/tests.rs"]
mod tests;
