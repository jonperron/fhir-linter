//! `%terminologies` (R5 fhirpath.html §2.1.9.4, Terminology Service API):
//! `expand()`, `validateVS()` and `translate()`, answered by the caller's
//! [`Terminologies`] hook. None guesses: without a hook, or when the hook
//! cannot decide, the call is an error.
use std::sync::Arc;

use crate::ast::Expr;
use crate::error::EvalError;
use crate::value::{Collection, Value};

use super::Evaluator;
use super::fhir_functions::{coded_type, field_string};
use super::helpers::check_arity_range;

/// A terminology service for `%terminologies`, see
/// [`super::Environment::terminologies`]. Each method answers as the FHIR R5
/// operation it stands for, or `None` when the answer cannot be decided.
pub trait Terminologies {
    /// `$expand` of the ValueSet at `value_set`: a ValueSet resource with
    /// `expansion.contains`.
    fn expand(&self, value_set: &str) -> Option<Value>;

    /// `$validate-code` against the ValueSet at `value_set` of the codings
    /// `(system, code)` of a code (no system), a Coding or a CodeableConcept
    /// (valid when one of its codings is): a Parameters resource with `result`.
    fn validate_vs(&self, value_set: &str, codings: &[(Option<&str>, &str)]) -> Option<Value>;

    /// `$translate` of `code` (from `system`, when known) through the
    /// ConceptMap at `concept_map`: a Parameters resource with `result` and a
    /// `match` (parts `relationship` and `concept`) per target.
    fn translate(&self, concept_map: &str, system: Option<&str>, code: &str) -> Option<Value>;
}

impl Evaluator<'_> {
    /// Call `%terminologies.<name>(args)`; the arguments are evaluated on `ctx`.
    pub(super) fn call_terminologies(
        &self,
        name: &str,
        args: &[Expr],
        ctx: &[Value],
    ) -> Result<Collection, EvalError> {
        let (min, max) = match name {
            "expand" => (1, 2),
            "validateVS" | "translate" => (2, 3),
            _ => {
                return Err(EvalError::UndefinedFunction(format!(
                    "%terminologies.{name}"
                )));
            }
        };
        check_arity_range(name, args, min, max)?;
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            // fhirpath.html: "All these functions return an empty value if any of the
            // parameters are empty, or a collection with more than one value".
            match self.eval(arg, ctx)?.as_slice() {
                [value] => values.push(value.clone()),
                _ => return Ok(vec![]),
            }
        }
        let undecided = |why: &str| {
            EvalError::Type(format!("%terminologies.{name}() cannot be answered: {why}"))
        };
        // `params` may change the answer (`activeOnly=true`): only none is honoured.
        if let Some(params) = values.get(max - 1) {
            match params.primitive_value() {
                Value::String(params) if params.is_empty() => {}
                Value::String(_) => return Err(undecided("params are not supported")),
                _ => return Ok(vec![]),
            }
        }
        let url = match values[0].primitive_value() {
            Value::String(url) => Arc::clone(url),
            // fhirpath.html also takes "an actual ValueSet" (or ConceptMap).
            Value::Object(fields, _) if fields.contains_key("resourceType") => {
                return Err(undecided(
                    "only a canonical URL is supported, not a resource",
                ));
            }
            _ => return Ok(vec![]),
        };
        let coded = values.get(1).filter(|_| name != "expand");
        let codings = match coded {
            Some(coded) => match coded_codings(coded, name == "validateVS") {
                Some(codings) => codings,
                // "or one or more of the parameters are not valid".
                None => return Ok(vec![]),
            },
            None => Vec::new(),
        };
        let Some(terminologies) = self.environment.terminologies else {
            return Err(undecided("no terminology service is set"));
        };
        let codings: Vec<_> = codings
            .iter()
            .map(|(system, code)| (system.as_deref(), &**code))
            .collect();
        let answer = match (name, codings.as_slice()) {
            ("expand", _) => terminologies.expand(&url),
            ("validateVS", _) => terminologies.validate_vs(&url, &codings),
            (_, [(system, code)]) => terminologies.translate(&url, *system, code),
            _ => None,
        };
        let answer = answer.ok_or_else(|| {
            undecided(&format!(
                "'{url}' is not loaded, or the loaded definitions do not decide it"
            ))
        })?;
        Ok(vec![answer])
    }
}

/// A coding as `(system, code)`.
type Coding = (Option<Arc<str>>, Arc<str>);

/// The `(system, code)` codings of a coded argument: a `code` element (no
/// system), a Coding, or with `codeable_concept` a CodeableConcept. `None` for
/// any other value, as fhirpath.html takes "a resource element that is a code",
/// not a plain String.
fn coded_codings(coded: &Value, codeable_concept: bool) -> Option<Vec<Coding>> {
    let coding = |coding: &Value| {
        Some((
            field_string(coding, "system"),
            field_string(coding, "code")?,
        ))
    };
    match (coded, coded_type(coded)) {
        (Value::Primitive(code, _, Some(fhir_type)), _) if &**fhir_type == "code" => {
            Some(vec![(None, code.as_string()?)])
        }
        (_, Some("Coding")) => Some(vec![coding(coded)?]),
        (Value::Object(fields, _), Some("CodeableConcept")) if codeable_concept => fields
            .get("coding")
            .into_iter()
            .flatten()
            .map(coding)
            .collect::<Option<Vec<_>>>()
            // Without a coding (text alone) there is no code to validate.
            .filter(|codings| !codings.is_empty()),
        _ => None,
    }
}
