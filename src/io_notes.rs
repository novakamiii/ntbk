use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::Local;

/// Resolve the modal's raw input into a concrete filename.
///
/// - Path separators are stripped so a note can never escape the notes directory.
/// - An empty result falls back to a timestamped name (spec 2.4).
/// - A missing extension gets `.txt`; any extension other than txt/md also gets `.txt`
///   appended, so every file written is one of the two supported formats (spec 2.6).
pub fn resolve_filename(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | '\0'))
        .collect();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        return format!("{}.txt", Local::now().format("%Y-%m-%d"));
    }
    match Path::new(cleaned).extension().and_then(|e| e.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("txt") || ext.eq_ignore_ascii_case("md") => {
            cleaned.to_string()
        }
        _ => format!("{cleaned}.txt"),
    }
}

pub fn save_note(base: &Path, name: &str, content: &str) -> io::Result<PathBuf> {
    let path = base.join(name);
    fs::write(&path, content)?;
    Ok(path)
}

pub fn load_note(base: &Path, name: &str) -> io::Result<String> {
    fs::read_to_string(base.join(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_base(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ntbk-notes-{tag}-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn appends_txt_when_extension_missing() {
        assert_eq!(resolve_filename("groceries"), "groceries.txt");
    }

    #[test]
    fn keeps_txt_and_md_extensions() {
        assert_eq!(resolve_filename("note.txt"), "note.txt");
        assert_eq!(resolve_filename("note.md"), "note.md");
        assert_eq!(resolve_filename("NOTE.MD"), "NOTE.MD");
    }

    #[test]
    fn appends_txt_after_unsupported_extension() {
        assert_eq!(resolve_filename("file.rtf"), "file.rtf.txt");
    }

    #[test]
    fn empty_name_falls_back_to_date() {
        let name = resolve_filename("");
        assert!(name.ends_with(".txt"));
        let stem = name.trim_end_matches(".txt");
        assert_eq!(stem.len(), 10);
        assert!(
            stem.chars().filter(|c| *c == '-').count() == 2,
            "expected YYYY-MM-DD, got {name}"
        );
    }

    #[test]
    fn path_separators_are_stripped() {
        assert_eq!(resolve_filename("a/b\\c"), "abc.txt");
    }

    #[test]
    fn whitespace_only_name_falls_back() {
        assert!(resolve_filename("   ").ends_with(".txt"));
    }

    #[test]
    fn save_and_load_round_trip() {
        let base = temp_base("roundtrip");
        let name = resolve_filename("roundtrip-check");
        save_note(&base, &name, "hello\nworld").unwrap();
        assert_eq!(load_note(&base, &name).unwrap(), "hello\nworld");
    }
}
