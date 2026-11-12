//! Turning a `links_to` or `impact` reference into the dependents query that
//! narrows the view.

use super::*;

fn bundle() -> MapStore {
    let first = adr("0001", "Accepted", None, "root");
    let second = adr("0002", "Accepted", None, "other");
    store_of(&[(&first.0, &first.1), (&second.0, &second.1)])
}

fn record_path(number: &str) -> PathBuf {
    Path::new("docs").join("adr").join(format!("{number}-x.md"))
}

fn query(links_to: Option<&str>, impact: Option<&str>) -> Result<Option<Dependents>, String> {
    resolve_dependents(&bundle(), Path::new("docs"), links_to, impact)
}

#[test]
fn neither_reference_gives_no_query() {
    assert!(matches!(query(None, None), Ok(None)));
}

#[test]
fn a_links_to_reference_resolves_to_the_direct_query_on_that_record() {
    let resolved = query(Some("adr/0001"), None);
    let Ok(Some(Dependents::LinksTo(path))) = resolved else {
        panic!("expected a links-to query");
    };
    assert_eq!(path, record_path("0001"));
}

#[test]
fn an_impact_reference_resolves_to_the_chained_query_on_that_record() {
    let resolved = query(None, Some("adr/0002"));
    let Ok(Some(Dependents::Impact(path))) = resolved else {
        panic!("expected an impact query");
    };
    assert_eq!(path, record_path("0002"));
}

#[test]
fn both_references_at_once_are_refused() {
    let Err(message) = query(Some("adr/0001"), Some("adr/0002")) else {
        panic!("expected a refusal");
    };
    assert!(message.contains("exclude one another"), "got: {message}");
}

#[test]
fn a_reference_naming_no_record_is_refused() {
    assert!(query(Some("adr/0099"), None).is_err());
    assert!(query(None, Some("adr/0099")).is_err());
}
