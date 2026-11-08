use std::fs;
use std::path::Path;

use chrono::Local;
use serde::{Deserialize, Serialize};

/// Recents index (spec 2.6): a dedicated file next to the notes, never a directory scan.
const INDEX_FILE: &str = ".ntbk";

#[derive(Serialize, Deserialize, Default)]
struct NoteIndex {
    entries: Vec<IndexEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
struct IndexEntry {
    name: String,
    last_edited: String,
}

/// Recents list, most recently edited first (guaranteed by construction on record).
pub fn load(base: &Path) -> Vec<String> {
    read_index(base)
        .entries
        .into_iter()
        .map(|e| e.name)
        .collect()
}

/// Move `name` to the front with a fresh timestamp and rewrite the index file.
pub fn record(base: &Path, name: &str) -> std::io::Result<()> {
    let mut index = read_index(base);
    index.entries.retain(|e| e.name != name);
    index.entries.insert(
        0,
        IndexEntry {
            name: name.to_string(),
            last_edited: Local::now().to_rfc3339(),
        },
    );
    let json = serde_json::to_string_pretty(&index).unwrap_or_default();
    fs::write(base.join(INDEX_FILE), json)
}

fn read_index(base: &Path) -> NoteIndex {
    match fs::read_to_string(base.join(INDEX_FILE)) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => NoteIndex::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_base(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ntbk-index-{tag}-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn missing_index_loads_empty() {
        let base = temp_base("missing");
        assert!(load(&base).is_empty());
    }

    #[test]
    fn corrupt_index_loads_empty() {
        let base = temp_base("corrupt");
        fs::write(base.join(INDEX_FILE), "not json at all").unwrap();
        assert!(load(&base).is_empty());
    }

    #[test]
    fn record_moves_name_to_front_most_recent_first() {
        let base = temp_base("order");
        record(&base, "a.txt").unwrap();
        record(&base, "b.txt").unwrap();
        record(&base, "c.txt").unwrap();
        record(&base, "a.txt").unwrap();
        assert_eq!(load(&base), vec!["a.txt", "c.txt", "b.txt"]);
    }
}
