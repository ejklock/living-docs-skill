use super::{check_decision_table, Reporter};
use crate::line_ending::LineEndingStore;
use crate::test_support::MapStore;
use std::path::Path;

const PATH: &str = "context/billing/fees.md";

fn record(body: &str) -> String {
    format!(
        "---\ntype: Context\ntitle: Fees\n---\n\n# Fees\n\n## Rules\n\n### Late fee\n\n{body}\n"
    )
}

fn run_at(path: &str, contents: &str) -> Vec<(String, String)> {
    run_in("", path, contents)
}

fn run_in(bundle: &str, path: &str, contents: &str) -> Vec<(String, String)> {
    let (inner, all_md) = MapStore::seeded(&[(path, contents)]);
    let store = LineEndingStore::new(&inner, true);
    let mut reporter = Reporter::new();
    check_decision_table(&store, Path::new(bundle), &all_md, &mut reporter);
    reporter
        .advisories
        .into_iter()
        .map(|a| (a.kind, a.message))
        .collect()
}

/// The messages raised for `body` placed under a `###` rule of a domain record.
fn messages(body: &str) -> Vec<String> {
    let got = run_at(PATH, &record(body));
    assert!(got.iter().all(|(kind, _)| kind == "decision-table"));
    got.into_iter().map(|(_, message)| message).collect()
}

const HEAD: &str = "| tier (gold, basic) | late (yes, no) | fee |\n|---|---|---|\n";

fn table(marker: &str, rows: &[&str]) -> String {
    format!("{marker}\n{HEAD}{}\n", rows.join("\n"))
}

const DEFAULT: &str = "<!-- decision-table -->";
const FIRST: &str = "<!-- decision-table: first -->";

const FULL: [&str; 4] = [
    "| gold | yes | 5 |",
    "| gold | no | 0 |",
    "| basic | yes | 10 |",
    "| basic | no | 0 |",
];

#[test]
fn a_complete_consistent_table_raises_nothing() {
    assert_eq!(messages(&table(DEFAULT, &FULL)), Vec::<String>::new());
}

#[test]
fn a_missing_combination_is_named() {
    let got = messages(&table(DEFAULT, &FULL[..3]));
    assert_eq!(got.len(), 1, "{got:?}");
    assert!(
        got[0].contains("no row covers tier=basic, late=no"),
        "{got:?}"
    );
    assert!(got[0].contains("Late fee"), "{got:?}");
}

#[test]
fn several_missing_combinations_are_one_advisory_with_a_count() {
    let got = messages(&table(DEFAULT, &FULL[..2]));
    assert_eq!(got.len(), 1, "{got:?}");
    assert!(got[0].contains("2 combinations"), "{got:?}");
    assert!(got[0].contains("tier=basic, late=yes"), "{got:?}");
}

#[test]
fn two_rows_with_different_outcomes_on_one_combination_contradict() {
    let rows = [FULL[0], FULL[1], FULL[2], FULL[3], "| gold | - | 7 |"];
    let got = messages(&table(DEFAULT, &rows));
    assert_eq!(got.len(), 2, "{got:?}");
    assert!(got[0].contains("rows 1 and 5"), "{got:?}");
    assert!(got[1].contains("rows 2 and 5"), "{got:?}");
}

#[test]
fn an_overlap_with_the_same_outcome_raises_nothing() {
    let rows = [FULL[0], FULL[1], FULL[2], FULL[3], "| gold | yes | 5 |"];
    assert_eq!(messages(&table(DEFAULT, &rows)), Vec::<String>::new());
}

#[test]
fn outcomes_compare_as_trimmed_text() {
    let rows = [
        "| - | yes | 5 |",
        "| gold, basic | yes |   5  |",
        "| - | no | 0 |",
    ];
    assert_eq!(messages(&table(DEFAULT, &rows)), Vec::<String>::new());
}

