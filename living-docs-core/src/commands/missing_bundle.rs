//! The one place the "bundle root not found" refusal is worded, so every
//! front reports a missing bundle the same way and differs only in guidance.

use std::path::Path;

const TOOL_GUIDANCE: &str = "start the server with `--docs-dir` naming the bundle, \
or create the bundle with the `write` operation `new`";

/// The refusal for a bundle root that does not exist (or is not a directory).
pub fn message(bundle: &Path) -> String {
    format!("bundle root not found: {}", bundle.display())
}

/// [`message`] followed by what a tool caller can do about it.
pub fn for_tool(bundle: &Path) -> String {
    format!("{}\n{TOOL_GUIDANCE}", message(bundle))
}

/// Refuses with [`for_tool`] unless `bundle` is a directory.
pub fn require_dir_for_tool(bundle: &Path) -> Result<(), String> {
    if bundle.is_dir() {
        Ok(())
    } else {
        Err(for_tool(bundle))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tool_text_starts_with_the_plain_message_and_adds_guidance() {
        let bundle = Path::new("/nowhere/docs");

        assert_eq!(message(bundle), "bundle root not found: /nowhere/docs");
        let text = for_tool(bundle);
        let (first, guidance) = text.split_once('\n').expect("two lines");
        assert_eq!(first, message(bundle));
        assert!(guidance.contains("--docs-dir"), "{guidance}");
        assert!(
            guidance.contains("`write`") && guidance.contains("`new`"),
            "{guidance}"
        );
    }
}
