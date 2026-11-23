//! Reads a record line by line for the `###` rule each line sits under, and
//! reads the marker line and the pipe table of a marked decision table. Every
//! malformed input comes back as one `Err` naming the problem.

use crate::fence::{toggle_fence, Fence};

/// The level and text of a markdown ATX heading line.
fn heading(line: &str) -> Option<(usize, &str)> {
    let level = line.chars().take_while(|c| *c == '#').count();
    let text = line[level..].strip_prefix(' ')?;
    (1..=6).contains(&level).then_some((level, text.trim()))
}

/// The `###` rule and the open code fence while lines are read in order.
#[derive(Default)]
pub(super) struct Scope<'a> {
    rule: Option<&'a str>,
    fence: Fence,
}

impl<'a> Scope<'a> {
    /// The rule `line` sits under; none for a heading, a fence line, a line
    /// inside a fence, or a line before the first `###` rule.
    pub(super) fn rule_of(&mut self, line: &'a str) -> Option<&'a str> {
        if toggle_fence(line, &mut self.fence) || self.fence.is_some() {
            return None;
        }
        if let Some((level, text)) = heading(line) {
            self.rule = match level {
                3 => Some(text),
                1 | 2 => None,
                _ => self.rule,
            };
            return None;
        }
        self.rule
    }
}

/// Which rows count when several match one combination.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Policy {
    /// No combination may match two rows with different outcomes.
    Unique,
    /// The first matching row wins; later rows may overlap earlier ones.
    First,
}

/// A condition column: its name and the values it can take.
pub(super) struct Condition {
    pub name: String,
    pub domain: Vec<String>,
}

/// What a row says about one condition.
pub(super) enum Cell {
    Any,
    Values(Vec<String>),
}

pub(super) struct Row {
    pub cells: Vec<Cell>,
    pub outcome: String,
}

pub(super) struct Table {
    pub conditions: Vec<Condition>,
    pub rows: Vec<Row>,
}

const MARKER: &str = "<!-- decision-table";

/// `None` when `line` is not a decision-table marker; otherwise its policy, or
/// the problem with the marker. A line indented four or more spaces, or by a
/// tab, is indented code, so its marker and the table below it are not read.
pub(super) fn parse_marker(line: &str) -> Option<Result<Policy, String>> {
    if line.starts_with('\t') || line.starts_with("    ") {
        return None;
    }
    let rest = line.trim().strip_prefix(MARKER)?.trim_start();
    if rest.starts_with("-->") {
        return Some(Ok(Policy::Unique));
    }
    if rest.is_empty() {
        return Some(Err(
            "the decision-table marker is not closed with `-->`".to_owned()
        ));
    }
    let named = rest.strip_prefix(':')?;
    let Some(name) = named.trim_end().strip_suffix("-->") else {
        return Some(Err(
            "the decision-table marker is not closed with `-->`".to_owned()
        ));
    };
    Some(match name.trim() {
        "first" => Ok(Policy::First),
        "" => {
            Err("the decision-table marker names an empty policy; write `first` or none".to_owned())
        }
        other => Err(format!(
            "the decision-table marker names the unknown policy '{other}'; write `first` or none"
        )),
    })
}

/// The cells of one table line, trimmed, without the outer pipes. A pipe
/// escaped as `\|` stays in its cell, written as is; a closing pipe that is not
/// escaped ends the last cell.
fn split_cells(line: &str) -> Vec<&str> {
    let trimmed = line.trim();
    let trimmed = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let mut cells = Vec::new();
    let (mut start, mut escaped) = (0, false);
    for (at, c) in trimmed.char_indices() {
        if c == '|' && !escaped {
            cells.push(trimmed[start..at].trim());
            start = at + 1;
        }
        escaped = c == '\\' && !escaped;
    }
    let closed_by_a_pipe = start == trimmed.len() && start > 0;
    if !closed_by_a_pipe {
        cells.push(trimmed[start..].trim());
    }
    cells
}

/// The comma-separated values of `text`, trimmed, empty ones dropped.
fn values_of(text: &str) -> Vec<String> {
    let mut values: Vec<String> = Vec::new();
    for value in text.split(',').map(str::trim).filter(|v| !v.is_empty()) {
        if !values.iter().any(|seen| seen == value) {
            values.push(value.to_owned());
        }
    }
    values
}

fn parse_condition(header: &str) -> Result<Condition, String> {
    let shape = || format!("condition column '{header}' must read `name (v1, v2, ...)`");
    let open = header.rfind('(').ok_or_else(shape)?;
    let (name, rest) = (&header[..open], &header[open + 1..]);
    let list = rest.trim_end().strip_suffix(')').ok_or_else(shape)?;
    let name = name.trim();
    let domain = values_of(list);
    if name.is_empty() {
        return Err(shape());
    }
    if domain.is_empty() {
        return Err(format!("condition column '{name}' declares no values"));
    }
    Ok(Condition {
        name: name.to_owned(),
        domain,
    })
}

fn is_separator(line: &str) -> bool {
    let cells = split_cells(line);
    cells
        .iter()
        .all(|c| c.contains('-') && c.chars().all(|ch| matches!(ch, '-' | ':')))
}

fn parse_row(number: usize, line: &str, width: usize) -> Result<Row, String> {
    let cells = split_cells(line);
    if cells.len() != width {
        return Err(format!(
            "row {number} has {} cells but the header has {width}",
            cells.len()
        ));
    }
    if cells.iter().any(|c| c.is_empty()) {
        return Err(format!("row {number} has an empty cell; write `-` for any"));
    }
    let (outcome, conditions) = cells.split_last().ok_or("empty row")?;
    let cells = conditions
        .iter()
        .map(|c| match *c {
            "-" => Cell::Any,
            set => Cell::Values(values_of(set)),
        })
        .collect();
    Ok(Row {
        cells,
        outcome: (*outcome).to_owned(),
    })
}

/// The table made of `lines`: a header, a separator row and the data rows.
pub(super) fn parse_table(lines: &[&str]) -> Result<Table, String> {
    let header = split_cells(
        lines
            .first()
            .ok_or("the marker is not followed by a table")?,
    );
    let Some((_, conditions)) = header.split_last().filter(|(_, rest)| !rest.is_empty()) else {
        return Err("the table needs a condition column before its outcome column".to_owned());
    };
    let conditions = conditions
        .iter()
        .map(|h| parse_condition(h))
        .collect::<Result<Vec<_>, _>>()?;
    if !lines.get(1).is_some_and(|l| is_separator(l)) {
        return Err("the table has no separator row under its header".to_owned());
    }
    let rows = lines[2..]
        .iter()
        .enumerate()
        .map(|(i, line)| parse_row(i + 1, line, header.len()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Table { conditions, rows })
}