#[test]
fn a_set_and_any_expand_to_their_values() {
    let rows = ["| gold, basic | yes | 5 |", "| - | no | 0 |"];
    assert_eq!(messages(&table(DEFAULT, &rows)), Vec::<String>::new());
}

#[test]
fn first_policy_ignores_overlap_between_different_outcomes() {
    let rows = ["| gold | yes | 5 |", "| - | - | 0 |"];
    assert_eq!(messages(&table(FIRST, &rows)), Vec::<String>::new());
}

#[test]
fn first_policy_names_a_row_fully_shadowed_by_earlier_rows() {
    let rows = ["| - | - | 0 |", "| gold | yes | 5 |"];
    let got = messages(&table(FIRST, &rows));
    assert_eq!(got.len(), 1, "{got:?}");
    assert!(got[0].contains("row 2 is fully shadowed"), "{got:?}");
}

#[test]
fn first_policy_accepts_a_partial_shadow() {
    let rows = ["| gold | yes | 5 |", "| gold, basic | - | 0 |"];
    assert_eq!(messages(&table(FIRST, &rows)), Vec::<String>::new());
}

#[test]
fn first_policy_still_requires_every_combination() {
    let rows = ["| gold | - | 0 |", "| basic | no | 0 |"];
    let got = messages(&table(FIRST, &rows));
    assert_eq!(got.len(), 1, "{got:?}");
    assert!(
        got[0].contains("no row covers tier=basic, late=yes"),
        "{got:?}"
    );
}

#[test]
fn a_value_outside_its_domain_is_reported() {
    let rows = ["| silver | yes | 5 |", "| - | - | 0 |"];
    let got = messages(&table(DEFAULT, &rows));
    assert_eq!(got.len(), 1, "{got:?}");
    assert!(
        got[0].contains("'silver'") && got[0].contains("tier"),
        "{got:?}"
    );
    assert!(got[0].contains("row 1"), "{got:?}");
}

