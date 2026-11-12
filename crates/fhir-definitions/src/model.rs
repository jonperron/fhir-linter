use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};

use fhir_fhirpath::{Expr, ParseError};

/// A lightweight StructureDefinition — only the fields the validator needs.
///
/// Built with [`StructureDefinition::new`], which indexes the snapshot by path;
/// the snapshot must not change afterwards (the registry only hands out `&` references).
#[derive(Debug, Clone)]
pub struct StructureDefinition {
    pub url: Arc<str>,
    pub name: Arc<str>,
    pub kind: Arc<str>,
    pub is_abstract: bool,
    pub snapshot: Vec<ElementDefinition>,
    /// Position in `snapshot` of the first element with each path.
    path_index: HashMap<Arc<str>, usize>,
}

impl StructureDefinition {
    pub fn new(
        url: Arc<str>,
        name: Arc<str>,
        kind: Arc<str>,
        is_abstract: bool,
        snapshot: Vec<ElementDefinition>,
    ) -> Self {
        let mut path_index = HashMap::with_capacity(snapshot.len());
        for (i, elem) in snapshot.iter().enumerate() {
            // Slices repeat their path; keep the first, as a linear `find` would.
            path_index.entry(elem.path.clone()).or_insert(i);
        }
        Self {
            url,
            name,
            kind,
            is_abstract,
            snapshot,
            path_index,
        }
    }

    /// The (first) snapshot element whose path is exactly `path`.
    pub fn element(&self, path: &str) -> Option<&ElementDefinition> {
        self.path_index
            .get(path)
            .and_then(|&i| self.snapshot.get(i))
    }

    /// The snapshot elements below `path`: the run right after its element whose
    /// paths start with `path.` (snapshots list descendants right after their parent).
    /// Empty when `path` is not in the snapshot.
    pub fn descendants(&self, path: &str) -> &[ElementDefinition] {
        let Some(&start) = self.path_index.get(path) else {
            return &[];
        };
        let rest = self.snapshot.get(start + 1..).unwrap_or_default();
        let len = rest
            .iter()
            .position(|e| {
                !e.path
                    .strip_prefix(path)
                    .is_some_and(|tail| tail.starts_with('.') || tail.is_empty())
            })
            .unwrap_or(rest.len());
        &rest[..len]
    }
}

/// A single element in a StructureDefinition snapshot.
#[derive(Debug, Clone)]
pub struct ElementDefinition {
    pub path: Arc<str>,
    pub min: u32,
    /// Either `"*"` or a non-negative integer represented as a string.
    pub max: Arc<str>,
    pub types: Vec<Arc<str>>,
    /// FHIRPath constraint expressions defined on this element.
    pub constraints: Vec<Constraint>,
    /// Terminology binding, if any.
    pub binding: Option<Binding>,
    /// `sliceName` when this element is a slice of a repeating element, not the element itself.
    pub slice_name: Option<Arc<str>>,
    /// `representation` holds `xmlAttr`: the XML form is an attribute, not a child element.
    pub xml_attr: bool,
    /// `contentReference`: the element whose content rules apply here. A local
    /// `#Questionnaire.item` is kept as the bare path `Questionnaire.item`; a reference into
    /// another definition (`http://...#Observation.referenceRange`) is kept whole.
    pub content_reference: Option<Arc<str>>,
    /// Profiles and target profiles declared on the element's types, one entry per type code
    /// that has any (type codes are unique per element, eld-13). Other types have no entry.
    pub type_profiles: Vec<TypeProfiles>,
    /// `fixed[x]`: the value SHALL be exactly this one.
    pub fixed: Option<Box<ChoiceValue>>,
    /// `pattern[x]`: the value SHALL contain at least these parts.
    pub pattern: Option<Box<ChoiceValue>>,
}

/// A `[x]` property of an ElementDefinition (`fixedCode`, `patternCodeableConcept`, ...),
/// kept raw: its key names the type, its value is the JSON as written in the definition.
#[derive(Debug, Clone)]
pub struct ChoiceValue {
    pub key: Arc<str>,
    pub value: serde_json::Value,
}

/// The profiles an `ElementDefinition.type` entry declares (canonical URLs, version stripped).
#[derive(Debug, Clone)]
pub struct TypeProfiles {
    /// The type code, resolved like the entries of `ElementDefinition::types`.
    pub code: Arc<str>,
    /// `type.profile`: the value must conform to at least one of these.
    pub profiles: Vec<Arc<str>>,
    /// `type.targetProfile`: a `Reference`/`canonical` must point to one of these.
    pub target_profiles: Vec<Arc<str>>,
}

/// A FHIRPath constraint expression on an element.
///
/// The expression is parsed at most once, on first use, and the result (or the
/// parse error) is cached in a [`OnceLock`] so a shared `Registry` stays `Send + Sync`.
#[derive(Debug, Clone)]
pub struct Constraint {
    pub key: Arc<str>,
    /// Either `"error"` or `"warning"`.
    pub severity: Arc<str>,
    pub human: Arc<str>,
    /// FHIRPath expression; absent for extension-only constraints.
    pub expression: Option<Arc<str>>,
    parsed: OnceLock<Result<Expr, ParseError>>,
}

