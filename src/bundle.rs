//! Single-binary bundles (`ludi build`). A bundle is the `ludi` runtime
//! executable with the application's `*.lua` files appended, followed by a
//! fixed-size trailer:
//!
//! ```text
//! [runtime executable] [payload] [payload length: u64 LE] [magic: 8 bytes]
//! ```
//!
//! At startup the binary reads its own trailer: magic present means "run
//! the embedded app", absent means "act as the CLI". The payload is a
//! length-prefixed list of (path, content) pairs plus the entrypoint name —
//! no compression, no external format, so decoding needs nothing beyond
//! std.

use std::collections::HashMap;
use std::io::{self, Read};
use std::path::Path;

use crate::watch::SKIPPED_DIRS;

pub const MAGIC: &[u8; 8] = b"LUDIPKG1";
const TRAILER_LEN: usize = 8 + MAGIC.len();

pub struct Bundle {
    /// Path (as bundled, e.g. `"server.lua"`) of the script to execute.
    pub entry: String,
    /// Relative path with `/` separators -> file contents.
    pub files: HashMap<String, Vec<u8>>,
}

/// Every `*.lua` file under `root`, as (relative path, contents), sorted by
/// path. Skips hidden directories and the usual non-source trees, same
/// rules as the dev-mode watcher.
pub fn collect_files(root: &Path) -> io::Result<Vec<(String, Vec<u8>)>> {
    let mut files = Vec::new();
    walk(root, root, &mut files)?;
    files.sort_by(|(a, _), (b, _)| a.cmp(b));
    Ok(files)
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) -> io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            if !SKIPPED_DIRS.contains(&name.as_ref()) {
                walk(root, &path, out)?;
            }
        } else if path.extension().is_some_and(|ext| ext == "lua") {
            let relative = path
                .strip_prefix(root)
                .expect("walk stays under root")
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            out.push((relative, std::fs::read(&path)?));
        }
    }
    Ok(())
}

pub fn encode(entry: &str, files: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    push_chunk(&mut out, entry.as_bytes());
    out.extend_from_slice(&(files.len() as u32).to_le_bytes());
    for (path, content) in files {
        push_chunk(&mut out, path.as_bytes());
        push_chunk(&mut out, content);
    }
    out
}

fn push_chunk(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
}

pub fn decode(payload: &[u8]) -> Option<Bundle> {
    let mut cursor = payload;
    let entry = String::from_utf8(read_chunk(&mut cursor)?).ok()?;
    let count = read_u32(&mut cursor)?;
    let mut files = HashMap::with_capacity(count as usize);
    for _ in 0..count {
        let path = String::from_utf8(read_chunk(&mut cursor)?).ok()?;
        files.insert(path, read_chunk(&mut cursor)?);
    }
    Some(Bundle { entry, files })
}

fn read_u32(cursor: &mut &[u8]) -> Option<u32> {
    let (len, rest) = cursor.split_first_chunk::<4>()?;
    *cursor = rest;
    Some(u32::from_le_bytes(*len))
}

fn read_chunk(cursor: &mut &[u8]) -> Option<Vec<u8>> {
    let len = read_u32(cursor)? as usize;
    if cursor.len() < len {
        return None;
    }
    let (chunk, rest) = cursor.split_at(len);
    *cursor = rest;
    Some(chunk.to_vec())
}

/// The payload appended to `exe_bytes`, if the trailer says there is one.
pub fn extract_payload(exe_bytes: &[u8]) -> Option<&[u8]> {
    let trailer_at = exe_bytes.len().checked_sub(TRAILER_LEN)?;
    let (rest, trailer) = exe_bytes.split_at(trailer_at);
    if &trailer[8..] != MAGIC {
        return None;
    }
    let len = u64::from_le_bytes(trailer[..8].try_into().unwrap()) as usize;
    rest.get(rest.len().checked_sub(len)?..)
}

/// Reads the running executable and decodes its bundle, if any.
pub fn read_own_bundle() -> io::Result<Option<Bundle>> {
    let exe = std::env::current_exe()?;
    let mut bytes = Vec::new();
    std::fs::File::open(exe)?.read_to_end(&mut bytes)?;
    Ok(extract_payload(&bytes).and_then(decode))
}

/// `runtime ++ payload ++ trailer`, ready to be written as the output
/// executable.
pub fn assemble(runtime: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(runtime.len() + payload.len() + TRAILER_LEN);
    out.extend_from_slice(runtime);
    out.extend_from_slice(payload);
    out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    out.extend_from_slice(MAGIC);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_files() -> Vec<(String, Vec<u8>)> {
        vec![
            ("app.lua".into(), b"return 1".to_vec()),
            ("routes/users.lua".into(), b"return {}".to_vec()),
        ]
    }

    #[test]
    fn encode_decode_roundtrip() {
        let payload = encode("app.lua", &sample_files());
        let bundle = decode(&payload).unwrap();

        assert_eq!(bundle.entry, "app.lua");
        assert_eq!(bundle.files.len(), 2);
        assert_eq!(bundle.files["routes/users.lua"], b"return {}");
    }

    #[test]
    fn assemble_extract_roundtrip() {
        let runtime = b"fake-elf-bytes";
        let payload = encode("app.lua", &sample_files());

        let binary = assemble(runtime, &payload);
        assert_eq!(extract_payload(&binary).unwrap(), &payload[..]);
    }

    #[test]
    fn plain_executable_has_no_payload() {
        assert!(extract_payload(b"fake-elf-bytes-without-trailer").is_none());
        assert!(extract_payload(b"tiny").is_none());
    }

    #[test]
    fn truncated_payload_is_rejected() {
        let payload = encode("app.lua", &sample_files());
        assert!(decode(&payload[..payload.len() - 3]).is_none());
    }

    #[test]
    fn collect_files_walks_and_skips_like_the_watcher() {
        let dir = std::env::temp_dir().join(format!("ludi-bundle-{}", std::process::id()));
        let nested = dir.join("routes");
        let skipped = dir.join("lua_modules");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::create_dir_all(&skipped).unwrap();
        std::fs::write(dir.join("app.lua"), "-- app").unwrap();
        std::fs::write(nested.join("users.lua"), "-- users").unwrap();
        std::fs::write(dir.join("notes.txt"), "not lua").unwrap();
        std::fs::write(skipped.join("vendored.lua"), "-- vendored").unwrap();

        let files = collect_files(&dir).unwrap();
        let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(paths, ["app.lua", "routes/users.lua"]);

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
