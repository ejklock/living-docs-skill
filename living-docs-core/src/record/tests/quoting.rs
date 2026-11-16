use super::super::*;
use canonical::format_strict_flow_list;

const TYPE_AMBIGUOUS: [&str; 10] = [
    "+4", "-3", "1.5", "1e3", "0x10", ".inf", "true", "false", "null", "~",
];

const TEXT_CHANGING: [&str; 6] = ["+4", "-3", "1e3", "0x10", "null", "~"];

const PLAIN_SINCE_BEFORE: [&str; 5] = ["1.5", "2.5", "true", "false", ".inf"];

const LEFT_PLAIN: [&str; 7] = [
    "12",
    "0",
    "abc-9",
    "plain words",
    "0005",
    "0089",
    "ref ab-77",
];

fn read_string(source: &str) -> Option<String> {
    match serde_yaml::from_str::<serde_yaml::Value>(source).ok()? {
        serde_yaml::Value::Mapping(map) => match map.get("k")? {
            serde_yaml::Value::Sequence(items) => items.first()?.as_str().map(str::to_owned),
            other => other.as_str().map(str::to_owned),
        },
        _ => None,
    }
}

#[test]
fn quotes_a_scalar_that_would_read_as_another_type() {
    for value in TYPE_AMBIGUOUS {
        let list = format_strict_flow_list(&[value]);
        assert!(list.starts_with("[\""), "{value}: {list}");
        assert_eq!(read_string(&format!("k: {list}")).as_deref(), Some(value));
    }
    for value in TEXT_CHANGING {
        let scalar = format_scalar(value);
        assert!(scalar.starts_with('"'), "{value}: {scalar}");
        assert_eq!(read_string(&format!("k: {scalar}")).as_deref(), Some(value));
        let list = format_flow_list(&[value]);
        assert!(list.starts_with("[\""), "{value}: {list}");
        assert_eq!(read_string(&format!("k: {list}")).as_deref(), Some(value));
    }
}

#[test]
fn a_label_that_would_read_as_another_type_is_written_quoted() {
    for value in PLAIN_SINCE_BEFORE {
        assert_eq!(format_strict_flow_list(&[value]), format!("[\"{value}\"]"));
    }
}

#[test]
fn a_value_the_previous_writer_accepted_keeps_its_bytes() {
    for value in PLAIN_SINCE_BEFORE {
        assert_eq!(format_scalar(value), value, "{value}");
    }
    let source = "---\ntype: ADR\ntitle: 1.5\ndescription: true\nowner: .inf\nstatus: false\nlabels: [true, 1.5]\n---\n\nBody.\n";
    let record = extract_record(Path::new("adr/0001-t.md"), source);
    let frontmatter = &source[..source.find("\n---\n\n").unwrap_or(0) + 5];
    assert!(to_canonical_markdown(&record).starts_with(frontmatter));
}

#[test]
fn leaves_integers_zero_padded_digits_and_words_unquoted() {
    for value in LEFT_PLAIN {
        assert_eq!(format_scalar(value), value);
        assert_eq!(format_flow_list(&[value]), format!("[{value}]"));
        assert_eq!(format_strict_flow_list(&[value]), format!("[{value}]"));
    }
}

#[test]
fn quotes_digits_beyond_what_a_number_can_hold_and_reads_them_back() {
    for value in ["12345678901234567890123", "18446744073709551616"] {
        for list in [format_flow_list(&[value]), format_strict_flow_list(&[value])] {
            assert_eq!(list, format!("[\"{value}\"]"));
            assert_eq!(read_string(&format!("k: {list}")).as_deref(), Some(value));
        }
    }
    assert_eq!(
        format_strict_flow_list(&["18446744073709551615"]),
        "[18446744073709551615]"
    );
}
