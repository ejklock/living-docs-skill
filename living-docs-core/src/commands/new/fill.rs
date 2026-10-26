//! Frontmatter and heading fills for `new` scaffolds: targeted line edits of
//! CLI-owned keys inside the leading frontmatter block, plus the record's
//! title heading — never a serde round-trip — so body placeholders survive
//! byte-for-byte.

use crate::doc_type::{self, Identity};
use crate::record::format_scalar;

/// Targeted line-edit fill of `type`/`status`/`timestamp` inside the leading
/// frontmatter block only — never a serde round-trip, so body placeholders
/// and frontmatter guidance comments outside those three keys survive
/// byte-for-byte. `status` is seeded with `type_value`'s own resolved
/// [`doc_type::DocTypeSpec::status_vocabulary`]'s first entry
/// rather than a hardcoded literal, so `new issue "t"` seeds `status: open`
/// while `new adr "t"` still seeds `status: Proposed`.
pub(crate) fn fill_frontmatter(template: &str, type_value: &str, timestamp: &str) -> String {
    let status_value = seed_status_for(type_value, template);
    map_frontmatter_lines(template, |line| {
        fill_frontmatter_line(line, type_value, &status_value, timestamp)
    })
}

/// Rewrites every line strictly inside the leading frontmatter block through
/// `edit`, leaving the fences and the body untouched; content without a
/// closing fence comes back unchanged.
fn map_frontmatter_lines(content: &str, edit: impl Fn(&str) -> String) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let Some(close) = frontmatter_close_index(&lines) else {
        return content.to_string();
    };
    let filled: Vec<String> = lines
        .iter()
        .enumerate()
        .map(|(i, &line)| {
            if i == 0 || i >= close {
                line.to_string()
            } else {
                edit(line)
            }
        })
        .collect();
    filled.join("\n") + "\n"
}

/// The `status:` value a fresh record of `type_value`'s doc type should seed,
/// resolved from that type's own registry row rather than a hardcoded
/// literal. A type with an empty vocabulary (the constitution)
/// keeps the `status:` literal its own `template` carries; `"Proposed"` is
/// the last resort when the type is unregistered or the template has none.
fn seed_status_for(type_value: &str, template: &str) -> String {
    doc_type::spec_for_frontmatter(type_value)
        .and_then(|spec| match spec.status_vocabulary.first() {
            Some(first) => Some((*first).to_string()),
            None => template_status_literal(template),
        })
        .unwrap_or_else(|| "Proposed".to_string())
}

/// The value of the template's frontmatter `status:` line, without its
/// trailing `# guidance comment`; `None` when absent or empty.
fn template_status_literal(template: &str) -> Option<String> {
    let lines: Vec<&str> = template.lines().collect();
    let close = frontmatter_close_index(&lines)?;
    let value = lines[1..close]
        .iter()
        .find_map(|line| line.strip_prefix("status:"))?;
    let value = value.split('#').next().unwrap_or("").trim();
    (!value.is_empty()).then(|| value.to_string())
}

pub(crate) fn frontmatter_close_index(lines: &[&str]) -> Option<usize> {
    lines
        .iter()
        .skip(1)
        .position(|&l| l == "---")
        .map(|i| i + 1)
}

/// Fills the frontmatter `title:` line with `title`, quoted exactly as
/// [`crate::record::to_canonical_markdown`] would (via
/// [`format_scalar`]) — never a local quoting rule — so a fresh scaffold's
/// frontmatter is already a canonical-check fixed point.
pub(crate) fn fill_frontmatter_title(content: &str, title: &str) -> String {
    map_frontmatter_lines(content, |line| {
        replace_targeted_value(line, "title", &format_scalar(title))
            .unwrap_or_else(|| line.to_string())
    })
}

/// Fills the frontmatter `description:` line with `description`, quoted via
/// [`format_scalar`] exactly as [`fill_frontmatter_title`] fills `title:` —
/// the CLI-owned counterpart to hand-editing the placeholder.
/// Delegates to [`crate::commands::supersede::apply_frontmatter_field`], the
/// same insert-or-replace primitive `describe` uses, so a template missing a
/// `description:` line gets one inserted rather than the value being
/// silently dropped. A `None` `description` is a deliberate no-op: `content`
/// returns unchanged and today's placeholder behavior stays intact.
pub(crate) fn fill_frontmatter_description(content: &str, description: Option<&str>) -> String {
    let Some(description) = description else {
        return content.to_string();
    };

    crate::commands::supersede::apply_frontmatter_field(
        content,
        "description",
        &format_scalar(description),
    )
    .unwrap_or_else(|| content.to_string())
}

