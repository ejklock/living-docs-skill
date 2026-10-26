use super::*;

pub type Tree = Vec<(String, String)>;

/// Every file under `dir` as (relative path, text), sorted, timestamps masked.
pub fn tree(dir: &Path) -> Tree {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in fs::read_dir(&next).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let rel = path.strip_prefix(dir).unwrap().display().to_string();
                let text = fs::read_to_string(&path).unwrap();
                let stamp = text.find("\ntimestamp: ").map(|at| at + 12);
                let text = match stamp {
                    Some(at) => format!("{}<stamp>{}", &text[..at], &text[at + 20..]),
                    None => text,
                };
                files.push((rel, text));
            }
        }
    }
    files.sort();
    files
}

/// Scaffolds records One and Two of `doc_type`, lets `prepare` rewrite the
/// bundle, runs `args` (expecting success) and returns the resulting tree.
pub fn run_prepared(label: &str, doc_type: &str, prepare: &dyn Fn(&Path), args: &[&str]) -> Tree {
    let docs = temp_bundle("set-read-back", label);
    assert!(run_new(&docs, doc_type, "One").status.success());
    assert!(run_new(&docs, doc_type, "Two").status.success());
    prepare(&docs);
    let out = run(&docs, args);
    assert_eq!(out.status.code(), Some(0), "{label}: {out:?}");
    tree(&docs)
}

pub fn to_crlf(docs: &Path, files: &[&str]) {
    for rel in files {
        let text = fs::read_to_string(docs.join(rel)).unwrap();
        fs::write(docs.join(rel), text.replace('\n', "\r\n")).unwrap();
    }
}

/// Runs `args` on an LF bundle and on its twin whose `crlf` files (relative
/// to the bundle) are CRLF, and returns both trees afterwards.
fn twin_run(label: &str, doc_type: &str, crlf: &[&str], args: &[&str]) -> (Tree, Tree) {
    let lf = run_prepared(&format!("{label}-lf"), doc_type, &|_| {}, args);
    let prepare = |docs: &Path| to_crlf(docs, crlf);
    (
        lf,
        run_prepared(&format!("{label}-crlf"), doc_type, &prepare, args),
    )
}

/// Each CRLF file is its LF twin with every line break doubled, without a
/// bare LF; each LF file is its twin unchanged.
fn assert_twins(lf: &Tree, crlf: &Tree) {
    assert_eq!(lf.len(), crlf.len());
    for ((lf_rel, lf_text), (crlf_rel, crlf_text)) in lf.iter().zip(crlf) {
        assert_eq!(lf_rel, crlf_rel);
        if crlf_text.contains('\r') {
            assert_eq!(crlf_text, &lf_text.replace('\n', "\r\n"), "{lf_rel}");
            assert!(!crlf_text.replace("\r\n", "").contains('\n'), "{lf_rel}");
        } else {
            assert_eq!(crlf_text, lf_text, "{lf_rel}");
        }
    }
}

const ADR_FILES: [&str; 3] = ["adr/0001-one.md", "adr/0002-two.md", "adr/index.md"];

/// The tree holds at least one file under `adr/`, and each keeps CRLF endings
/// with no bare LF.
fn assert_typed_files_stay_crlf(crlf: &Tree) {
    let typed: Vec<_> = crlf
        .iter()
        .filter(|(rel, _)| rel.starts_with("adr/"))
        .collect();
    assert!(!typed.is_empty());
    for (rel, text) in typed {
        assert!(text.contains("\r\n"), "{rel} stays CRLF");
        assert!(
            !text.replace("\r\n", "").contains('\n'),
            "{rel} has a bare LF"
        );
    }
}

#[test]
fn crlf_adr_writes_equal_the_lf_twin_with_crlf() {
    let section = std::env::temp_dir().join("living-docs-set-read-back-section.txt");
    fs::write(&section, "Added text.\n").unwrap();
    let from = section.to_str().unwrap();
    let commands: [&[&str]; 5] = [
        &["set", "0001", "description", "a new sentence"],
        &["set", "0001", "owner", "someone"],
        &["set", "0001", "status", "Accepted"],
        &["set", "0001", "title", "Renamed"],
        &["set", "0001", "section", "Notes", "--from", from],
    ];
    for (n, args) in commands.iter().enumerate() {
        let (lf, crlf) = twin_run(&format!("adr{n}"), "adr", &ADR_FILES, args);
        assert_twins(&lf, &crlf);
        assert_typed_files_stay_crlf(&crlf);
        let root = |tree: &Tree| tree.iter().find(|(rel, _)| rel == "index.md").cloned();
        assert_eq!(
            root(&crlf),
            root(&lf),
            "the root index keeps its own ending"
        );
    }
    let _ = fs::remove_file(section);
}

#[test]
fn crlf_issue_writes_equal_the_lf_twin_with_crlf() {
    let files = ["issues/0001-one.md", "issues/index.md"];
    let commands: [&[&str]; 2] = [
        &["set", "0001", "tracker", "12"],
        &["set", "0001", "labels", "a, b"],
    ];
    for (n, args) in commands.iter().enumerate() {
        let (lf, crlf) = twin_run(&format!("issue{n}"), "issue", &files, args);
        assert_twins(&lf, &crlf);
        let record = crlf.iter().find(|(rel, _)| rel == files[0]).unwrap();
        assert!(record.1.contains("\r\n"), "the record stays CRLF");
    }
}

#[test]
fn crlf_supersede_equals_the_lf_twin_in_either_direction() {
    let sides = [
        ["adr/0001-one.md", "adr/index.md"],
        ["adr/0002-two.md", "adr/index.md"],
    ];
    for (n, crlf_side) in sides.iter().enumerate() {
        let args = ["supersede", "0001", "0002"];
        let (lf, crlf) = twin_run(&format!("sup{n}"), "adr", crlf_side, &args);
        assert_twins(&lf, &crlf);
        let crlf_files = crlf.iter().filter(|(_, t)| t.contains('\r')).count();
        assert_eq!(crlf_files, 2, "only the CRLF record and index are CRLF");
    }
}

/// A record whose last line has no terminator.
fn without_final_newline(docs: &Path, crlf: bool) {
    let path = docs.join("adr/0001-one.md");
    let text = fs::read_to_string(&path).unwrap();
    let text = if crlf {
        text.replace('\n', "\r\n")
    } else {
        text
    };
    let trimmed = text.trim_end_matches(['\r', '\n']);
    assert!(trimmed.len() < text.len());
    fs::write(path, trimmed).unwrap();
}

#[test]
fn set_adds_the_missing_final_newline_like_a_terminated_record_in_either_ending() {
    let args = ["set", "0001", "owner", "bob"];
    let plain = run_prepared("eof-base", "adr", &|_| {}, &args);
    let lf = run_prepared("eof-lf", "adr", &|d| without_final_newline(d, false), &args);
    assert_eq!(lf, plain);
    let crlf = run_prepared(
        "eof-crlf",
        "adr",
        &|d| without_final_newline(d, true),
        &args,
    );
    let record = crlf
        .iter()
        .position(|(rel, _)| rel == "adr/0001-one.md")
        .unwrap();
    assert_eq!(crlf[record].1, plain[record].1.replace('\n', "\r\n"));
}
