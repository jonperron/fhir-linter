//! Python bindings for the FHIR linter and validator.
//!
//! Exposes `validate()` and `lint()`, both returning a list of `Diagnostic`.
use std::env;
use std::sync::Arc;

use fhir_definitions::Registry;
use fhir_linter::{Config, RuleRegistry};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

const DEFINITIONS_DIR_ENV: &str = "FHIR_DEFINITIONS_DIR";

/// A validation or lint finding.
#[pyclass(frozen, get_all, module = "fhir_linter")]
struct Diagnostic {
    /// FHIR IssueSeverity code: `fatal`, `error`, `warning` or `information`.
    severity: &'static str,
    code: String,
    message: String,
    path: String,
    /// 1-indexed line.
    line: Option<u32>,
    /// 1-indexed UTF-8 byte column, not a character column.
    col: Option<u32>,
    /// 0-indexed UTF-8 byte offset into the input, not a `str` index.
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
            severity: d.severity.as_str(),
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

/// Resolve the definitions directory and return its shared registry.
fn registry_for(definitions_dir: Option<String>) -> PyResult<Arc<Registry>> {
    let dir = definitions_dir
        .or_else(|| env::var(DEFINITIONS_DIR_ENV).ok())
        .ok_or_else(|| {
            PyValueError::new_err(format!(
                "no FHIR definitions: pass definitions_dir or set {DEFINITIONS_DIR_ENV}"
            ))
        })?;
    Registry::shared(dir)
        .map_err(|e| PyRuntimeError::new_err(format!("cannot load definitions: {e}")))
}

fn parse(json: &str) -> PyResult<fhir_parser::Resource> {
    fhir_parser::parse_json(json).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Validate a FHIR R5 JSON resource against the FHIR definitions.
///
/// Raises `ValueError` when the JSON cannot be parsed or no definitions are found.
#[pyfunction]
#[pyo3(signature = (json, definitions_dir=None))]
fn validate(
    py: Python<'_>,
    json: &str,
    definitions_dir: Option<String>,
) -> PyResult<Vec<Diagnostic>> {
    // Release the GIL: loading the registry takes seconds.
    py.detach(|| {
        let resource = parse(json)?;
        let registry = registry_for(definitions_dir)?;
        Ok(fhir_validator::validate(&resource, &registry)
            .into_iter()
            .map(Diagnostic::from)
            .collect())
    })
}

/// Run the lint rules on a FHIR R5 JSON resource.
///
/// `config_toml` is the content of a `fhir-linter.toml` file. Raises `ValueError`
/// on unparsable JSON, invalid configuration or missing definitions.
#[pyfunction]
#[pyo3(signature = (json, config_toml=None, definitions_dir=None))]
fn lint(
    py: Python<'_>,
    json: &str,
    config_toml: Option<&str>,
    definitions_dir: Option<String>,
) -> PyResult<Vec<Diagnostic>> {
    let config = config_toml
        .map(Config::from_toml)
        .transpose()
        .map_err(|e| PyValueError::new_err(format!("invalid lint config: {e}")))?
        .unwrap_or_default();
    py.detach(|| {
        let resource = parse(json)?;
        let registry = registry_for(definitions_dir)?;
        let diagnostics = RuleRegistry::default()
            .run(&resource, &registry, &config)
            .map_err(PyValueError::new_err)?;
        Ok(diagnostics.into_iter().map(Diagnostic::from).collect())
    })
}

#[pymodule]
#[pyo3(name = "fhir_linter")]
fn fhir_python(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Diagnostic>()?;
    m.add_function(wrap_pyfunction!(validate, m)?)?;
    m.add_function(wrap_pyfunction!(lint, m)?)?;
    Ok(())
}
