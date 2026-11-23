use super::export_diagram;

const FLAT: &str = "stateDiagram-v2\n[*] --> pending\npending --> paid: pay\npaid --> shipped\nshipped --> [*]\npending --> cancelled: customer cancels\ncancelled --> [*]\n";

const FLAT_MODULE: &str = r#"---- MODULE order_lifecycle ----
VARIABLE state

States == {"pending", "paid", "shipped", "cancelled"}

Terminal == {"shipped", "cancelled"}

Init == state = "pending"

\* pending --> paid: pay
T1 == state = "pending" /\ state' = "paid"

\* paid --> shipped
T2 == state = "paid" /\ state' = "shipped"

\* pending --> cancelled: customer cancels
T3 == state = "pending" /\ state' = "cancelled"

Next == T1 \/ T2 \/ T3 \/ (state \in Terminal /\ state' = state)

TypeOK == state \in States

Spec == Init /\ [][Next]_state /\ WF_state(Next)
====
"#;

fn refusal(body: &str) -> String {
    export_diagram("v", body).expect_err("the diagram is refused")
}

#[test]
fn a_flat_lifecycle_renders_the_exact_module() {
    assert_eq!(
        export_diagram("order-lifecycle", FLAT).unwrap(),
        FLAT_MODULE
    );
}

#[test]
fn two_runs_are_byte_identical() {
    assert_eq!(export_diagram("a-b", FLAT), export_diagram("a-b", FLAT));
}

#[test]
fn a_slug_with_hyphens_names_the_module_with_underscores() {
    let module = export_diagram("order-lifecycle", FLAT).unwrap();
    assert!(module.starts_with("---- MODULE order_lifecycle ----\n"));
}

#[test]
fn a_state_id_that_is_not_a_tla_identifier_is_a_string_constant() {
    let body = "stateDiagram-v2\n[*] --> 2fa.pending\n2fa.pending --> done\ndone --> [*]\n";
    let module = export_diagram("v", body).unwrap();
    assert!(
        module.contains("States == {\"2fa.pending\", \"done\"}"),
        "{module}"
    );
    assert!(
        module.contains("T1 == state = \"2fa.pending\" /\\ state' = \"done\""),
        "{module}"
    );
}

#[test]
fn a_state_with_a_description_keeps_its_id_as_the_constant() {
    let body = "stateDiagram-v2\nstate \"Waiting for payment\" as waiting\n[*] --> waiting\nwaiting --> [*]\n";
    let module = export_diagram("v", body).unwrap();
    assert!(module.contains("Init == state = \"waiting\""), "{module}");
}

#[test]
fn a_composite_state_is_refused_by_name() {
    let body = "stateDiagram-v2\n[*] --> Outer\nstate Outer {\n[*] --> Inner\nInner --> [*]\n}\nOuter --> [*]\n";
    let message = refusal(body);
    assert!(message.contains("composite state 'Outer'"), "{message}");
}

#[test]
fn concurrent_regions_are_refused() {
    let message = refusal("stateDiagram-v2\n[*] --> A\nstate A {\nB --> C\n--\nD --> E\n}\n");
    assert!(message.contains("concurrent regions"), "{message}");
    assert!(message.contains("composite state 'A'"), "{message}");
}

#[test]
fn a_diagram_without_an_initial_transition_is_refused() {
    let message = refusal("stateDiagram-v2\nA --> B\nB --> [*]\n");
    assert!(message.contains("no initial transition"), "{message}");
}

#[test]
fn several_initial_transitions_are_refused() {
    let message = refusal("stateDiagram-v2\n[*] --> A\n[*] --> B\nA --> [*]\nB --> [*]\n");
    assert!(message.contains("2 initial transitions"), "{message}");
}

#[test]
fn a_diagram_that_does_not_parse_names_the_parse_failure() {
    let message = refusal("stateDiagram-v2\n[*] --> order-paid\n");
    assert!(message.contains("does not parse"), "{message}");
}

#[test]
fn a_diagram_without_a_terminal_state_has_an_empty_terminal_set() {
    let module = export_diagram("v", "stateDiagram-v2\n[*] --> A\nA --> B\nB --> A\n").unwrap();
    assert!(module.contains("Terminal == {}\n"), "{module}");
}

#[test]
fn a_backslash_and_a_quote_in_a_state_id_are_escaped() {
    assert_eq!(super::constant("a\\b\"c"), "\"a\\\\b\\\"c\"");
}

#[test]
fn text_that_is_not_a_state_diagram_is_refused() {
    let message = refusal("graph TD\na-->b\n");
    assert!(message.contains("not a state diagram"), "{message}");
}
