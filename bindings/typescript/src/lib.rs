//! Node.js bindings for the FHIR linter and validator.
//!
//! Exposes async `validate()` and `lint()`, both resolving to an array of `Diagnostic`.
use std::env;
use std::sync::Arc;

use fhir_definitions::Registry;
use fhir_linter::{Config, RuleRegistry};
use napi::tokio::task::spawn_blocking;
use napi::{Error, Result, Status};
use napi_derive::napi;

const DEFINITIONS_DIR_ENV: &str = "FHIR_DEFINITIONS_DIR";

/// A validation or lint finding.
#[napi(object)]
pub struct Diagnostic {
    /// `error`, `warning`, `information` or `hint`.
    pub severity: String,
    pub code: String,
    pub message: String,
    pub path: String,
    pub line: Option<u32>,
    pub col: Option<u32>,
    pub offset: Option<u32>,
    /// Lint rule id; absent for validator findings.
    pub rule: Option<String>,
}

impl From<fhir_validator::Diagnostic> for Diagnostic {
    fn from(d: fhir_validator::Diagnostic) -> Self {
        Self {
            severity: d.severity.as_str().to_owned(),
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

fn invalid_argument(message: impl Into<String>) -> Error {
    Error::new(Status::InvalidArg, message.into())
}

/// Resolve the definitions directory and return its shared registry.
fn registry_for(definitions_dir: Option<String>) -> Result<Arc<Registry>> {
    let dir = definitions_dir
        .or_else(|| env::var(DEFINITIONS_DIR_ENV).ok())
        .ok_or_else(|| {
            invalid_argument(format!(
                "no FHIR definitions: pass definitionsDir or set {DEFINITIONS_DIR_ENV}"
            ))
        })?;
    Registry::shared(dir).map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("cannot load definitions: {e}"),
        )
    })
}

fn parse(json: &str) -> Result<fhir_parser::Resource> {
    fhir_parser::parse_json(json).map_err(|e| invalid_argument(e.to_string()))
}

/// Run blocking work off the event loop; a panic becomes a rejected promise.
async fn off_event_loop<T: Send + 'static>(
    work: impl FnOnce() -> Result<T> + Send + 'static,
) -> Result<T> {
    spawn_blocking(work)
        .await
        .map_err(|e| Error::new(Status::GenericFailure, format!("task failed: {e}")))?
}

/// Validate a FHIR R5 JSON resource against the FHIR definitions.
///
/// Rejects when the JSON cannot be parsed or no definitions are found.
#[napi]
pub async fn validate(json: String, definitions_dir: Option<String>) -> Result<Vec<Diagnostic>> {
    off_event_loop(move || {
        let resource = parse(&json)?;
        let registry = registry_for(definitions_dir)?;
        Ok(fhir_validator::validate(&resource, &registry)
            .into_iter()
            .map(Diagnostic::from)
            .collect())
    })
    .await
}

/// Run the lint rules on a FHIR R5 JSON resource.
///
/// `configToml` is the content of a `fhir-linter.toml` file. Rejects on unparsable JSON,
/// invalid configuration or missing definitions.
#[napi]
pub async fn lint(
    json: String,
    config_toml: Option<String>,
    definitions_dir: Option<String>,
) -> Result<Vec<Diagnostic>> {
    off_event_loop(move || {
        let config = config_toml
            .as_deref()
            .map(Config::from_toml)
            .transpose()
            .map_err(|e| invalid_argument(format!("invalid lint config: {e}")))?
            .unwrap_or_default();
        let resource = parse(&json)?;
        let registry = registry_for(definitions_dir)?;
        let diagnostics = RuleRegistry::default()
            .run(&resource, &registry, &config)
            .map_err(invalid_argument)?;
        Ok(diagnostics.into_iter().map(Diagnostic::from).collect())
    })
    .await
}
