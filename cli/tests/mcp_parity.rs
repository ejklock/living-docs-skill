//! Every MCP tool operation against its `living-docs <verb> --json` twin:
//! the same files left behind and the same JSON returned. Rows read as
//! documented on `common::parity::parse`.

mod common;

use common::parity::{assert_parity, parse};

const WRITE: &[&str] = &[
    r#"write new adr | write new adr Queue-choice --owner alice --description Pick-a-queue --body {body} | write | {"operation":"new","type":"adr","title":"Queue-choice","owner":"alice","description":"Pick-a-queue","body":"{body}"}"#,
    r#"write new view with a kind | write new view Gears --kind component --description gears | write | {"operation":"new","type":"view","title":"Gears","kind":"component","description":"gears"}"#,
    r#"write set field | write set adr/0001 status Accepted | write | {"operation":"set","reference":"adr/0001","key":"status","value":"Accepted"}"#,
    r#"write set section | write set adr/0001 section Context --from {body} | write | {"operation":"set","reference":"adr/0001","heading":"Context","text":"{body}"}"#,
    r#"write supersede | write supersede adr/0001 adr/0002 | write | {"operation":"supersede","old":"adr/0001","new":"adr/0002"}"#,
    r#"write set unknown reference | write set adr/9999 status Accepted | write | {"operation":"set","reference":"adr/9999","key":"status","value":"Accepted"} | fails"#,
];

const READ_RECORDS: &[&str] = &[
    r#"read records | read records | read | {"operation":"records"}"#,
    r#"read records topic | read records --topic cache | read | {"operation":"records","topic":"cache"}"#,
    r#"read records full | read records --full | read | {"operation":"records","full":true}"#,
    r#"read records contract | read records --contract | read | {"operation":"records","contract":true}"#,
    r#"read records links_to | read records --links-to adr/0001 | read | {"operation":"records","links_to":"adr/0001"}"#,
    r#"read records impact | read records --impact adr/0001 | read | {"operation":"records","impact":"adr/0001"}"#,
];

const READ_GUIDE: &[&str] = &[
    r#"read guide list | read guide --list | read | {"operation":"guide","list":true}"#,
    r#"read guide default skill | read guide | read | {"operation":"guide"}"#,
    r#"read guide topic | read guide adr | read | {"operation":"guide","topic":"adr"}"#,
    r#"read guide named skill | read guide living-docs-drift | read | {"operation":"guide","skill":"living-docs-drift"}"#,
    r#"read guide unknown topic | read guide no-such-topic | read | {"operation":"guide","topic":"no-such-topic"} | fails"#,
];

const MANAGE: &[&str] = &[
    r#"manage index one type | manage index adr | manage | {"operation":"index","type":"adr"} | stale,shows=adr/index.md"#,
    r#"manage index every type | manage index | manage | {"operation":"index"} | stale,shows=adr/index.md,shows=architecture/index.md"#,
    r#"manage fmt check only | manage fmt --check | manage | {"operation":"fmt","check_only":true} | noncanonical,report"#,
    r#"manage fmt writing | manage fmt | manage | {"operation":"fmt"} | noncanonical"#,
];

const CHECK: &[&str] = &[
    r#"check clean bundle | check | check | {} | canonical,shows="ok":true"#,
    r#"check unfilled slot | check | check | {} | report,shows="ok":false,shows=unfilled"#,
];

#[cfg(unix)]
const WRITE_WITH_UNWRITABLE_INDEX: &[&str] = &[
    r#"write new adr keeps the record when the index refresh fails | write new adr Two --owner o --description d | write | {"operation":"new","type":"adr","title":"Two","owner":"o","description":"d"} | readonly,partial"#,
];

#[cfg(unix)]
#[test]
fn write_with_an_unwritable_index_matches_the_cli() {
    assert_parity(&parse(WRITE_WITH_UNWRITABLE_INDEX));
}

#[test]
fn write_operations_match_the_cli() {
    assert_parity(&parse(WRITE));
}

#[test]
fn read_records_match_the_cli() {
    assert_parity(&parse(READ_RECORDS));
}

#[test]
fn read_guide_matches_the_cli() {
    assert_parity(&parse(READ_GUIDE));
}

#[test]
fn manage_operations_match_the_cli() {
    assert_parity(&parse(MANAGE));
}

#[test]
fn check_operations_match_the_cli() {
    assert_parity(&parse(CHECK));
}