fn sized(first: usize, second: usize) -> String {
    let values = |n: usize| {
        (0..n)
            .map(|i| format!("v{i}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "{DEFAULT}\n| a ({}) | b ({}) | out |\n|---|---|---|\n| - | - | x |\n",
        values(first),
        values(second)
    )
}

#[test]
fn a_table_over_the_limit_is_too_large_to_prove() {
    let got = messages(&sized(17, 241));
    assert_eq!(got.len(), 1, "{got:?}");
    assert!(got[0].contains("too large to prove"), "{got:?}");
    assert!(got[0].contains("more than 4096 combinations"), "{got:?}");
}

#[test]
fn a_row_covering_no_valid_combination_is_not_called_shadowed() {
    let rows = ["| - | - | 0 |", "| silver | yes | 5 |"];
    let got = messages(&table(FIRST, &rows));
    assert_eq!(got.len(), 1, "{got:?}");
    assert!(got[0].contains("'silver'"), "{got:?}");
}

#[test]
fn an_escaped_pipe_stays_inside_its_cell_and_an_escaped_backslash_does_not() {
    for first in ["| - | yes | x \\| y |", "| - | yes | x \\\\|"] {
        let rows = [first, "| - | no | 0 |"];
        assert_eq!(messages(&table(DEFAULT, &rows)), Vec::<String>::new());
    }
}

#[test]
fn a_condition_name_may_hold_parentheses() {
    let body = "| amount (USD) (low, high) | fee |\n|---|---|\n| low | 1 |\n| high | 2 |";
    let got = messages(&format!("{DEFAULT}\n{body}"));
    assert_eq!(got, Vec::<String>::new());
}

#[test]
fn a_table_at_the_limit_is_enumerated() {
    assert_eq!(messages(&sized(64, 64)), Vec::<String>::new());
    let missing = sized(64, 64).replace("| - | - | x |", "| v0 | - | x |");
    assert!(messages(&missing)[0].contains("no row covers"));
}

#[test]
fn an_unmarked_table_is_never_read() {
    let body = format!("{HEAD}| gold | yes | 5 |\n");
    assert_eq!(messages(&body), Vec::<String>::new());
}

#[test]
fn a_marked_table_in_a_record_that_is_not_a_domain_record_is_not_read() {
    let contents = record(&table(DEFAULT, &FULL[..1]));
    assert_eq!(run_at("adr/0001-x.md", &contents), vec![]);
    let other_type = contents.replace("type: Context", "type: ADR");
    assert_eq!(run_at(PATH, &other_type), vec![]);
}

#[test]
fn only_the_bundles_own_context_folder_holds_domain_records() {
    let contents = record(&table(DEFAULT, &FULL[..1]));
    let bundle = "/work/context/bundle";
    assert_eq!(
        run_in(bundle, "/work/context/bundle/notes/x.md", &contents),
        []
    );
    let inside = run_in(bundle, "/work/context/bundle/context/fees.md", &contents);
    assert_eq!(inside.len(), 1, "{inside:?}");
}

#[test]
fn a_marked_table_under_a_second_level_heading_is_not_read() {
    let contents = format!(
        "---\ntype: Context\n---\n\n## Rules\n\n{}",
        table(DEFAULT, &FULL[..1])
    );
    assert_eq!(run_at(PATH, &contents), vec![]);
}

#[test]
fn a_marked_table_inside_a_backtick_or_tilde_fence_is_not_read() {
    for fence in ["```", "~~~"] {
        let body = format!("{fence}md\n{}{fence}", table(DEFAULT, &FULL[..1]));
        assert_eq!(messages(&body), Vec::<String>::new());
    }
}

fn one(body: &str, needle: &str) {
    let got = messages(body);
    assert_eq!(got.len(), 1, "{body:?} -> {got:?}");
    assert!(got[0].contains(needle), "{got:?} lacks {needle:?}");
}

#[test]
fn a_malformed_marker_or_table_is_one_advisory_naming_the_problem() {
    let plain = |rest: &str| format!("{DEFAULT}\n{rest}");
    let ragged = [FULL[0], "| gold | no |", FULL[2], FULL[3]];
    let cases = [
        (plain("| tier | fee |\n|---|---|\n| gold | 5 |\n"), "tier"),
        (
            plain("| tier () | fee |\n|---|---|\n| gold | 5 |\n"),
            "tier",
        ),
        (plain("| fee |\n|---|\n| 5 |\n"), "condition"),
        (plain("| tier (a, b) | fee |\n| a | 5 |\n"), "separator"),
        (plain("some prose\n"), "not followed by a table"),
        (table(DEFAULT, &ragged), "row 2"),
        (table("<!-- decision-table: last -->", &FULL), "last"),
        (table("<!-- decision-table: -->", &FULL), "policy"),
        (table("<!-- decision-table", &FULL), "marker"),
    ];
    for (body, needle) in cases {
        one(&body, needle);
    }
}

#[test]
fn a_marker_at_the_end_of_the_file_is_one_advisory() {
    let contents = "---\ntype: Context\n---\n\n### Rule\n\n<!-- decision-table -->";
    let got = run_at(PATH, contents);
    assert_eq!(got.len(), 1, "{got:?}");
    assert!(got[0].1.contains("not followed by a table"), "{got:?}");
}

#[test]
fn crlf_input_reads_the_same_as_lf() {
    let lf = record(&table(DEFAULT, &FULL[..3]));
    let crlf = lf.replace('\n', "\r\n");
    assert_eq!(run_at(PATH, &crlf), run_at(PATH, &lf));
    assert_eq!(run_at(PATH, &crlf).len(), 1);
}

#[test]
fn two_marked_tables_under_one_rule_are_each_judged() {
    let body = format!(
        "{}\n{}",
        table(DEFAULT, &FULL[..3]),
        table(FIRST, &FULL[..3])
    );
    assert_eq!(messages(&body).len(), 2);
}
