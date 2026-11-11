use std::sync::Arc;

use fhir_definitions::StructureDefinition;
use fhir_parser::Resource;

use crate::diagnostic::{Diagnostic, Severity};

/// Evaluate FHIRPath `constraint.expression` invariants from the root element.
///
/// Emits `INVARIANT_<key>` for every constraint whose expression evaluates to
/// anything other than `[true]`. Expressions that fail to parse or evaluate
/// emit one `Information` `INVARIANT_SKIPPED` naming the key and the reason, so
/// a clean result never hides an unchecked constraint (shortcut: the parser
/// lacks `%` environment variables such as `%resource`, so `dom-3` is always
/// skipped; add them, with a real `resolve()`, to check contained references).
/// `is_choice_variant(base, suffix)` makes choice elements navigable (`value`
/// finds `valueQuantity`).
pub fn validate(
    resource: &Resource,
    sd: &StructureDefinition,
    is_choice_variant: &dyn Fn(&str, &str) -> bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let resource_type = resource.resource_type.as_ref();

    // The root element carries the constraints for the whole resource.
    let Some(root_elem) = sd.element(resource_type) else {
        return;
    };

    let context_value = fhir_fhirpath::resource_to_value(resource);
    let context = vec![context_value];

    for constraint in &root_elem.constraints {
        let Some(parsed) = constraint.parsed_expression() else {
            continue;
        };

        let result = match parsed
            .as_ref()
            .map_err(|e| fhir_fhirpath::EvalError::from(e.clone()))
            .and_then(|expr| {
                fhir_fhirpath::evaluate_parsed_with_choices(expr, &context, is_choice_variant)
            }) {
            Ok(r) => r,
            // Parse/evaluation errors are non-fatal: report the skip, then move on.
            Err(e) => {
                diagnostics.push(Diagnostic {
                    severity: Severity::Information,
                    code: Arc::from("INVARIANT_SKIPPED"),
                    message: format!("constraint {} not checked: {e}", constraint.key),
                    path: resource_type.to_owned(),
                    location: None,
                    rule: None,
                });
                continue;
            }
        };

        // A constraint is satisfied when the expression returns exactly [true].
        // An empty collection or [false] counts as a violation.
        let satisfied = matches!(result.as_slice(), [fhir_fhirpath::Value::Bool(true)]);

        if !satisfied {
            // "warning" and "information" are downgraded; "error" and unknown values stay errors.
            let severity = match constraint.severity.as_ref() {
                "warning" | "information" => Severity::Warning,
                _ => Severity::Error,
            };
            diagnostics.push(Diagnostic {
                severity,
                code: Arc::from(format!("INVARIANT_{}", constraint.key)),
                message: constraint.human.to_string(),
                path: resource_type.to_owned(),
                location: None,
                rule: None,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use fhir_definitions::{Constraint, ElementDefinition, StructureDefinition};
    use indexmap::IndexMap;

    fn make_constraint(key: &str, severity: &str, expression: &str) -> Constraint {
        Constraint::new(
            Arc::from(key),
            Arc::from(severity),
            Arc::from(format!("Constraint {key} violated").as_str()),
            Some(Arc::from(expression)),
        )
    }

    fn make_sd_with_constraints(
        resource_type: &str,
        constraints: Vec<Constraint>,
    ) -> StructureDefinition {
        StructureDefinition::new(
            Arc::from(format!("http://hl7.org/fhir/StructureDefinition/{resource_type}").as_str()),
            Arc::from(resource_type),
            Arc::from("resource"),
            false,
            None,
            vec![ElementDefinition {
                path: Arc::from(resource_type),
                min: 0,
                max: Arc::from("*"),
                types: vec![],
                constraints,
                binding: None,
                slice_name: None,
                xml_attr: false,
            }],
        )
    }

    fn make_empty_resource(resource_type: &str) -> fhir_parser::Resource {
        fhir_parser::Resource {
            resource_type: Arc::from(resource_type),
            id: None,
            from_xml: false,
            fields: IndexMap::new(),
            trimmed_attributes: Vec::new(),
        }
    }

    #[test]
    fn satisfied_constraint_produces_no_diagnostic() {
        // "true" always evaluates to [true]
        let sd =
            make_sd_with_constraints("Patient", vec![make_constraint("test-1", "error", "true")]);
        let resource = make_empty_resource("Patient");
        let mut d = vec![];
        validate(&resource, &sd, &|_, _| false, &mut d);
        assert!(d.is_empty());
    }

    #[test]
    fn violated_error_constraint_produces_invariant_error() {
        // "false" evaluates to [false] — constraint violated
        let sd =
            make_sd_with_constraints("Patient", vec![make_constraint("test-2", "error", "false")]);
        let resource = make_empty_resource("Patient");
        let mut d = vec![];
        validate(&resource, &sd, &|_, _| false, &mut d);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "INVARIANT_test-2");
        assert_eq!(d[0].severity, Severity::Error);
    }

    #[test]
    fn violated_warning_constraint_produces_invariant_warning() {
        let sd = make_sd_with_constraints(
            "Patient",
            vec![make_constraint("test-3", "warning", "false")],
        );
        let resource = make_empty_resource("Patient");
        let mut d = vec![];
        validate(&resource, &sd, &|_, _| false, &mut d);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].severity, Severity::Warning);
    }

    #[test]
    fn absent_expression_is_skipped() {
        let sd = make_sd_with_constraints(
            "Patient",
            vec![Constraint::new(
                Arc::from("test-4"),
                Arc::from("error"),
                Arc::from("no expression"),
                None,
            )],
        );
        let resource = make_empty_resource("Patient");
        let mut d = vec![];
        validate(&resource, &sd, &|_, _| false, &mut d);
        assert!(d.is_empty());
    }

    #[test]
    fn evaluation_error_reports_invariant_skipped_information() {
        // This expression references an undefined function — should not crash.
        let sd = make_sd_with_constraints(
            "Patient",
            vec![make_constraint(
                "test-5",
                "error",
                "undefinedFunction99999()",
            )],
        );
        let resource = make_empty_resource("Patient");
        let mut d = vec![];
        validate(&resource, &sd, &|_, _| false, &mut d);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "INVARIANT_SKIPPED");
        assert_eq!(d[0].severity, Severity::Information);
        assert!(d[0].message.contains("test-5"), "{}", d[0].message);
    }

    #[test]
    fn unparsable_expression_reports_invariant_skipped_information() {
        let sd = make_sd_with_constraints(
            "Patient",
            vec![make_constraint("test-7", "error", "name.where(")],
        );
        let mut d = vec![];
        validate(&make_empty_resource("Patient"), &sd, &|_, _| false, &mut d);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "INVARIANT_SKIPPED");
        assert_eq!(d[0].severity, Severity::Information);
        assert!(d[0].message.contains("test-7"), "{}", d[0].message);
    }

    #[test]
    fn empty_expression_result_is_treated_as_violation() {
        // Expression that returns empty: evaluating a path that doesn't exist
        // on an empty resource returns [] which is a violation.
        let sd = make_sd_with_constraints(
            "Patient",
            vec![make_constraint(
                "test-6",
                "error",
                "contained.contained.exists()",
            )],
        );
        // Empty resource: no contained → contained.contained → [] → .exists() → [false]
        let resource = make_empty_resource("Patient");
        let mut d = vec![];
        validate(&resource, &sd, &|_, _| false, &mut d);
        // [false] → violated
        assert_eq!(d.len(), 1);
    }

    #[test]
    fn multiple_constraints_are_evaluated_independently() {
        let sd = make_sd_with_constraints(
            "Patient",
            vec![
                make_constraint("pass-1", "error", "true"),
                make_constraint("fail-1", "error", "false"),
                make_constraint("pass-2", "warning", "true"),
                make_constraint("fail-2", "warning", "false"),
            ],
        );
        let resource = make_empty_resource("Patient");
        let mut d = vec![];
        validate(&resource, &sd, &|_, _| false, &mut d);
        assert_eq!(d.len(), 2);
        let codes: Vec<&str> = d.iter().map(|x| x.code.as_ref()).collect();
        assert!(codes.contains(&"INVARIANT_fail-1"));
        assert!(codes.contains(&"INVARIANT_fail-2"));
    }

    #[test]
    fn information_severity_maps_to_warning() {
        let sd = make_sd_with_constraints(
            "Patient",
            vec![make_constraint("test-10", "information", "false")],
        );
        let mut d = vec![];
        validate(&make_empty_resource("Patient"), &sd, &|_, _| false, &mut d);
        assert_eq!(d[0].severity, Severity::Warning);
    }
}
