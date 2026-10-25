use super::*;
use crate::test_support::WritableMapStore as MapStore;

fn adr(status: &str) -> String {
    format!("---\ntype: ADR\ntitle: A Decision\ndescription: A decision.\nstatus: {status}\n---\n\n# 0001. A Decision\n\nBody.\n")
}

const ADR_PATH: &str = "/bundle/adr/0001-a-decision.md";

fn bundle() -> &'static Path {
    Path::new("/bundle")
}

fn store(status: &str) -> MapStore {
    MapStore::seeded(&[(ADR_PATH, &adr(status))])
}

#[test]
fn retitle_rewrites_frontmatter_heading_and_filename() {
    let store = store("Proposed");
    let retitled = apply(
        &store,
        bundle(),
        Path::new(ADR_PATH),
        "Cache is write-through",
    )
    .expect("proposed");

    let contents = store.contents("/bundle/adr/0001-cache-is-write-through.md");
    assert_eq!(
        retitled.path,
        PathBuf::from("/bundle/adr/0001-cache-is-write-through.md")
    );
    assert!(
        contents.contains("title: Cache is write-through\n"),
        "{contents}"
    );
    assert!(
        contents.contains("# 0001. Cache is write-through\n"),
        "{contents}"
    );
    assert!(
        store.read(Path::new(ADR_PATH)).is_err(),
        "old file still present"
    );
}

#[test]
fn retitle_is_allowed_on_an_accepted_record() {
    let store = store("Accepted");
    apply(&store, bundle(), Path::new(ADR_PATH), "Still revisable")
        .expect("accepted is not closed");
    assert!(!store
        .contents("/bundle/adr/0001-still-revisable.md")
        .is_empty());
}

#[test]
fn retitle_is_refused_on_a_terminal_record_and_writes_nothing() {
    let store = store("Deprecated");
    let err =
        apply(&store, bundle(), Path::new(ADR_PATH), "Too late").expect_err("terminal status");

    assert!(err.contains("living-docs supersede"), "got: {err}");
    assert_eq!(store.contents(ADR_PATH), adr("Deprecated"));
}

#[test]
fn retitle_is_refused_on_a_superseded_record() {
    let store = store("Superseded");
    let err = apply(&store, bundle(), Path::new(ADR_PATH), "Too late").expect_err("superseded");
    assert!(err.contains("living-docs supersede"), "got: {err}");
}

#[test]
fn retitle_is_refused_when_the_new_slug_is_taken_and_writes_nothing() {
    let taken = "---\ntype: Architecture View\ntitle: System Context\nkind: context\n---\n\n# System Context\n";
    let moving = "---\ntype: Architecture View\ntitle: Container View\nkind: container\n---\n\n# Container View\n";
    let store = MapStore::seeded(&[
        ("/bundle/architecture/system-context.md", taken),
        ("/bundle/architecture/container-view.md", moving),
    ]);

    let err = apply(
        &store,
        bundle(),
        Path::new("/bundle/architecture/container-view.md"),
        "System Context",
    )
    .expect_err("a named record's slug is its whole filename");

    assert!(err.contains("already holds that slug"), "got: {err}");
    assert_eq!(
        store.contents("/bundle/architecture/container-view.md"),
        moving
    );
}

#[test]
fn retitle_refuses_a_title_with_no_slug() {
    let store = store("Proposed");
    let err = apply(&store, bundle(), Path::new(ADR_PATH), "!!!").expect_err("empty slug");
    assert!(err.contains("has no slug"), "got: {err}");
    assert_eq!(store.contents(ADR_PATH), adr("Proposed"));
}

#[test]
fn retitle_repoints_every_in_bundle_reference_to_the_old_filename() {
    let citing = "---\ntype: Issue\ntitle: Cites\nstatus: open\n---\n\n# 0009. Cites\n\nSee [ADR 0001](/adr/0001-a-decision.md) and `0001-a-decision.md`.\n";
    let store = MapStore::seeded(&[
        (ADR_PATH, &adr("Accepted")),
        ("/bundle/issues/0009-cites.md", citing),
    ]);

    let retitled =
        apply(&store, bundle(), Path::new(ADR_PATH), "Write-through cache").expect("live");

    let cites = store.contents("/bundle/issues/0009-cites.md");
    assert!(
        cites.contains("/adr/0001-write-through-cache.md"),
        "{cites}"
    );
    assert!(!cites.contains("0001-a-decision.md"), "{cites}");
    assert_eq!(
        retitled.rewritten,
        vec![PathBuf::from("/bundle/issues/0009-cites.md")]
    );
}

