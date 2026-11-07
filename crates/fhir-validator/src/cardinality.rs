use fhir_definitions::StructureDefinition;
use fhir_parser::Resource;

use crate::diagnostic::Diagnostic;
use crate::nested::check_children;

/// Validate cardinality constraints (min / max) for depth-1 elements.
///
/// Emits `CARDINALITY_001` when a required element (min > 0) is absent, and
/// `CARDINALITY_002` when an element exceeds its declared maximum cardinality
/// or a choice element holds more than one variant.
pub fn validate(resource: &Resource, sd: &StructureDefinition, diagnostics: &mut Vec<Diagnostic>) {
    let resource_type = resource.resource_type.as_ref();
    check_children(
        &resource.fields,
        sd,
        resource_type,
        resource_type,
        diagnostics,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use fhir_definitions::{ElementDefinition, StructureDefinition};
    use fhir_parser::{Node, Span, Value};
    use indexmap::IndexMap;

    fn make_elem(path: &str, min: u32, max: &str) -> ElementDefinition {
        ElementDefinition {
            path: Arc::from(path),
            min,
            max: Arc::from(max),
            types: vec![],
            constraints: vec![],
            binding: None,
        }
    }

    fn make_sd(resource_type: &str, elems: Vec<ElementDefinition>) -> StructureDefinition {
        let mut snapshot = vec![make_elem(resource_type, 0, "1")];
        snapshot.extend(elems);
        StructureDefinition {
            url: Arc::from(
                format!("http://hl7.org/fhir/StructureDefinition/{resource_type}").as_str(),
            ),
            name: Arc::from(resource_type),
            kind: Arc::from("resource"),
            is_abstract: false,
            base_definition: None,
            snapshot,
        }
    }

    fn make_resource(resource_type: &str, fields: &[(&str, Value)]) -> fhir_parser::Resource {
        let mut map: IndexMap<Arc<str>, Node> = IndexMap::new();
        let span = Span::default();
        for (name, val) in fields {
            map.insert(
                Arc::from(*name),
                Node {
                    value: val.clone(),
                    span,
                },
            );
        }
        fhir_parser::Resource {
            resource_type: Arc::from(resource_type),
            id: None,
            fields: map,
        }
    }

    #[test]
    fn required_field_present_produces_no_error() {
        let sd = make_sd("Obs", vec![make_elem("Obs.status", 1, "1")]);
        let resource = make_resource("Obs", &[("status", Value::Str(Arc::from("final")))]);
        let mut diagnostics = vec![];
        validate(&resource, &sd, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn required_field_absent_produces_cardinality_001() {
        let sd = make_sd("Obs", vec![make_elem("Obs.status", 1, "1")]);
        let resource = make_resource("Obs", &[]);
        let mut diagnostics = vec![];
        validate(&resource, &sd, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code.as_ref(), "CARDINALITY_001");
        assert!(diagnostics[0].path.contains("status"));
    }

    #[test]
    fn optional_field_absent_produces_no_error() {
        let sd = make_sd("Patient", vec![make_elem("Patient.active", 0, "1")]);
        let resource = make_resource("Patient", &[]);
        let mut diagnostics = vec![];
        validate(&resource, &sd, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn array_exceeding_max_produces_cardinality_002() {
        let sd = make_sd("Patient", vec![make_elem("Patient.active", 0, "1")]);
        let items = vec![
            Node {
                value: Value::Bool(true),
                span: Span::default(),
            },
            Node {
                value: Value::Bool(false),
                span: Span::default(),
            },
        ];
        let resource = make_resource("Patient", &[("active", Value::Array(items))]);
        let mut diagnostics = vec![];
        validate(&resource, &sd, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code.as_ref(), "CARDINALITY_002");
    }

    #[test]
    fn array_within_max_produces_no_error() {
        let sd = make_sd("Patient", vec![make_elem("Patient.name", 0, "*")]);
        let items = vec![
            Node {
                value: Value::Str(Arc::from("Alice")),
                span: Span::default(),
            },
            Node {
                value: Value::Str(Arc::from("Bob")),
                span: Span::default(),
            },
        ];
        let resource = make_resource("Patient", &[("name", Value::Array(items))]);
        let mut diagnostics = vec![];
        validate(&resource, &sd, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn missing_required_choice_is_reported_at_its_placeholder_path() {
        let sd = make_sd("Obs", vec![make_elem("Obs.value[x]", 1, "1")]);
        let resource = make_resource("Obs", &[]);
        let mut diagnostics = vec![];
        validate(&resource, &sd, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code.as_ref(), "CARDINALITY_001");
        assert_eq!(diagnostics[0].path, "Obs.value[x]");
    }
}
