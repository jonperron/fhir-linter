use std::str::FromStr;
use std::sync::Arc;

use fhir_parser::Span;

/// Diagnostic severity level: the FHIR R5 IssueSeverity codes, most severe
/// first. `success` is not a diagnostic level and has no variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Fatal,
    Error,
    Warning,
    Information,
}

impl Severity {
    /// Every variant, most severe first.
    pub const ALL: [Self; 4] = [Self::Fatal, Self::Error, Self::Warning, Self::Information];

    /// IssueSeverity code, as exposed by the bindings: `fatal`, `error`,
    /// `warning`, `information`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fatal => "fatal",
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Information => "information",
        }
    }
}

impl FromStr for Severity {
    type Err = ();

    /// Parses an IssueSeverity code exactly as `as_str` writes it.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL.into_iter().find(|v| v.as_str() == s).ok_or(())
    }
}

/// A validation finding emitted by the validator or linter.
///
/// `Diagnostic` is the single output type across all crates.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    /// Machine-readable code, e.g. `"CARDINALITY_001"`.
    pub code: Arc<str>,
    pub message: String,
    /// FHIRPath to the offending node, e.g. `"Patient.name[0].given"`.
    pub path: String,
    /// Source location, when available.
    pub location: Option<Span>,
    /// Lint rule identifier, if this diagnostic originated from a lint rule.
    pub rule: Option<Arc<str>>,
}

impl Diagnostic {
    pub fn error(code: &str, message: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            code: Arc::from(code),
            message: message.into(),
            path: path.into(),
            location: None,
            rule: None,
        }
    }

    pub fn warning(code: &str, message: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            code: Arc::from(code),
            message: message.into(),
            path: path.into(),
            location: None,
            rule: None,
        }
    }

    pub fn with_location(mut self, location: Span) -> Self {
        self.location = Some(location);
        self
    }

    pub fn with_rule(mut self, rule: impl Into<Arc<str>>) -> Self {
        self.rule = Some(rule.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_sets_severity_and_code() {
        let d = Diagnostic::error("TEST_001", "test message", "Patient.foo");
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(d.code.as_ref(), "TEST_001");
        assert_eq!(d.message, "test message");
        assert_eq!(d.path, "Patient.foo");
        assert!(d.location.is_none());
        assert!(d.rule.is_none());
    }

    #[test]
    fn warning_sets_severity() {
        let d = Diagnostic::warning("TEST_002", "warn", "Patient.bar");
        assert_eq!(d.severity, Severity::Warning);
    }

    #[test]
    fn with_location_attaches_span() {
        let span = Span {
            line: 3,
            col: 5,
            offset: 42,
        };
        let d = Diagnostic::error("X", "msg", "p").with_location(span);
        assert_eq!(d.location, Some(span));
    }

    #[test]
    fn with_rule_attaches_rule_id() {
        let d = Diagnostic::warning("X", "msg", "p").with_rule("my-rule");
        assert_eq!(d.rule.as_deref(), Some("my-rule"));
    }

    #[test]
    fn severity_ordering_most_severe_first() {
        assert!(Severity::Fatal < Severity::Error);
        assert!(Severity::Error < Severity::Warning);
        assert!(Severity::Warning < Severity::Information);
        assert!(Severity::ALL.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn severity_as_str_is_issue_severity_code() {
        assert_eq!(Severity::Fatal.as_str(), "fatal");
        assert_eq!(Severity::Error.as_str(), "error");
        assert_eq!(Severity::Warning.as_str(), "warning");
        assert_eq!(Severity::Information.as_str(), "information");
    }

    #[test]
    fn severity_from_str_round_trips_as_str() {
        for s in Severity::ALL {
            assert_eq!(s.as_str().parse::<Severity>(), Ok(s));
        }
    }

    #[test]
    fn severity_from_str_rejects_other_codes() {
        for code in ["success", "hint", "Error", ""] {
            assert!(code.parse::<Severity>().is_err(), "{code}");
        }
    }
}
