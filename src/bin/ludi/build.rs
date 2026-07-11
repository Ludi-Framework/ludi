//! `ludi build`: packs the current directory into a self-contained binary.

use std::path::Path;
use std::process::ExitCode;

use ludi_core::bundle;

pub fn run(args: &[String]) -> ExitCode {
    match try_build(args) {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("ludi build: {err}");
            ExitCode::FAILURE
        }
    }
}

fn try_build(args: &[String]) -> Result<String, String> {
    let mut entry: Option<String> = None;
    let mut output: Option<String> = None;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-o" | "--output" => {
                output = Some(
                    iter.next()
                        .ok_or_else(|| format!("{arg} expects a file name"))?
                        .clone(),
                );
            }
            flag if flag.starts_with('-') => return Err(format!("unknown flag {flag:?}")),
            positional if entry.is_none() => entry = Some(positional.to_string()),
            extra => return Err(format!("unexpected argument {extra:?}")),
        }
    }

    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let files = bundle::collect_files(&cwd).map_err(|e| e.to_string())?;

    let entry = match entry {
        Some(given) => {
            let normalized = given.strip_prefix("./").unwrap_or(&given).to_string();
            if !files.iter().any(|(path, _)| *path == normalized) {
                return Err(format!(
                    "entrypoint {given:?} not found under {}",
                    cwd.display()
                ));
            }
            normalized
        }
        None => detect_entry(&files)?,
    };

    let output = output.unwrap_or_else(|| {
        cwd.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "app".to_string())
    });

    // Windows only launches files with an executable extension, so the
    // artifact has to end in `.exe` no matter what name it was given
    // (the directory default rarely carries one). Elsewhere the name is
    // left untouched.
    #[cfg(windows)]
    let output = if Path::new(&output)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
    {
        output
    } else {
        format!("{output}.exe")
    };

    let own_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let runtime = std::fs::read(&own_exe).map_err(|e| e.to_string())?;
    if bundle::extract_payload(&runtime).is_some() {
        return Err(
            "this executable already contains an app; build with the plain ludi CLI".into(),
        );
    }

    let payload = bundle::encode(&entry, &files);
    let binary = bundle::assemble(&runtime, &payload);
    let size = binary.len();

    write_executable(Path::new(&output), &binary).map_err(|e| e.to_string())?;

    Ok(format!(
        "built {output} ({} files, entry {entry}, {:.1} MB)",
        files.len(),
        size as f64 / (1024.0 * 1024.0)
    ))
}

/// Conventional entrypoint names, accepted only when unambiguous: with two
/// or more present there is no safe guess (an `app.lua` next to a
/// `server.lua` is usually a module, not the runner), so the user must
/// pass the entrypoint explicitly.
const ENTRY_CANDIDATES: &[&str] = &["app.lua", "server.lua", "main.lua", "init.lua"];

fn detect_entry(files: &[(String, Vec<u8>)]) -> Result<String, String> {
    let present: Vec<&str> = ENTRY_CANDIDATES
        .iter()
        .copied()
        .filter(|name| files.iter().any(|(path, _)| path == name))
        .collect();

    match present.as_slice() {
        [only] => Ok(only.to_string()),
        [] => Err(format!(
            "no entrypoint found (looked for {}); pass one: ludi build <entry.lua>",
            ENTRY_CANDIDATES.join(", ")
        )),
        several => Err(format!(
            "several possible entrypoints here ({}); pass one: ludi build <entry.lua>",
            several.join(", ")
        )),
    }
}

fn write_executable(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}
