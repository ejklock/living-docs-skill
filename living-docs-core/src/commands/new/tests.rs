use super::*;

#[test]
fn civil_date_from_unix_days_matches_known_calendar_dates() {
    assert_eq!(civil_date_from_unix_days(0), (1970, 1, 1));
    assert_eq!(civil_date_from_unix_days(1), (1970, 1, 2));
    assert_eq!(civil_date_from_unix_days(31), (1970, 2, 1));
}

#[test]
fn now_iso8601_has_the_expected_shape() {
    let timestamp = now_iso8601();
    assert_eq!(timestamp.len(), 20);
    assert_eq!(&timestamp[4..5], "-");
    assert_eq!(&timestamp[7..8], "-");
    assert_eq!(&timestamp[10..11], "T");
    assert_eq!(&timestamp[13..14], ":");
    assert_eq!(&timestamp[16..17], ":");
    assert_eq!(&timestamp[19..20], "Z");
}

#[test]
fn unsupported_type_message_names_the_offending_type() {
    assert!(unsupported_type_message("constitution").contains("constitution"));
}

/// ADR 0026 fitness function C: the message is generated from
/// `DOC_TYPES` rather than a hand-maintained list, so it can never omit
/// a token the registry actually supports.
#[test]
fn unsupported_type_message_lists_every_registry_token() {
    let message = unsupported_type_message("bogus");
    for spec in doc_type::DOC_TYPES {
        assert!(
            message.contains(spec.token),
            "{message:?} is missing registry token {:?}",
            spec.token
        );
    }
}
