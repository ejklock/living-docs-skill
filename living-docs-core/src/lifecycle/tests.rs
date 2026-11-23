use super::*;
use crate::doc_type::spec_for_frontmatter;

fn adr() -> &'static DocTypeSpec {
    spec_for_frontmatter("ADR").expect("ADR is registered")
}

fn issue() -> &'static DocTypeSpec {
    spec_for_frontmatter("Issue").expect("Issue is registered")
}

fn facts(number: i32, status: &str, successor: Option<&str>) -> Facts {
    Facts {
        number,
        status: Some(status.to_owned()),
        successor: successor.map(str::to_owned),
    }
}

fn links(pairs: &[(i32, i32)]) -> BTreeMap<i32, i32> {
    pairs.iter().copied().collect()
}

#[test]
fn a_status_may_be_set_on_a_record_with_no_successor() {
    for status in ["Proposed", "Accepted", "Deprecated"] {
        assert_eq!(refuse_set_status(adr(), &facts(1, status, None)), Ok(()));
    }
    assert_eq!(
        refuse_set_status(adr(), &facts(1, "Accepted", Some(""))),
        Ok(())
    );
}

#[test]
fn a_status_may_not_be_set_on_a_record_that_names_a_successor() {
    let refusal = refuse_set_status(adr(), &facts(1, "Accepted", Some("0002")))
        .expect_err("a named successor refuses a status");
    let text = refusal.to_string();
    assert!(
        text.contains("ADR") && text.contains("0001") && text.contains("superseded_by is 0002"),
        "{text}"
    );
    assert!(text.contains("by hand"), "{text}");
}

#[test]
fn a_status_may_not_be_set_on_a_record_whose_status_is_superseded() {
    let refusal = refuse_set_status(issue(), &facts(7, "Superseded", None))
        .expect_err("a superseded status refuses a status");
    let Refusal::StatusOnSuperseded { label, successor } = refusal else {
        panic!("expected a refusal of the status")
    };
    assert!(
        label.starts_with("Issue") && label.ends_with("0007"),
        "{label}"
    );
    assert_eq!(successor, None);
}

#[test]
fn a_record_with_no_successor_is_superseded_by_a_proposed_record() {
    let outcome = refuse_supersede(
        adr(),
        &facts(1, "Accepted", None),
        &facts(2, "Proposed", None),
        &links(&[]),
    );
    assert_eq!(outcome, Ok(()));
}

#[test]
fn a_record_with_a_successor_is_refused_a_second_one() {
    let refusal = refuse_supersede(
        adr(),
        &facts(1, "Superseded", Some("0002")),
        &facts(3, "Proposed", None),
        &links(&[(1, 2)]),
    )
    .expect_err("a second successor is refused");
    let text = refusal.to_string();
    assert!(text.contains("0001 already superseded by"), "{text}");
    assert!(text.contains("0002; a record has one successor"), "{text}");
}

#[test]
fn a_successor_whose_chain_reaches_the_record_closes_a_cycle() {
    let refusal = refuse_supersede(
        adr(),
        &facts(3, "Accepted", None),
        &facts(1, "Superseded", Some("0002")),
        &links(&[(1, 2), (2, 3)]),
    )
    .expect_err("a cycle is refused");
    let Refusal::Cycle { chain } = refusal else {
        panic!("expected a cycle")
    };
    let numbers: Vec<&str> = chain
        .iter()
        .map(|l| l.rsplit(' ').next().unwrap())
        .collect();
    assert_eq!(numbers, ["0003", "0001", "0002", "0003"]);
}

#[test]
fn a_chain_that_loops_elsewhere_does_not_hang_the_walk() {
    let outcome = refuse_supersede(
        adr(),
        &facts(1, "Accepted", None),
        &facts(2, "Proposed", None),
        &links(&[(2, 3), (3, 2)]),
    );
    assert_eq!(outcome, Ok(()));
}

#[test]
fn a_retired_successor_is_refused_by_the_type_s_own_terminal_statuses() {
    let retired = [
        (adr(), "Deprecated"),
        (adr(), "Superseded"),
        (issue(), "closed"),
    ];
    for (spec, status) in retired {
        let refusal = refuse_supersede(
            spec,
            &facts(1, "x", None),
            &facts(2, status, None),
            &links(&[]),
        )
        .expect_err("a retired successor is refused");
        let text = refusal.to_string();
        assert!(
            text.contains(status) && text.contains("0001") && text.contains("0002"),
            "{text}"
        );
    }
}

#[test]
fn an_open_issue_is_a_legal_successor() {
    let outcome = refuse_supersede(
        issue(),
        &facts(1, "open", None),
        &facts(2, "open", None),
        &links(&[]),
    );
    assert_eq!(outcome, Ok(()));
}

#[test]
fn a_cycle_is_reported_before_a_retired_successor() {
    let refusal = refuse_supersede(
        adr(),
        &facts(2, "Accepted", None),
        &facts(1, "Superseded", Some("0002")),
        &links(&[(1, 2)]),
    )
    .expect_err("refused");
    assert!(matches!(refusal, Refusal::Cycle { .. }), "{refusal:?}");
}

#[test]
fn repeating_the_same_supersede_is_legal_even_when_the_successor_was_since_retired() {
    for status in ["Proposed", "Deprecated"] {
        let outcome = refuse_supersede(
            adr(),
            &facts(1, "Superseded", Some("0002")),
            &facts(2, status, None),
            &links(&[(1, 2)]),
        );
        assert_eq!(outcome, Ok(()), "{status}");
    }
}

#[test]
fn a_different_successor_is_still_a_re_supersede() {
    let outcome = refuse_supersede(
        adr(),
        &facts(1, "Superseded", Some("0002")),
        &facts(3, "Proposed", None),
        &links(&[(1, 2)]),
    );
    assert!(matches!(outcome, Err(Refusal::AlreadySuperseded { .. })));
}
