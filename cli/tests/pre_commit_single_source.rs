//! This repository's own pre-commit hook is the distributed hook, not a fork of it.

use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the cli crate sits under the workspace root")
        .to_path_buf()
}

#[test]
fn the_repository_hook_is_byte_identical_to_the_distributed_hook() {
    let distributed = root().join("skills/living-docs/hooks/pre-commit");
    let repository = root().join(".githooks/pre-commit");

    let expected = fs::read(&distributed).expect("the distributed hook is readable");
    let actual = fs::read(&repository).expect("the repository hook is readable");

    assert!(
        actual == expected,
        "{} differs from {}",
        repository.display(),
        distributed.display()
    );
}
