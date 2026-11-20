//! The Registry as the FHIRPath `%terminologies` service: `$expand`,
//! `$validate-code` and `$translate` answered from the loaded definitions, in
//! the output shapes of the R5 OperationDefinitions ValueSet-expand,
//! ValueSet-validate-code and ConceptMap-translate.

use fhir_fhirpath::{Collection, Terminologies, Value};

use super::Registry;

impl Terminologies for Registry {
    /// shortcut: `status` and `expansion.timestamp` (both 1..1) are left out, as
    /// the Registry keeps neither a status nor a clock; add them once a caller
    /// reads them.
    fn expand(&self, value_set: &str) -> Option<Value> {
        let codes = Registry::expand(self, value_set)?;
        let total = i64::try_from(codes.len()).ok()?;
        let contains = codes
            .into_iter()
            .map(|(system, code, display)| {
                Value::object(
                    "BackboneElement",
                    vec![
                        ("system", vec![Value::fhir_string(system, "uri")]),
                        ("code", vec![Value::fhir_string(code, "code")]),
                        (
                            "display",
                            display
                                .map(|d| Value::fhir_string(d, "string"))
                                .into_iter()
                                .collect(),
                        ),
                    ],
                )
            })
            .collect();
        let expansion = Value::object(
            "BackboneElement",
            vec![
                ("total", vec![Value::Integer(total)]),
                ("contains", contains),
            ],
        );
        Some(resource(
            "ValueSet",
            vec![
                ("url", vec![Value::fhir_string(value_set, "uri")]),
                ("expansion", vec![expansion]),
            ],
        ))
    }

    /// Valid when one coding is in the ValueSet; not valid when every coding (at
    /// least one) is decided out of it; else undecided.
    fn validate_vs(&self, value_set: &str, codings: &[(Option<&str>, &str)]) -> Option<Value> {
        let answers: Vec<_> = codings
            .iter()
            .map(|&(system, code)| self.value_set_contains(value_set, system, code))
            .collect();
        let result = if answers.contains(&Some(true)) {
            true
        } else if !answers.is_empty() && answers.iter().all(|a| *a == Some(false)) {
            false
        } else {
            return None;
        };
        Some(parameters(vec![parameter(
            "result",
            "valueBoolean",
            Value::Bool(result),
        )]))
    }

    fn translate(&self, concept_map: &str, system: Option<&str>, code: &str) -> Option<Value> {
        let matches = Registry::translate(self, concept_map, system, code)?;
        // ConceptMap-translate: "The value can only be true if at least one
        // returned match has a relationship other than 'not-related-to'."
        let result = matches.iter().any(|&(_, _, rel)| rel != "not-related-to");
        let mut params = vec![parameter("result", "valueBoolean", Value::Bool(result))];
        params.extend(
            matches
                .into_iter()
                .map(|(target_system, target_code, relationship)| {
                    let concept = Value::object(
                        "Coding",
                        vec![
                            ("system", vec![Value::fhir_string(target_system, "uri")]),
                            ("code", vec![Value::fhir_string(target_code, "code")]),
                        ],
                    );
                    Value::object(
                        "BackboneElement",
                        vec![
                            ("name", vec![Value::fhir_string("match", "string")]),
                            (
                                "part",
                                vec![
                                    parameter(
                                        "relationship",
                                        "valueCode",
                                        Value::fhir_string(relationship, "code"),
                                    ),
                                    parameter("concept", "valueCoding", concept),
                                ],
                            ),
                        ],
                    )
                }),
        );
        Some(parameters(params))
    }
}

/// A resource of type `resource_type` with `fields` after its `resourceType`.
fn resource(resource_type: &str, fields: Vec<(&str, Collection)>) -> Value {
    let mut all = vec![("resourceType", vec![Value::String(resource_type.into())])];
    all.extend(fields);
    Value::object(resource_type, all)
}

/// A Parameters resource holding `parameter`.
fn parameters(parameter: Collection) -> Value {
    resource("Parameters", vec![("parameter", parameter)])
}

/// A `Parameters.parameter` (or `part`) named `name` with `value` in the
/// `value[x]` field `value_field` (`valueBoolean`).
fn parameter(name: &str, value_field: &str, value: Value) -> Value {
    Value::object(
        "BackboneElement",
        vec![
            ("name", vec![Value::fhir_string(name, "string")]),
            (value_field, vec![value]),
        ],
    )
}
