use std::collections::HashMap;

use serde::Deserialize;

/// Per-rule settings from `fhir-linter.toml`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleConfig {
    /// `false` disables the rule. Defaults to enabled.
    pub enabled: Option<bool>,
    /// Overrides the rule's default severity: `error`, `warning`, `information` or `hint`.
    pub severity: Option<String>,
}

/// Linter configuration, deserialized from TOML:
///
/// ```toml
/// [rules.narrative-absent]
/// severity = "error"
///
/// [rules.missing-identifier]
/// enabled = false
/// ```
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub rules: HashMap<String, RuleConfig>,
}

impl Config {
    pub fn from_toml(text: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(text)
    }
}