/// Fills the frontmatter `owner:` field with `owner`, quoted via
/// [`format_scalar`], inserted immediately after the `description:` line —
/// its canonical position, matching
/// [`crate::record::to_canonical_markdown`]'s fixed field order, so a fresh
/// scaffold's frontmatter is already a canonical-check fixed point exactly
/// as [`fill_frontmatter_description`] achieves for `description:` itself.
/// A `None` `owner` is a deliberate no-op: `content` returns unchanged and
/// the record carries no `owner:` field at all.
pub(crate) fn fill_frontmatter_owner(content: &str, owner: Option<&str>) -> String {
    let Some(owner) = owner else {
        return content.to_string();
    };

    insert_line_after_description(content, &format!("owner: {}", format_scalar(owner)))
        .unwrap_or_else(|| content.to_string())
}

/// Inserts `line` right after the frontmatter block's `description:` line,
/// `None` when the block has no closing fence or no `description:` line to
/// anchor on.
fn insert_line_after_description(content: &str, line: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let close = frontmatter_close_index(&lines)?;
    let description_offset = lines[1..close]
        .iter()
        .position(|candidate| candidate.starts_with("description:"))?;
    let insert_at = description_offset + 2;

    let mut updated: Vec<String> = lines.iter().map(|&l| l.to_string()).collect();
    updated.insert(insert_at, line.to_string());
    Some(updated.join("\n") + "\n")
}

fn fill_frontmatter_line(
    line: &str,
    type_value: &str,
    status_value: &str,
    timestamp: &str,
) -> String {
    replace_targeted_value(line, "type", type_value)
        .or_else(|| replace_targeted_value(line, "status", status_value))
        .or_else(|| replace_targeted_value(line, "timestamp", timestamp))
        .unwrap_or_else(|| line.to_string())
}

/// Replaces the value of a `key: value` frontmatter line, preserving any
/// trailing `# guidance comment` verbatim.
pub(crate) fn replace_targeted_value(line: &str, key: &str, new_value: &str) -> Option<String> {
    let prefix = format!("{key}:");
    let rest = line.strip_prefix(&prefix)?;
    match rest.find('#') {
        Some(hash_idx) => Some(format!("{prefix} {new_value} {}", &rest[hash_idx..])),
        None => Some(format!("{prefix} {new_value}")),
    }
}

/// Fills the frontmatter `kind:` line for a Named-identity type from the
/// closed [`doc_type::VIEW_KIND_ORDER`] vocabulary. `None`
/// keeps the template's seed; a kind on any other identity is refused —
/// inserting the key there would invent frontmatter the type's template
/// never declared.
pub(crate) fn fill_frontmatter_kind(
    content: &str,
    spec: &doc_type::DocTypeSpec,
    kind: Option<&str>,
) -> Result<String, String> {
    let Some(kind) = kind else {
        return Ok(content.to_string());
    };
    if !matches!(spec.identity, Identity::Named { .. }) {
        return Err(format!(
            "--kind applies only to architecture views, not '{}'",
            spec.token
        ));
    }
    if !doc_type::VIEW_KIND_ORDER.contains(&kind) {
        return Err(format!(
            "unknown view kind '{kind}' (expected one of {})",
            doc_type::VIEW_KIND_ORDER.join(", ")
        ));
    }
    crate::commands::supersede::apply_frontmatter_field(content, "kind", kind)
        .ok_or_else(|| "template has no frontmatter block to fill `kind` into".to_string())
}

/// Fills the template's title heading — the first `# `/`## ` line after the
/// frontmatter that still carries an angle-bracket placeholder — with
/// `NNNN. <title>` for a numbered record or the bare title otherwise, so the
/// agent never hand-writes the one heading the CLI already knows.
pub(crate) fn fill_heading(content: &str, title: &str, number: Option<u32>) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let Some(close) = frontmatter_close_index(&lines) else {
        return content.to_string();
    };
    let heading = match number {
        Some(number) => format!("{number:04}. {title}"),
        None => title.to_string(),
    };
    let mut filled = false;
    let updated: Vec<String> = lines
        .iter()
        .enumerate()
        .map(|(i, &line)| {
            if filled || i < close || !is_placeholder_heading(line) {
                return line.to_string();
            }
            filled = true;
            let prefix = line.split_whitespace().next().unwrap_or("#");
            format!("{prefix} {heading}")
        })
        .collect();
    updated.join("\n") + "\n"
}

/// Replaces everything below the title heading — the template's slots — with
/// `body`, keeping the frontmatter, the heading and the blank line after it.
/// `None`, or a template with no heading, leaves `content` as it is.
pub(crate) fn fill_body(content: &str, body: Option<&str>) -> String {
    let Some(body) = body else {
        return content.to_string();
    };
    let lines: Vec<&str> = content.lines().collect();
    let close = frontmatter_close_index(&lines).unwrap_or(0);
    let Some(heading) = lines
        .iter()
        .skip(close + 1)
        .position(|l| l.starts_with('#'))
    else {
        return content.to_string();
    };
    let kept = lines[..=close + 1 + heading].join("\n");
    format!("{kept}\n\n{}\n", body.trim_end())
}

fn is_placeholder_heading(line: &str) -> bool {
    (line.starts_with("# ") || line.starts_with("## ")) && line.contains('<') && line.contains('>')
}

#[cfg(test)]
mod tests;
