//! Locating the definitions directory for callers without a CLI (the language bindings).

use std::env;
use std::path::Path;
use std::sync::Arc;

use thiserror::Error;

use crate::{Registry, Result};

/// Environment variable naming the definitions directory when none is passed.
pub const DEFINITIONS_DIR_ENV: &str = "FHIR_DEFINITIONS_DIR";

/// Why [`Registry::shared_or_env_with_extra`] failed; the messages are the bindings' own.
#[derive(Debug, Error)]
pub enum SharedRegistryError {
    #[error("cannot load definitions: {0}")]
    Base(crate::Error),

    // fhir-definitions errors name the offending file, never its content.
    #[error("cannot load extra definitions: {0}")]
    Extra(crate::Error),
}

impl Registry {
    /// The shared registry for `dir`, or for [`DEFINITIONS_DIR_ENV`] when `dir` is `None`.
    ///
    /// Returns `None` when neither names a directory (an unset or non-UTF-8 variable),
    /// so each caller can word that error itself. Loading goes through [`Registry::shared`].
    pub fn shared_or_env(dir: Option<String>) -> Option<Result<Arc<Self>>> {
        Self::shared_or_else(dir, |key| env::var(key).ok())
    }

    /// [`Registry::shared_or_env`] with `extra` layered on top through
    /// [`Registry::with_extra`]; the base itself when `extra` is empty.
    ///
    /// Built per call, never cached. `None` as in `shared_or_env`.
    pub fn shared_or_env_with_extra(
        dir: Option<String>,
        extra: &[impl AsRef<Path>],
    ) -> Option<std::result::Result<Arc<Self>, SharedRegistryError>> {
        Self::shared_or_else_with_extra(dir, extra, |key| env::var(key).ok())
    }

    fn shared_or_else_with_extra(
        dir: Option<String>,
        extra: &[impl AsRef<Path>],
        lookup_env: impl FnOnce(&str) -> Option<String>,
    ) -> Option<std::result::Result<Arc<Self>, SharedRegistryError>> {
        let base = match Self::shared_or_else(dir, lookup_env)? {
            Ok(base) => base,
            Err(e) => return Some(Err(SharedRegistryError::Base(e))),
        };
        if extra.is_empty() {
            return Some(Ok(base));
        }
        Some(
            Self::with_extra(base, extra)
                .map(Arc::new)
                .map_err(SharedRegistryError::Extra),
        )
    }

    fn shared_or_else(
        dir: Option<String>,
        lookup_env: impl FnOnce(&str) -> Option<String>,
    ) -> Option<Result<Arc<Self>>> {
        dir.or_else(|| lookup_env(DEFINITIONS_DIR_ENV))
            .map(Self::shared)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::path::PathBuf;
    use std::sync::Arc;

    use crate::Registry;

    type TestResult = Result<(), Box<dyn Error>>;

    fn definitions_dir() -> String {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../definitions/r5")
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn explicit_dir_wins_over_fallback() -> TestResult {
        let fallback = |_: &str| Some("/nonexistent-fallback".to_owned());
        Registry::shared_or_else(Some(definitions_dir()), fallback).ok_or("dir given")??;
        Ok(())
    }

    #[test]
    fn fallback_is_used_without_explicit_dir() -> TestResult {
        let from_fallback = Registry::shared_or_else(None, |_| Some(definitions_dir()))
            .ok_or("fallback given")??;
        let explicit = Registry::shared(definitions_dir())?;
        assert!(Arc::ptr_eq(&from_fallback, &explicit));
        Ok(())
    }

    #[test]
    fn fallback_looks_up_definitions_dir_env() {
        let mut asked = None;
        let _ = Registry::shared_or_else(None, |key| {
            asked = Some(key.to_owned());
            None
        });
        assert_eq!(asked.as_deref(), Some("FHIR_DEFINITIONS_DIR"));
    }

    #[test]
    fn no_dir_and_no_fallback_is_none() {
        assert!(Registry::shared_or_else(None, |_| None).is_none());
    }

    #[test]
    fn with_extra_without_paths_is_the_shared_base() -> TestResult {
        let none: &[PathBuf] = &[];
        let registry = Registry::shared_or_else_with_extra(Some(definitions_dir()), none, |_| None)
            .ok_or("dir given")??;
        assert!(Arc::ptr_eq(
            &registry,
            &Registry::shared(definitions_dir())?
        ));
        Ok(())
    }

    #[test]
    fn with_extra_layers_a_supplied_file() -> TestResult {
        let url = "http://example.org/ValueSet/shared-or-env-extra";
        let file = std::env::temp_dir().join(format!("{}_shared_extra.json", std::process::id()));
        let value_set = format!(r#"{{"resourceType":"ValueSet","url":"{url}","name":"Extra"}}"#);
        std::fs::write(&file, value_set)?;
        let registry =
            Registry::shared_or_else_with_extra(Some(definitions_dir()), &[&file], |_| None);
        std::fs::remove_file(&file)?;
        let registry = registry.ok_or("dir given")??;
        let base = Registry::shared(definitions_dir())?;
        assert!(registry.value_set(url).is_some());
        assert!(!Arc::ptr_eq(&registry, &base));
        assert!(
            base.value_set(url).is_none(),
            "the shared base is never mutated"
        );
        Ok(())
    }

    #[test]
    fn with_extra_bad_path_is_an_extra_definitions_error() -> TestResult {
        let registry = Registry::shared_or_else_with_extra(
            Some(definitions_dir()),
            &["/nonexistent-extra.json"],
            |_| None,
        )
        .ok_or("dir given")?;
        let message = registry.err().ok_or("bad path must fail")?.to_string();
        assert!(
            message.starts_with("cannot load extra definitions: "),
            "{message}"
        );
        Ok(())
    }

    #[test]
    fn with_extra_bad_base_is_a_definitions_error() -> TestResult {
        let explicit = Some("/nonexistent-explicit".to_owned());
        let none: &[PathBuf] = &[];
        let registry =
            Registry::shared_or_else_with_extra(explicit, none, |_| None).ok_or("dir given")?;
        let message = registry.err().ok_or("bad base must fail")?.to_string();
        assert!(
            message.starts_with("cannot load definitions: "),
            "{message}"
        );
        Ok(())
    }

    #[test]
    fn with_extra_without_dir_is_none() {
        let none: &[PathBuf] = &[];
        assert!(Registry::shared_or_else_with_extra(None, none, |_| None).is_none());
    }

    #[test]
    fn unloadable_dir_is_an_error() -> TestResult {
        let explicit = Some("/nonexistent-explicit".to_owned());
        let registry = Registry::shared_or_else(explicit, |_| None).ok_or("dir given")?;
        assert!(registry.is_err());
        Ok(())
    }
}
