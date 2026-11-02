//! Python bindings for the FHIR linter and validator.
//!
//! Exposes `validate()` and `lint()`, both returning a list of `Diagnostic`.
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use fhir_definitions::Registry;
use fhir_linter::{Config, RuleRegistry};
use fhir_validator::Severity;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

const DEFINITIONS_DIR_ENV: &str = "FHIR_DEFINITIONS_DIR";

/// A validation or lint finding.
#[pyclass(frozen, get_all, module = "fhir_linter")]
struct Diagnostic {
    /// `error`, `warning`, `information` or `hint`.
    severity: &'static str,
    code: String,
    message: String,
    path: String,
    line: Option<u32>,
    col: Option<u32>,
    offset: Option<u32>,
    /// Lint rule id; `None` for validator findings.
    rule: Option<String>,
}

#[pymethods]
impl Diagnostic {
    fn __repr__(&self) -> String {
        format!(
            "Diagnostic(severity={:?}, code={:?}, path={:?}, message={:?})",
            self.severity, self.code, self.path, self.message
        )
    }
}

impl From<fhir_validator::Diagnostic> for Diagnostic {
    fn from(d: fhir_validator::Diagnostic) -> Self {
        Self {
            severity: match d.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Information => "information",
                Severity::Hint => "hint",
            },
            code: d.code.to_string(),
            message: d.message,
            path: d.path,
            line: d.location.map(|s| s.line),
            col: d.location.map(|s| s.col),
            offset: d.location.map(|s| s.offset),
            rule: d.rule.map(|r| r.to_string()),
        }
    }
}

/// Registries are expensive to build (~60 MB of JSON), so build one per directory.
fn registry_for(definitions_dir: Option<String>) -> PyResult<Arc<Registry>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Arc<Registry>>>> = OnceLock::new();

    let dir = definitions_dir
        .or_else(|| env::var(DEFINITIONS_DIR_ENV).ok())
        .map(PathBuf::from)
        .ok_or_else(|| {
            PyValueError::new_err(format!(
                "no FHIR definitions: pass definitions_dir or set {DEFINITIONS_DIR_ENV}"
            ))
        })?;

    let mut cache = CACHE
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(registry) = cache.get(&dir) {
        return Ok(Arc::clone(registry));
    }
    let registry = Arc::new(
        Registry::from_definitions_dir(&dir)
            .map_err(|e| PyRuntimeError::new_err(format!("cannot load definitions: {e}")))?,
    );
    cache.insert(dir, Arc::clone(&registry));
    Ok(registry)
}

fn parse(json: &str) -> PyResult<fhir_parser::Resource> {
    fhir_parser::parse_json(json).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Validate a FHIR R5 JSON resource against the FHIR definitions.
///
/// Raises `ValueError` when the JSON cannot be parsed or no definitions are found.
#[pyfunction]
#[pyo3(signature = (json, definitions_dir=None))]
fn validate(json: &str, definitions_dir: Option<String>) -> PyResult<Vec<Diagnostic>> {
    let resource = parse(json)?;
    let registry = registry_for(definitions_dir)?;
    Ok(fhir_validator::validate(&resource, &registry)
        .into_iter()
        .map(Diagnostic::from)
        .collect())
}

/// Run the lint rules on a FHIR R5 JSON resource.
///
/// `config_toml` is the content of a `fhir-linter.toml` file. Raises `ValueError`
/// on unparsable JSON, invalid configuration or missing definitions.
#[pyfunction]
#[pyo3(signature = (json, config_toml=None, definitions_dir=None))]
fn lint(
    json: &str,
    config_toml: Option<&str>,
    definitions_dir: Option<String>,
) -> PyResult<Vec<Diagnostic>> {
    let config = config_toml
        .map(Config::from_toml)
        .transpose()
        .map_err(|e| PyValueError::new_err(format!("invalid lint config: {e}")))?
        .unwrap_or_default();
    let resource = parse(json)?;
    let registry = registry_for(definitions_dir)?;
    let diagnostics = RuleRegistry::default()
        .run(&resource, &registry, &config)
        .map_err(PyValueError::new_err)?;
    Ok(diagnostics.into_iter().map(Diagnostic::from).collect())
}

#[pymodule]
#[pyo3(name = "fhir_linter")]
fn fhir_python(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Diagnostic>()?;
    m.add_function(wrap_pyfunction!(validate, m)?)?;
    m.add_function(wrap_pyfunction!(lint, m)?)?;
    Ok(())
}
