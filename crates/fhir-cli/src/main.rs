#![forbid(unsafe_code)]
//! `fhir-lint`: validate and lint FHIR R5 resources from the command line.
//!
//! Exit codes: 0 = clean, 1 = validation or lint errors (warnings too with
//! `--strict`), 2 = tool error (bad arguments, unreadable input, bad config
//! or definitions).

mod files;
mod output;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, ValueEnum};
use fhir_definitions::Registry;
use fhir_linter::{Config, RuleRegistry};
use fhir_parser::{ParseError, Resource, Span};
use fhir_validator::{Diagnostic, Severity};

use crate::output::Finding;

#[derive(Debug, Parser)]
#[command(name = "fhir-lint", version, about)]
struct Cli {
    /// FHIR resource files, or directories scanned recursively for `*.json` and `*.xml`.
    #[arg(required = true)]
    paths: Vec<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
    /// Path to a `fhir-linter.toml` rule configuration.
    #[arg(long)]
    config: Option<PathBuf>,
    /// FHIR version of the inputs.
    #[arg(long, value_enum, default_value_t = FhirVersion::R5)]
    fhir_version: FhirVersion,
    /// Directory holding the FHIR definitions (`profiles-resources.json`, ...).
    #[arg(long, env = "FHIR_DEFINITIONS_DIR", default_value = "definitions/r5")]
    definitions: PathBuf,
    /// Treat warnings as errors for the exit code.
    #[arg(long)]
    strict: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Json,
    Sarif,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FhirVersion {
    R5,
}

fn main() -> ExitCode {
    match run(&Cli::parse()) {
        Ok(failed) => ExitCode::from(u8::from(failed)),
        Err(message) => {
            eprintln!("fhir-lint: {message}");
            ExitCode::from(2)
        }
    }
}

/// Lint every input and print the report. Returns whether the run failed.
fn run(cli: &Cli) -> Result<bool, String> {
    // Only R5 exists; adding a version makes this a compile error until it is handled.
    let FhirVersion::R5 = cli.fhir_version;
    let rules = RuleRegistry::default();
    let config = load_config(cli.config.as_deref())?;
    // Checked before reading inputs so config errors are never hidden by path errors.
    rules
        .check_config(&config)
        .map_err(|e| format!("invalid config: {e}"))?;
    let inputs = files::collect(&cli.paths)?;
    let registry = Registry::from_definitions_dir(&cli.definitions).map_err(|e| {
        format!(
            "cannot load definitions from '{}': {e}",
            cli.definitions.display()
        )
    })?;

    let mut findings = Vec::new();
    for path in &inputs {
        let file = path.display().to_string();
        for diagnostic in lint_file(path, &rules, &registry, &config)? {
            findings.push(Finding {
                file: file.clone(),
                diagnostic,
            });
        }
    }

    let report = output::render(&findings, cli.format);
    if !report.is_empty() {
        match writeln!(std::io::stdout().lock(), "{report}") {
            // The reader went away (`| head`): the verdict is still the exit code.
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
            result => result.map_err(|e| format!("cannot write report: {e}"))?,
        }
    }
    Ok(findings.iter().any(|f| match f.diagnostic.severity {
        Severity::Fatal | Severity::Error => true,
        Severity::Warning => cli.strict,
        Severity::Information => false,
    }))
}

fn load_config(path: Option<&Path>) -> Result<Config, String> {
    let Some(path) = path else {
        return Ok(Config::default());
    };
    let text = fs::read_to_string(path)
        .map_err(|e| format!("cannot read config '{}': {e}", path.display()))?;
    Config::from_toml(&text).map_err(|e| format!("invalid config '{}': {e}", path.display()))
}

/// Validate and lint one file. A resource that does not parse yields a
/// `PARSE_001` diagnostic; only an unreadable file is a tool error.
fn lint_file(
    path: &Path,
    rules: &RuleRegistry,
    registry: &Registry,
    config: &Config,
) -> Result<Vec<Diagnostic>, String> {
    let bytes = fs::read(path).map_err(|e| format!("cannot read '{}': {e}", path.display()))?;
    let resource = match String::from_utf8(bytes) {
        Ok(text) => parse(path, &text),
        Err(e) => Err(parse_diagnostic(
            format!("file is not valid UTF-8: {e}"),
            None,
        )),
    };
    let resource = match resource {
        Ok(resource) => resource,
        Err(diagnostic) => return Ok(vec![diagnostic]),
    };
    let mut diagnostics = fhir_validator::validate(&resource, registry);
    diagnostics.extend(rules.run(&resource, registry, config)?);
    Ok(diagnostics)
}

fn parse(path: &Path, text: &str) -> Result<Resource, Diagnostic> {
    let result = if files::is_xml(path) {
        fhir_parser::parse_xml(text)
    } else {
        fhir_parser::parse_json(text)
    };
    result.map_err(|e| {
        let location = match &e {
            ParseError::JsonError { line, col, .. } | ParseError::XmlError { line, col, .. } => {
                // ParseError carries no byte offset; reports only print line and column.
                Some(Span {
                    line: *line,
                    col: *col,
                    offset: 0,
                })
            }
            ParseError::MissingResourceType => None,
        };
        parse_diagnostic(e.to_string(), location)
    })
}

fn parse_diagnostic(message: String, location: Option<Span>) -> Diagnostic {
    let diagnostic = Diagnostic::error("PARSE_001", message, "");
    match location {
        Some(span) => diagnostic.with_location(span),
        None => diagnostic,
    }
}