impl Constraint {
    #[must_use]
    pub fn new(
        key: Arc<str>,
        severity: Arc<str>,
        human: Arc<str>,
        expression: Option<Arc<str>>,
    ) -> Self {
        Self {
            key,
            severity,
            human,
            expression,
            parsed: OnceLock::new(),
        }
    }

    /// The parsed expression, parsed on the first call only; `None` when absent.
    pub fn parsed_expression(&self) -> Option<&Result<Expr, ParseError>> {
        let expression = self.expression.as_deref()?;
        Some(self.parsed.get_or_init(|| fhir_fhirpath::parse(expression)))
    }
}

/// A terminology binding on an element.
#[derive(Debug, Clone)]
pub struct Binding {
    pub strength: Arc<str>,
    pub value_set: Option<Arc<str>>,
}

/// A lightweight ValueSet: identity plus the `compose.include` entries needed
/// for membership checks.
#[derive(Debug, Clone)]
pub struct ValueSet {
    pub url: Arc<str>,
    pub name: Arc<str>,
    pub includes: Vec<Include>,
    /// True when the compose has `exclude` entries; membership is then not decidable.
    pub has_exclude: bool,
}

/// One `compose.include` entry of a ValueSet.
#[derive(Debug, Clone, Default)]
pub struct Include {
    pub system: Option<Arc<str>>,
    /// CodeSystem version the include is pinned to (`include.version`), if any.
    pub version: Option<Arc<str>>,
    /// Explicitly enumerated codes; empty means "the whole system" (unless `has_filter`).
    pub codes: Vec<Arc<str>>,
    /// True when a `filter` restricts the system; membership is then not decidable.
    pub has_filter: bool,
    /// Canonical URLs of ValueSets this include is intersected with.
    pub value_sets: Vec<Arc<str>>,
}

/// A lightweight CodeSystem with its flattened concept codes.
#[derive(Debug, Clone)]
pub struct CodeSystem {
    pub url: Arc<str>,
    pub name: Arc<str>,
    /// Business version (`CodeSystem.version`), if any.
    pub version: Option<Arc<str>>,
    pub codes: HashSet<Arc<str>>,
    /// Codes of concepts flagged notSelectable (R5 concept property); a subset of `codes`.
    pub not_selectable: HashSet<Arc<str>>,
    /// False when the system is a fragment/not-present, so absence of a code proves nothing.
    pub complete: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn elem(path: &str, min: u32) -> ElementDefinition {
        ElementDefinition {
            path: Arc::from(path),
            min,
            max: Arc::from("*"),
            types: vec![],
            constraints: vec![],
            binding: None,
            slice_name: None,
            xml_attr: false,
            content_reference: None,
            type_profiles: vec![],
            fixed: None,
            pattern: None,
        }
    }

    fn sliced_observation() -> StructureDefinition {
        let paths = [
            ("Observation", 0),
            ("Observation.category", 0),
            ("Observation.category.coding", 0),
            ("Observation.category", 1),
            ("Observation.category.coding", 1),
            ("Observation.categoryText", 0),
            ("Observation.code", 1),
        ];
        StructureDefinition::new(
            Arc::from("http://example.org/sd"),
            Arc::from("Obs"),
            Arc::from("resource"),
            false,
            paths.iter().map(|&(p, min)| elem(p, min)).collect(),
        )
    }

    #[test]
    fn parsed_expression_is_computed_once_and_reused() {
        let constraint =
            Constraint::new("k".into(), "error".into(), "h".into(), Some("true".into()));
        let first = constraint.parsed_expression().expect("has expression");
        let second = constraint.parsed_expression().expect("has expression");
        assert!(std::ptr::eq(first, second));
        assert!(first.is_ok());
    }

    #[test]
    fn parsed_expression_caches_parse_errors_and_skips_absent() {
        let bad = Constraint::new(
            "k".into(),
            "error".into(),
            "h".into(),
            Some("name.where(".into()),
        );
        assert!(bad.parsed_expression().expect("has expression").is_err());
        let none = Constraint::new("k".into(), "error".into(), "h".into(), None);
        assert!(none.parsed_expression().is_none());
    }

    #[test]
    fn element_returns_the_first_element_with_the_path() {
        let sd = sliced_observation();
        assert_eq!(sd.element("Observation.category").map(|e| e.min), Some(0));
        assert_eq!(sd.element("Observation.code").map(|e| e.min), Some(1));
        assert!(sd.element("Observation.cat").is_none());
    }

    #[test]
    fn descendants_span_slices_and_stop_at_the_next_sibling() {
        let sd = sliced_observation();
        let paths: Vec<_> = sd
            .descendants("Observation.category")
            .iter()
            .map(|e| (e.path.as_ref(), e.min))
            .collect();
        assert_eq!(
            paths,
            [
                ("Observation.category.coding", 0),
                ("Observation.category", 1),
                ("Observation.category.coding", 1),
            ]
        );
        assert_eq!(sd.descendants("Observation").len(), 6);
        assert!(sd.descendants("Observation.code").is_empty());
        assert!(sd.descendants("Observation.unknown").is_empty());
    }
}
