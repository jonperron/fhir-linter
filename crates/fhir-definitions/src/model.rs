use std::collections::HashSet;
use std::sync::Arc;

/// A lightweight StructureDefinition — only the fields the validator needs.
#[derive(Debug, Clone)]
pub struct StructureDefinition {
    pub url: Arc<str>,
    pub name: Arc<str>,
    pub kind: Arc<str>,
    pub is_abstract: bool,
    pub base_definition: Option<Arc<str>>,
    pub snapshot: Vec<ElementDefinition>,
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
}

/// A FHIRPath constraint expression on an element.
#[derive(Debug, Clone)]
pub struct Constraint {
    pub key: Arc<str>,
    /// Either `"error"` or `"warning"`.
    pub severity: Arc<str>,
    pub human: Arc<str>,
    /// FHIRPath expression; absent for extension-only constraints.
    pub expression: Option<Arc<str>>,
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
    pub codes: HashSet<Arc<str>>,
    /// False when the system is a fragment/not-present, so absence of a code proves nothing.
    pub complete: bool,
}