#[test]
fn retitle_of_a_named_record_renames_it_to_the_bare_slug() {
    let view = "---\ntype: Architecture View\ntitle: Context View\nkind: context\n---\n\n# Context View\n\nBody.\n";
    let store = MapStore::seeded(&[("/bundle/architecture/context-view.md", view)]);

    apply(
        &store,
        bundle(),
        Path::new("/bundle/architecture/context-view.md"),
        "System Context",
    )
    .expect("a view has no status vocabulary");

    let contents = store.contents("/bundle/architecture/system-context.md");
    assert!(contents.contains("# System Context\n"), "{contents}");
}

#[test]
fn retitle_of_a_singleton_rewrites_the_record_without_renaming_it() {
    let constitution = "---\ntype: Constitution\ntitle: Old Name\n---\n\n# Old Name\n\nBody.\n";
    let store = MapStore::seeded(&[("/bundle/constitution.md", constitution)]);

    let retitled = apply(
        &store,
        bundle(),
        Path::new("/bundle/constitution.md"),
        "Product Constitution",
    )
    .expect("a singleton keeps its filename");

    let contents = store.contents("/bundle/constitution.md");
    assert_eq!(retitled.path, PathBuf::from("/bundle/constitution.md"));
    assert!(
        contents.contains("title: Product Constitution\n"),
        "{contents}"
    );
    assert!(contents.contains("# Product Constitution\n"), "{contents}");
}

const FRONT: &str =
    "---\ntype: ADR\ntitle: Old\ndescription: A decision.\nstatus: Proposed\n---\n\n";
const OLD_PATH: &str = "/bundle/adr/0003-old.md";
const NEW_PATH: &str = "/bundle/adr/0003-t.md";

fn retitle_to_t(body: &str) -> String {
    let store = MapStore::seeded(&[(OLD_PATH, &format!("{FRONT}{body}"))]);
    apply(&store, bundle(), Path::new(OLD_PATH), "T").expect("proposed");
    let contents = store.contents(NEW_PATH);
    assert!(contents.contains("title: T\n"), "{contents}");
    contents
        .split_once("---\n\n")
        .map(|(_, rest)| rest.to_owned())
        .expect("frontmatter")
}

#[test]
fn retitle_leaves_the_body_untouched_when_there_is_no_title_heading() {
    let body = "## Context\n\nWhy.\n";
    assert_eq!(retitle_to_t(body), body);
}

#[test]
fn retitle_skips_a_fenced_heading_that_carries_the_number() {
    assert_eq!(
        retitle_to_t("```\n# 0003. Fake\n```\n\n# 0003. Old\n"),
        "```\n# 0003. Fake\n```\n\n# 0003. T\n"
    );
}

#[test]
fn retitle_renames_only_the_title_heading_above_a_section() {
    assert_eq!(
        retitle_to_t("# 0003. Old\n\n## Context\n"),
        "# 0003. T\n\n## Context\n"
    );
}

#[test]
fn retitle_renames_an_unnumbered_level_one_heading_of_a_numbered_record() {
    assert_eq!(retitle_to_t("## Context\n\n# Old\n"), "## Context\n\n# T\n");
}

#[test]
fn retitle_of_a_singleton_renames_its_level_one_heading_only() {
    let constitution = "---\ntype: Constitution\ntitle: Old\n---\n\n## Principles\n\n# Old\n";
    let store = MapStore::seeded(&[("/bundle/constitution.md", constitution)]);

    apply(&store, bundle(), Path::new("/bundle/constitution.md"), "T").expect("singleton");

    let contents = store.contents("/bundle/constitution.md");
    assert!(contents.ends_with("## Principles\n\n# T\n"), "{contents}");
}

#[test]
fn retitle_keeps_crlf_on_the_rewritten_heading() {
    let record = format!("{FRONT}# 0003. Old\r\nBody.\r\n");
    let store = MapStore::seeded(&[(OLD_PATH, &record)]);

    rewrite_heading(&store, Path::new(OLD_PATH), "T").expect("heading");

    assert_eq!(
        store.contents(OLD_PATH),
        format!("{FRONT}# 0003. T\r\nBody.\r\n")
    );
}
