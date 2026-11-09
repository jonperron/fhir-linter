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
    /// Check `config` against the registered rules without running them.
    ///
    /// Returns an error naming the rule when `config` holds an unknown rule id
    /// or severity, so typos in `fhir-linter.toml` are not silently ignored.
    pub fn check_config(&self, config: &Config) -> Result<(), String> {
        for (id, rule_config) in &config.rules {
            if !self.rules.iter().any(|r| r.id() == id.as_str()) {
                return Err(format!("unknown lint rule '{id}'"));
            }
            if let Some(s) = rule_config.severity.as_deref() {
                parse_severity(s).ok_or(format!("invalid severity '{s}' for rule '{id}'"))?;
            }
        }
        Ok(())
    }

    /// Run every enabled rule; severities are overridden per `config`.
    ///
    /// Fails like [`RuleRegistry::check_config`] on an invalid `config`.
    pub fn run(
        &self,
        resource: &Resource,
        registry: &Registry,
        config: &Config,
    ) -> Result<Vec<Diagnostic>, String> {
        self.check_config(config)?;
        let mut out = Vec::new();
        for rule in &self.rules {
            let cfg = config.rules.get(rule.id());
            // Already validated by `check_config`.
            let severity = cfg
                .and_then(|c| c.severity.as_deref())
                .and_then(parse_severity);
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
