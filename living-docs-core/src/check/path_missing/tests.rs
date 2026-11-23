use super::*;

#[test]
fn a_position_suffix_and_a_leading_dot_slash_are_dropped() {
    assert_eq!(path_of("src/a.rs:42").as_deref(), Some("src/a.rs"));
    assert_eq!(path_of("src/a.rs:42:7").as_deref(), Some("src/a.rs"));
    assert_eq!(path_of("./src/a.rs").as_deref(), Some("src/a.rs"));
}

#[test]
fn a_token_outside_the_charset_or_without_a_slash_is_no_path() {
    for token in [
        "a.rs",
        "a b/c",
        "a*b/c",
        "https://x.io/a",
        "src/a.rs:x",
        "a/b:",
    ] {
        assert_eq!(path_of(token), None, "{token}");
    }
}

#[test]
fn an_absolute_token_or_one_with_a_parent_segment_is_no_path() {
    for token in [
        "/etc/passwd",
        "/login",
        "//x",
        "/",
        "src/../a.rs",
        "../a/b.rs",
    ] {
        assert_eq!(path_of(token), None, "{token}");
    }
}

#[test]
fn a_distinct_path_is_listed_once_and_fenced_code_is_skipped() {
    let body = "`src/a.rs` `src/a.rs:3`\n\n```\n`src/b.rs`\nsrc/c.rs\n```\n";
    assert_eq!(candidate_paths(body), vec!["src/a.rs".to_owned()]);
}

struct Probing;

impl DocStore for Probing {
    fn list(&self, _root: &Path) -> std::io::Result<Vec<PathBuf>> {
        Ok(Vec::new())
    }

    fn read(&self, _path: &Path) -> std::io::Result<String> {
        Ok(String::new())
    }

    fn write(&self, _path: &Path, _contents: &str) -> std::io::Result<()> {
        Ok(())
    }

    fn rename(&self, _from: &Path, _to: &Path) -> std::io::Result<()> {
        Ok(())
    }

    fn repository_entry_exists(&self, relative: &Path) -> Option<bool> {
        match relative.to_str()? {
            "present" => Some(true),
            "absent" => Some(false),
            _ => None,
        }
    }
}

fn answers(store: &dyn DocStore) -> [Option<bool>; 3] {
    ["present", "absent", "unknown"].map(|name| store.repository_entry_exists(Path::new(name)))
}

#[test]
fn the_line_ending_wrapper_passes_the_probe_through() {
    let inner = Probing;
    let wrapped = crate::line_ending::LineEndingStore::new(&inner, true);
    assert_eq!(answers(&wrapped), [Some(true), Some(false), None]);
}

#[test]
fn the_read_once_wrapper_passes_the_probe_through() {
    let inner = Probing;
    let wrapped = crate::check::read_once::ReadOnceStore::new(&inner);
    assert_eq!(answers(&wrapped), [Some(true), Some(false), None]);
}

struct Serving;

impl DocStore for Serving {
    fn list(&self, _root: &Path) -> std::io::Result<Vec<PathBuf>> {
        Ok(Vec::new())
    }

    fn read(&self, _path: &Path) -> std::io::Result<String> {
        Ok(String::new())
    }

    fn write(&self, _path: &Path, _contents: &str) -> std::io::Result<()> {
        Ok(())
    }

    fn rename(&self, _from: &Path, _to: &Path) -> std::io::Result<()> {
        Ok(())
    }

    fn repository_files(&self, _globs: &[String]) -> Option<crate::store::RepositoryFiles> {
        Some(Ok(vec![(PathBuf::from("a.txt"), b"x".to_vec())]))
    }
}

fn served(store: &dyn DocStore) -> Vec<PathBuf> {
    let files = store.repository_files(&["*".to_owned()]);
    let files = files.expect("the store serves files").unwrap();
    files.into_iter().map(|(path, _)| path).collect()
}

#[test]
fn the_line_ending_wrapper_passes_the_repository_files_through() {
    let inner = Serving;
    let wrapped = crate::line_ending::LineEndingStore::new(&inner, true);
    assert_eq!(served(&wrapped), vec![PathBuf::from("a.txt")]);
}

#[test]
fn the_read_once_wrapper_passes_the_repository_files_through() {
    let inner = Serving;
    let wrapped = crate::check::read_once::ReadOnceStore::new(&inner);
    assert_eq!(served(&wrapped), vec![PathBuf::from("a.txt")]);
}
