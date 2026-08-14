use super::*;
use regex::Regex;

fn body_after_frontmatter(template: &str) -> String {
    let mut dashes_seen = 0;
    let mut past_frontmatter = false;
    let mut lines = Vec::new();

    for line in template.lines() {
        if past_frontmatter {
            lines.push(line);
            continue;
        }
        if line.trim_end() == "---" {
            dashes_seen += 1;
            past_frontmatter = dashes_seen == 2;
        }
    }

    lines.join("\n")
}

fn skip_first_title_heading(body: &str) -> String {
    let heading = Regex::new(r"^#{1,2} ").expect("valid heading regex");
    let mut heading_skipped = false;
    let mut lines = Vec::new();

    for line in body.lines() {
        if !heading_skipped && heading.is_match(line) {
            heading_skipped = true;
            continue;
        }
        lines.push(line);
    }

    lines.join("\n")
}

fn strip_html_comments(body: &str) -> String {
    Regex::new(r"(?s)<!--.*?-->")
        .expect("valid html comment regex")
        .replace_all(body, "")
        .into_owned()
}

fn strip_fenced_code_blocks(body: &str) -> String {
    Regex::new(r"(?s)```.*?```")
        .expect("valid fenced code block regex")
        .replace_all(body, "")
        .into_owned()
}

fn strip_inline_code_spans(body: &str) -> String {
    Regex::new(r"`[^`]*`")
        .expect("valid inline code regex")
        .replace_all(body, "")
        .into_owned()
}

/// ADR 0029 fitness function: a numbered type's template comment must
/// name every one of that type's own `status_vocabulary` values and
/// mention Superseded, so the registry and the template comment cannot
/// silently drift apart. Constitution is skipped -- its vocabulary is
/// empty and out of scope (ADR 0029) -- and so is every Named type:
/// a view carries no status and is never superseded (ADR 0036).
#[test]
fn template_comments_agree_with_their_own_status_vocabulary() {
    for spec in DOC_TYPES {
        if !matches!(spec.identity, Identity::Numbered { .. }) {
            continue;
        }

        for value in spec.status_vocabulary {
            assert!(
                spec.template.contains(*value),
                "{} template is missing status_vocabulary value {value:?}",
                spec.token
            );
        }

        assert!(
            spec.template.to_lowercase().contains("uperseded"),
            "{} template must mention Superseded via `living-docs supersede`",
            spec.token
        );
    }
}

/// ADR 0030 fitness function: a template's body may no longer carry the legacy
/// angle-bracket-with-embedded-space placeholder (e.g. `<the choice, in active
/// voice -- specific and testable>`) that made programmatic edits fragile
/// (issue 0022). The scan works on what remains after, in order: (1) the
/// frontmatter block is stripped through the second `---` line inclusive; (2)
/// the first H1/H2 title-heading line is skipped -- that placeholder is out of
/// scope per ADR 0030 rule 5; (3) every HTML comment span (`<!-- ... -->`,
/// non-greedy, may span multiple lines) is stripped, since guidance comments
/// are not placeholders; (4) every fenced code block (triple backtick to the
/// next triple backtick, non-greedy, may span multiple lines) is stripped, so
/// a future Mermaid node label like `A[<foo bar>]` inside a ```mermaid fence
/// can never false-trip the guard; (5) every inline code span (backtick to
/// the next backtick) is stripped, so a worked-example Rust generic like ``
/// `Result<R, E>` `` in a table cell is never mistaken for a placeholder.
/// What is left is searched for any angle-bracket span containing at least
/// one whitespace character; any match fails the assertion, naming the
/// spec's token and the exact matched text, so a future template edit cannot
/// reintroduce the fragile shape.
#[test]
fn fitness_function_no_legacy_angle_bracket_placeholder_survives_in_any_template_body() {
    let placeholder_span = Regex::new(r"<[^<>]*\s[^<>]*>").expect("valid placeholder regex");

    for spec in DOC_TYPES {
        let body = body_after_frontmatter(spec.template);
        let body = skip_first_title_heading(&body);
        let body = strip_html_comments(&body);
        let body = strip_fenced_code_blocks(&body);
        let body = strip_inline_code_spans(&body);

        let legacy_placeholder = placeholder_span.find(&body).map(|m| m.as_str());
        assert!(
            legacy_placeholder.is_none(),
            "{} template still contains a legacy angle-bracket placeholder: {:?}",
            spec.token,
            legacy_placeholder
        );
    }
}

/// Direct unit test for [`strip_fenced_code_blocks`]: proves the helper
/// actually removes a fenced block's contents (not merely that the
/// caller's scan passes), so it stays non-vacuous if the helper is ever
/// reduced to a no-op.
#[test]
fn strip_fenced_code_blocks_removes_a_mermaid_fence_with_an_angle_bracket_span() {
    let body = "Before.\n\n```mermaid\ngraph TD\n  A[<foo bar>] --> B\n```\n\nAfter.";

    let stripped = strip_fenced_code_blocks(body);

    assert!(!stripped.contains("foo bar"));
    assert!(stripped.contains("Before."));
    assert!(stripped.contains("After."));
}

/// Issue 0021 gap: `commands::new::fill_frontmatter_description` only
/// replaces an existing `description:` scalar -- it has no insert path,
/// unlike `describe`'s insert-capable `set_frontmatter_fields`. This
/// fitness function keeps that asymmetry safe by asserting every
/// registered template's frontmatter block already carries a
/// `description:` line for `new` to replace, so a template that ever
/// dropped the line would fail loudly here instead of silently leaving
/// `--description` a no-op.
#[test]
fn every_registered_template_frontmatter_carries_a_description_line() {
    for spec in DOC_TYPES {
        let lines: Vec<&str> = spec.template.lines().collect();
        let close = lines
            .iter()
            .skip(1)
            .position(|&line| line == "---")
            .map(|i| i + 1)
            .unwrap_or_else(|| panic!("{} template has no closing frontmatter '---'", spec.token));

        let has_description_line = lines[..close]
            .iter()
            .any(|line| line.starts_with("description:"));

        assert!(
            has_description_line,
            "{} template frontmatter is missing a description: line",
            spec.token
        );
    }
}
