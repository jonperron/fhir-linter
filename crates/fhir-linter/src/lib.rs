#![forbid(unsafe_code)]
//! Configurable FHIR lint rules.
//!
//! Implements the `Rule` trait and a `RuleRegistry` for running user-configured
//! checks on top of the core validator output.

mod config;
mod rules;

pub use config::{Config, RuleConfig};
pub use rules::{MissingIdentifier, MissingMetaProfile, NarrativeAbsent};

use fhir_definitions::Registry;
use fhir_parser::Resource;
use fhir_validator::{Diagnostic, Severity};

/// A single lint check. Rules emit diagnostics at their default severity;
/// the registry applies configuration and tags them with the rule id.
pub trait Rule {
    /// Stable kebab-case identifier used in `fhir-linter.toml`.
    fn id(&self) -> &'static str;
    fn check(&self, resource: &Resource, registry: &Registry) -> Vec<Diagnostic>;
}

/// A set of rules that can be run against a resource.
pub struct RuleRegistry {
    rules: Vec<Box<dyn Rule>>,
}

impl Default for RuleRegistry {
    /// All built-in rules.
    fn default() -> Self {
        Self {
            rules: vec![
                Box::new(NarrativeAbsent),
                Box::new(MissingIdentifier),
                Box::new(MissingMetaProfile),
            ],
        }
    }
}

impl RuleRegistry {
    /// Run every enabled rule; severities are overridden per `config`.
    ///
    /// Returns an error naming the rule when `config` holds an unknown rule id
    /// or severity, so typos in `fhir-linter.toml` are not silently ignored.
    pub fn run(
        &self,
        resource: &Resource,
        registry: &Registry,
        config: &Config,
    ) -> Result<Vec<Diagnostic>, String> {
        if let Some(id) = config
            .rules
            .keys()
            .find(|id| !self.rules.iter().any(|r| r.id() == id.as_str()))
        {
            return Err(format!("unknown lint rule '{id}'"));
        }
        let mut out = Vec::new();
        for rule in &self.rules {
            let cfg = config.rules.get(rule.id());
            let severity = cfg
                .and_then(|c| c.severity.as_deref())
                .map(|s| {
                    parse_severity(s)
                        .ok_or(format!("invalid severity '{s}' for rule '{}'", rule.id()))
                })
                .transpose()?;
            if cfg.and_then(|c| c.enabled) == Some(false) {
                continue;
            }
            for mut d in rule.check(resource, registry) {
                if let Some(s) = severity {
                    d.severity = s;
                }
                out.push(d.with_rule(rule.id()));
            }
        }
        Ok(out)
    }
}

/// An IssueSeverity code, or `hint`: a config alias kept from earlier
/// releases and output as `information`.
fn parse_severity(s: &str) -> Option<Severity> {
    match s {
        "hint" => Some(Severity::Information),
        _ => s.parse().ok(),
    }
}
