//! Locating the definitions directory for callers without a CLI (the language bindings).

use std::env;
use std::sync::Arc;

use crate::{Registry, Result};

/// Environment variable naming the definitions directory when none is passed.
pub const DEFINITIONS_DIR_ENV: &str = "FHIR_DEFINITIONS_DIR";

impl Registry {
    /// The shared registry for `dir`, or for [`DEFINITIONS_DIR_ENV`] when `dir` is `None`.
    ///
    /// Returns `None` when neither names a directory (an unset or non-UTF-8 variable),
    /// so each caller can word that error itself. Loading goes through [`Registry::shared`].
    pub fn shared_or_env(dir: Option<String>) -> Option<Result<Arc<Self>>> {
        Self::shared_or_else(dir, || env::var(DEFINITIONS_DIR_ENV).ok())
    }

    fn shared_or_else(
        dir: Option<String>,
        fallback: impl FnOnce() -> Option<String>,
    ) -> Option<Result<Arc<Self>>> {
        dir.or_else(fallback).map(Self::shared)
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
        let fallback = || Some("/nonexistent-fallback".to_owned());
        Registry::shared_or_else(Some(definitions_dir()), fallback).ok_or("dir given")??;
        Ok(())
    }

    #[test]
    fn fallback_is_used_without_explicit_dir() -> TestResult {
        let from_fallback = Registry::shared_or_else(None, || Some(definitions_dir()))
            .ok_or("fallback given")??;
        let explicit = Registry::shared(definitions_dir())?;
        assert!(Arc::ptr_eq(&from_fallback, &explicit));
        Ok(())
    }

    #[test]
    fn no_dir_and_no_fallback_is_none() {
        assert!(Registry::shared_or_else(None, || None).is_none());
    }

    #[test]
    fn unloadable_dir_is_an_error() -> TestResult {
        let explicit = Some("/nonexistent-explicit".to_owned());
        let registry = Registry::shared_or_else(explicit, || None).ok_or("dir given")?;
        assert!(registry.is_err());
        Ok(())
    }
}
