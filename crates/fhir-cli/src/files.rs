//! Input discovery: explicit files plus `*.json` / `*.xml` found under directories.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Expand `paths` into the list of files to lint, in a stable order.
///
/// Explicit files are kept whatever their extension; directories are walked
/// recursively for `*.json` and `*.xml`. Symlinked directories are skipped so
/// a link cycle cannot loop forever. A missing path is an error.
pub fn collect(paths: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    for path in paths {
        let metadata =
            fs::metadata(path).map_err(|e| format!("cannot access '{}': {e}", path.display()))?;
        if metadata.is_dir() {
            walk(path, &mut out)?;
        } else if metadata.is_file() {
            out.push(path.clone());
        } else {
            // A FIFO or device could block the read forever.
            return Err(format!(
                "cannot read '{}': not a regular file",
                path.display()
            ));
        }
    }
    // Overlapping inputs must not repeat findings; the first spelling wins.
    let mut seen = HashSet::new();
    out.retain(|path| seen.insert(fs::canonicalize(path).unwrap_or_else(|_| path.clone())));
    Ok(out)
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let read_error = |e: std::io::Error| format!("cannot read directory '{}': {e}", dir.display());
    let mut entries = fs::read_dir(dir)
        .map_err(read_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(read_error)?;
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type().map_err(read_error)?;
        if file_type.is_dir() {
            walk(&path, out)?;
        } else if is_resource(&path) && (file_type.is_file() || path.is_file()) {
            out.push(path);
        }
    }
    Ok(())
}

fn is_resource(path: &Path) -> bool {
    is_xml(path) || has_extension(path, "json")
}

/// XML inputs are recognised by extension; everything else is parsed as JSON.
pub fn is_xml(path: &Path) -> bool {
    has_extension(path, "xml")
}

fn has_extension(path: &Path, extension: &str) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(extension))
}
