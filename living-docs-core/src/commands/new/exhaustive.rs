//! Every `--dir` value up to five characters over an alphabet of slug
//! characters, separators, dots, a space, a backslash and a non-ASCII letter
//! is either refused or a relative folder of normal components whose every
//! segment is its own slug; the unsafe shapes are always refused.

use super::validated_subdir;
use crate::exhaustive::for_each_sequence;
use crate::paths::slugify;
use std::path::Component;

const ALPHABET: [char; 7] = ['a', '-', '/', '.', ' ', 'é', '\\'];

#[test]
fn an_accepted_dir_is_a_relative_folder_of_slug_segments_for_every_short_value() {
    let cases = for_each_sequence(&ALPHABET, 5, &mut |chars| {
        let dir: String = chars.iter().collect();
        let unsafe_shape = dir.starts_with('/')
            || dir.contains('\\')
            || dir
                .split('/')
                .any(|segment| matches!(segment, "" | "." | ".."));
        let outcome = validated_subdir(&dir);
        if let Ok(path) = &outcome {
            assert!(!unsafe_shape, "{dir:?} was accepted");
            assert!(path.is_relative(), "{dir:?} gave {path:?}");
            assert!(
                path.components().all(|c| matches!(c, Component::Normal(_))),
                "{dir:?} gave {path:?}"
            );
            assert!(
                dir.split('/').all(|segment| slugify(segment) == segment),
                "{dir:?} was accepted with a segment that is not a slug"
            );
        }
        if unsafe_shape {
            assert!(outcome.is_err(), "{dir:?} was accepted");
        }
    });
    assert_eq!(cases, 1 + 7 + 49 + 343 + 2401 + 16_807);
}
