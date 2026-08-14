//! `new --json` section fill (ADR 0038): the payload is a flat JSON object
//! whose keys must match the template's own body headings (every heading
//! after the title heading), plus the reserved key `Intro` for the span
//! between the title heading and the first section heading. Named sections
//! are filled in place of the template guidance; unnamed ones keep it. The
//! title heading itself is filled from the record's title (and `NNNN` when
//! the template's title heading carries the marker).

use serde_json::Value;

pub(crate) const INTRO_KEY: &str = "Intro";

pub(crate) fn fill_sections(
    content: &str,
    payload: &str,
    title: &str,
    number: Option<u32>,
) -> Result<String, String> {
    let sections = parse_payload(payload)?;
    let lines: Vec<&str> = content.lines().collect();
    let headings = heading_indices(&lines);
    let Some((&title_idx, section_headings)) = headings.split_first() else {
        return Err("template body has no title heading to fill".to_string());
    };
    validate_keys(&sections, &lines, section_headings)?;

    let mut out: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
    out[title_idx] = filled_title_heading(lines[title_idx], title, number);
    Ok(splice_sections(
        out,
        &lines,
        title_idx,
        section_headings,
        &sections,
    ))
}

fn splice_sections(
    out: Vec<String>,
    lines: &[&str],
    title_idx: usize,
    section_headings: &[usize],
    sections: &std::collections::BTreeMap<String, String>,
) -> String {
    let mut result: Vec<String> = out[..=title_idx].to_vec();
    let mut spans = Vec::new();
    let first_section = section_headings.first().copied().unwrap_or(lines.len());
    spans.push((INTRO_KEY.to_string(), title_idx + 1, first_section));
    for (i, &at) in section_headings.iter().enumerate() {
        let end = section_headings.get(i + 1).copied().unwrap_or(lines.len());
        spans.push((heading_key(lines[at]), at, end));
    }
    for (key, start, end) in spans {
        emit_span(&mut result, &out, sections, &key, start, end);
    }
    result.join("\n") + "\n"
}

fn emit_span(
    result: &mut Vec<String>,
    out: &[String],
    sections: &std::collections::BTreeMap<String, String>,
    key: &str,
    start: usize,
    end: usize,
) {
    let is_intro = key == INTRO_KEY;
    if !is_intro {
        result.push(out[start].clone());
    }
    let body_start = if is_intro { start } else { start + 1 };
    match sections.get(key) {
        Some(value) => {
            result.push(String::new());
            result.push(value.trim().to_string());
            result.push(String::new());
        }
        None => result.extend(out[body_start..end].iter().cloned()),
    }
}

fn parse_payload(payload: &str) -> Result<std::collections::BTreeMap<String, String>, String> {
    let value: Value = serde_json::from_str(payload)
        .map_err(|e| format!("--json payload is not valid JSON: {e}"))?;
    let Value::Object(map) = value else {
        return Err("--json payload must be a JSON object of section -> text".to_string());
    };
    map.into_iter()
        .map(|(key, value)| match value {
            Value::String(text) => Ok((key, text)),
            _ => Err(format!("--json section {key:?} must be a string")),
        })
        .collect()
}

fn validate_keys(
    sections: &std::collections::BTreeMap<String, String>,
    lines: &[&str],
    section_headings: &[usize],
) -> Result<(), String> {
    let known: Vec<String> = section_headings
        .iter()
        .map(|&at| heading_key(lines[at]))
        .collect();
    for key in sections.keys() {
        if key != INTRO_KEY && !known.contains(key) {
            return Err(format!(
                "--json names unknown section {key:?} (this type's sections: {INTRO_KEY}, {})",
                known.join(", ")
            ));
        }
    }
    Ok(())
}

fn heading_indices(lines: &[&str]) -> Vec<usize> {
    let close = frontmatter_close(lines);
    lines
        .iter()
        .enumerate()
        .skip(close)
        .filter(|(_, line)| line.starts_with('#'))
        .map(|(i, _)| i)
        .collect()
}

fn frontmatter_close(lines: &[&str]) -> usize {
    if lines.first() != Some(&"---") {
        return 0;
    }
    lines
        .iter()
        .skip(1)
        .position(|&l| l == "---")
        .map_or(0, |i| i + 2)
}

fn heading_key(line: &str) -> String {
    line.trim_start_matches('#').trim().to_string()
}

/// The title heading, preserving the template's heading level: a template
/// marker containing `NNNN` gets `{hashes} {number:04}. {title}`, any other
/// shape gets `{hashes} {title}`.
fn filled_title_heading(template_line: &str, title: &str, number: Option<u32>) -> String {
    let hashes: String = template_line.chars().take_while(|c| *c == '#').collect();
    match number.filter(|_| template_line.contains("NNNN")) {
        Some(n) => format!("{hashes} {n:04}. {title}"),
        None => format!("{hashes} {title}"),
    }
}

#[cfg(test)]
mod tests;
