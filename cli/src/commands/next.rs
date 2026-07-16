use std::fs;
use std::path::Path;
use std::process::ExitCode;

pub fn run(docs_dir: &Path, doc_type: &str) -> ExitCode {
    match next_number(docs_dir, doc_type) {
        Ok(n) => {
            println!("{n:04}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("living-docs next: {e}");
            ExitCode::from(2)
        }
    }
}

/// Highest existing `NNNN` under `docs_dir/doc_type`, plus one. `doc_type`
/// here is the resolved directory name (e.g. `issues`, not `issue`) — `new`
/// reuses this to avoid duplicating the allocation logic.
pub fn next_number(docs_dir: &Path, doc_type: &str) -> std::io::Result<u32> {
    let type_dir = docs_dir.join(doc_type);
    let highest = match fs::read_dir(&type_dir) {
        Ok(entries) => highest_numeric_prefix(entries),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => 0,
        Err(e) => return Err(e),
    };
    Ok(highest + 1)
}

fn highest_numeric_prefix(entries: fs::ReadDir) -> u32 {
    entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| numeric_prefix(&entry.file_name().to_string_lossy()))
        .max()
        .unwrap_or(0)
}

fn numeric_prefix(filename: &str) -> Option<u32> {
    if !filename.ends_with(".md") || filename.as_bytes().get(4) != Some(&b'-') {
        return None;
    }
    let prefix = filename.get(0..4)?;
    if !prefix.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    prefix.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_prefix_accepts_four_digit_dash_form() {
        assert_eq!(numeric_prefix("0007-old.md"), Some(7));
    }

    #[test]
    fn numeric_prefix_rejects_non_matching_filenames() {
        assert_eq!(numeric_prefix("index.md"), None);
        assert_eq!(numeric_prefix("notes.txt"), None);
        assert_eq!(numeric_prefix("12-old.md"), None);
        assert_eq!(numeric_prefix("abcd-old.md"), None);
    }
}
