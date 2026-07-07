//! Dev-mode file watcher: polls the mtimes of `*.lua` files under a root
//! directory and sends one `Msg::Reload` per batch of changes through the
//! channel consumed by the Lua thread. Plain std polling — no platform
//! notification APIs, no extra dependency; the interval is the debounce.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use tokio::sync::mpsc;

use crate::types::Msg;

const POLL_INTERVAL: Duration = Duration::from_millis(300);

/// Directories that never contain application sources: the Rust build
/// output and the per-project rock tree (`luarocks --tree lua_modules`).
/// Shared with the bundler, so `ludi build` packs exactly what the
/// watcher watches.
pub(crate) const SKIPPED_DIRS: &[&str] = &["target", "lua_modules"];

pub fn spawn(root: PathBuf, tx: mpsc::UnboundedSender<Msg>) {
    std::thread::spawn(move || {
        let mut seen = scan(&root);
        loop {
            std::thread::sleep(POLL_INTERVAL);
            let current = scan(&root);
            let changed = diff(&seen, &current);
            seen = current;
            if !changed.is_empty() && tx.send(Msg::Reload(changed)).is_err() {
                return; // Lua thread gone, stop watching
            }
        }
    });
}

/// Paths added, removed, or with a different mtime, sorted for stable output.
fn diff(
    before: &HashMap<PathBuf, SystemTime>,
    after: &HashMap<PathBuf, SystemTime>,
) -> Vec<String> {
    let mut changed: Vec<String> = after
        .iter()
        .filter(|(path, mtime)| before.get(*path) != Some(mtime))
        .map(|(path, _)| path.display().to_string())
        .chain(
            before
                .keys()
                .filter(|path| !after.contains_key(*path))
                .map(|path| path.display().to_string()),
        )
        .collect();
    changed.sort();
    changed
}

fn scan(root: &Path) -> HashMap<PathBuf, SystemTime> {
    let mut out = HashMap::new();
    scan_into(root, &mut out);
    out
}

fn scan_into(dir: &Path, out: &mut HashMap<PathBuf, SystemTime>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            if !SKIPPED_DIRS.contains(&name.as_ref()) {
                scan_into(&path, out);
            }
        } else if path.extension().is_some_and(|ext| ext == "lua") {
            if let Ok(mtime) = entry.metadata().and_then(|meta| meta.modified()) {
                out.insert(path, mtime);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(dir: &Path, name: &str, contents: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn scan_finds_lua_files_recursively_and_skips_noise() {
        let dir = std::env::temp_dir().join(format!("ludi-watch-{}", std::process::id()));
        let nested = dir.join("routes");
        let skipped = dir.join("lua_modules");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::create_dir_all(&skipped).unwrap();

        let app = touch(&dir, "app.lua", "-- app");
        let route = touch(&nested, "users.lua", "-- users");
        touch(&dir, "notes.txt", "not lua");
        touch(&skipped, "vendored.lua", "-- vendored");

        let found = scan(&dir);
        assert!(found.contains_key(&app));
        assert!(found.contains_key(&route));
        assert_eq!(found.len(), 2, "found: {:?}", found.keys());

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn diff_reports_added_changed_and_removed() {
        let file_a = PathBuf::from("a.lua");
        let file_b = PathBuf::from("b.lua");
        let file_c = PathBuf::from("c.lua");
        let early = SystemTime::UNIX_EPOCH;
        let late = early + Duration::from_secs(1);

        let before = HashMap::from([(file_a.clone(), early), (file_b.clone(), early)]);
        let after = HashMap::from([(file_a.clone(), late), (file_c.clone(), early)]);

        assert_eq!(diff(&before, &after), vec!["a.lua", "b.lua", "c.lua"]);
        assert!(diff(&before, &before).is_empty());
    }
}
