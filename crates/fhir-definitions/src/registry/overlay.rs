//! Supplied definitions layered over a base registry.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;

use super::Registry;
use crate::input::{MAX_INPUT_BYTES, read_capped};
use crate::loader::{self, Definitions};
use crate::{Error, Result};

/// Largest total size of the supplied files (256 MiB).
const MAX_TOTAL_BYTES: u64 = 256 * 1024 * 1024;
/// Most `.json` files read from one directory.
const MAX_DIRECTORY_FILES: usize = 10_000;

impl Registry {
    /// A registry holding the definitions in `paths` on top of `base`.
    ///
    /// Each path is a `.json` file holding one resource or a Bundle, or a directory
    /// whose `*.json` files are read (not recursively; symlinks and other files are
    /// skipped). StructureDefinitions, ValueSets, CodeSystems and ConceptMaps are loaded; every
    /// lookup reads them first, then falls back to `base`, which is left untouched.
    /// The overlay is not cached: build it once per validation context.
    /// Other resources, and definitions the registry skips (no url, name or kind),
    /// are ignored, as in the bundled definitions.
    ///
    /// Errors, each naming the file: [`Error::Conflict`] when a canonical URL is
    /// already in `base` or appears twice among the supplied files;
    /// [`Error::Unsupported`] for a StructureDefinition with
    /// `derivation = specialization` or a path that is no `.json` file or directory;
    /// [`Error::Read`] for an unreadable file or one over [`MAX_INPUT_BYTES`];
    /// [`Error::Limit`] past 256 MiB in total or 10,000 files in one directory.
    /// A StructureDefinition without a snapshot is not stored: see
    /// [`Registry::needs_snapshot`].
    pub fn with_extra(base: Arc<Registry>, paths: &[impl AsRef<Path>]) -> Result<Registry> {
        let mut defs = Definitions::default();
        let mut seen = HashSet::new();
        let mut total_bytes = 0;
        for path in paths {
            for file in definition_files(path.as_ref())? {
                let name = file.display().to_string();
                let bytes = read_capped(&file, MAX_INPUT_BYTES).map_err(|source| Error::Read {
                    file: name.clone(),
                    source,
                })?;
                total_bytes += bytes.len() as u64;
                if total_bytes > MAX_TOTAL_BYTES {
                    return Err(Error::Limit {
                        file: name,
                        message: format!(
                            "supplied definitions exceed the {} MiB total limit",
                            MAX_TOTAL_BYTES / (1024 * 1024)
                        ),
                    });
                }
                let value: Value =
                    serde_json::from_slice(&bytes).map_err(|source| Error::Json {
                        file: name.clone(),
                        source,
                    })?;
                drop(bytes);
                let resources: Vec<&Value> = if value["resourceType"] == "Bundle" {
                    // An absent `entry` is an empty Bundle; any other non-array is malformed.
                    let entries = match &value["entry"] {
                        Value::Null => &[][..],
                        Value::Array(entries) => entries,
                        _ => {
                            return Err(Error::MalformedBundle {
                                file: name,
                                message: "'entry' is not an array".to_owned(),
                            });
                        }
                    };
                    entries.iter().map(|entry| &entry["resource"]).collect()
                } else {
                    vec![&value]
                };
                for resource in resources {
                    load_extra(resource, &name, &base, &mut defs, &mut seen)?;
                }
            }
        }
        Ok(Self::from_loaded(defs, Some(base)))
    }
}

/// Load one supplied resource into `defs`, rejecting specializations and
/// canonical URLs already in `base` or in `seen` (per resource type).
fn load_extra(
    resource: &Value,
    file: &str,
    base: &Registry,
    defs: &mut Definitions,
    seen: &mut HashSet<(Arc<str>, Arc<str>)>,
) -> Result<()> {
    let Some(resource_type) = resource["resourceType"].as_str() else {
        return Ok(());
    };
    if resource_type == "StructureDefinition" && resource["derivation"] == "specialization" {
        return Err(Error::Unsupported {
            file: file.to_owned(),
            message: format!(
                "StructureDefinition {} is a specialization; only constraints are supported",
                resource["url"].as_str().unwrap_or("without url")
            ),
        });
    }
    let Some(url) = loader::load_resource(resource, file, defs)? else {
        return Ok(());
    };
    let in_base = match resource_type {
        "StructureDefinition" => {
            base.structure_definition(&url).is_some() || base.needs_snapshot(&url)
        }
        "ValueSet" => base.value_set(&url).is_some(),
        "CodeSystem" => base.code_system(&url).is_some(),
        _ => base.concept_map(&url).is_some(),
    };
    if in_base || !seen.insert((Arc::from(resource_type), Arc::clone(&url))) {
        return Err(Error::Conflict {
            url: url.to_string(),
            file: file.to_owned(),
        });
    }
    Ok(())
}

/// The files a supplied path names: itself if it is a `.json` file, or the
/// `.json` regular files directly inside it, sorted, if it is a directory.
fn definition_files(path: &Path) -> Result<Vec<PathBuf>> {
    let read_error = |source: io::Error| Error::Read {
        file: path.display().to_string(),
        source,
    };
    let unsupported = |message: &str| Error::Unsupported {
        file: path.display().to_string(),
        message: message.to_owned(),
    };
    let canonical = fs::canonicalize(path).map_err(read_error)?;
    let metadata = fs::metadata(&canonical).map_err(read_error)?;
    if metadata.is_file() {
        return if is_json(&canonical) {
            Ok(vec![canonical])
        } else {
            Err(unsupported("not a .json file"))
        };
    }
    if !metadata.is_dir() {
        return Err(unsupported("neither a regular file nor a directory"));
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(&canonical).map_err(read_error)? {
        let entry = entry.map_err(read_error)?;
        // `file_type` does not follow symlinks, so a link is skipped, never read.
        if !entry.file_type().map_err(read_error)?.is_file() || !is_json(&entry.path()) {
            continue;
        }
        if files.len() == MAX_DIRECTORY_FILES {
            return Err(Error::Limit {
                file: path.display().to_string(),
                message: format!("directory holds more than {MAX_DIRECTORY_FILES} .json files"),
            });
        }
        files.push(entry.path());
    }
    files.sort();
    Ok(files)
}

fn is_json(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "json")
}

#[cfg(test)]
mod tests;
