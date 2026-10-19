use super::*;

#[test]
fn stdin_text_is_read_and_crlf_normalized() {
    let mut input = "a\r\nb\r\n".as_bytes();
    assert_eq!(read_source_with("-", false, &mut input).unwrap(), "a\nb\n");
}

#[test]
fn stdin_on_a_terminal_is_refused_with_a_hint() {
    let mut input = "ignored".as_bytes();
    let error = read_source_with("-", true, &mut input).unwrap_err();
    assert!(
        error.contains("terminal") && error.contains("pipe"),
        "{error}"
    );
}

#[test]
fn a_terminal_does_not_affect_a_file_path() {
    let error = read_source_with("/nonexistent/prose.md", true, &mut io::empty()).unwrap_err();
    assert!(
        error.contains("cannot read /nonexistent/prose.md"),
        "{error}"
    );
}

#[test]
fn body_ends_with_exactly_one_newline() {
    assert_eq!(body_from("x\n\n\n").unwrap(), "x\n");
    assert_eq!(body_from("x").unwrap(), "x\n");
}

#[test]
fn whitespace_only_body_is_refused() {
    assert!(body_from(" \n\t\n").is_err());
    assert!(body_from("").is_err());
}

#[test]
fn body_opening_with_a_fence_is_refused_but_a_later_rule_is_kept() {
    assert!(body_from("---\ntype: x\n---\n").is_err());
    assert_eq!(body_from("x\n---\ny\n").unwrap(), "x\n---\ny\n");
}
