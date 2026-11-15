//! FHIR additions to FHIRPath (R5 fhirpath.html §2.1.9.1): `resolve()`,
//! `memberOf()` and `htmlChecks()`. None guesses: a reference `resolve()` cannot find is an
//! error, and `memberOf()` is empty when membership cannot be decided.
use std::sync::Arc;

use crate::ast::Expr;
use crate::error::EvalError;
use crate::value::{Collection, Value};

use super::Evaluator;
use super::helpers::{check_arity, singleton};

impl Evaluator<'_> {
    /// Handle `resolve()`, `memberOf()` and `htmlChecks()`.
    /// Returns `None` if the function name is not in this category.
    pub(super) fn call_fhir_function(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Option<Result<Collection, EvalError>> {
        match name {
            "resolve" => Some(check_arity(name, args, 0).and_then(|()| self.resolve(focus))),
            "memberOf" => Some(self.member_of(args, focus, ctx)),
            "htmlChecks" => Some(check_arity(name, args, 0).and_then(|()| html_checks(focus))),
            _ => None,
        }
    }

    /// Each item that is a reference string, or a Reference with one, adds the
    /// resource it names (fhirpath.html `resolve()`).
    ///
    /// fhirpath.html ignores an item that does not resolve, but an invariant
    /// that then sees empty would fail on a reference this evaluator merely
    /// cannot follow, so such an item is an error and the invariant is skipped.
    ///
    /// shortcut: only `#id` (a resource in `%rootResource.contained`) and `#`
    /// (`%rootResource`) resolve (references.html, ref-1); Bundle-local and
    /// server references are errors. Upgrade when the environment carries the
    /// enclosing Bundle and the entry's fullUrl.
    fn resolve(&self, focus: &[Value]) -> Result<Collection, EvalError> {
        // `%rootResource` falls back to `%resource`; with neither, nothing resolves.
        let root = self
            .environment_variable("rootResource")
            .unwrap_or_default();
        let root = root.first().filter(|root| matches!(root, Value::Object(_)));
        let contained = root
            .and_then(|root| root.fields()?.get("contained"))
            .map_or(&[][..], Vec::as_slice);
        focus
            .iter()
            .map(|item| {
                let reference = reference(item);
                let found = match reference.as_deref().and_then(|r| r.strip_prefix('#')) {
                    Some("") => root.cloned(),
                    Some(id) => contained
                        .iter()
                        .find(|resource| field_string(resource, "id").as_deref() == Some(id))
                        .cloned(),
                    None => None,
                };
                found.ok_or_else(|| {
                    EvalError::Type(format!(
                        "resolve() cannot follow '{}': only #id references to contained \
                         resources are resolved",
                        reference.as_deref().unwrap_or("<no reference>"),
                    ))
                })
            })
            .collect()
    }

    /// `memberOf(valueset)`: whether the single code, Coding or
    /// CodeableConcept input is in the ValueSet, through
    /// [`super::Environment::member_of`]. Empty when that cannot be decided.
    fn member_of(
        &self,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Result<Collection, EvalError> {
        check_arity("memberOf", args, 1)?;
        let url = self.eval(&args[0], ctx)?;
        let Some(url) = singleton(&url, "memberOf")?.and_then(Value::as_string) else {
            return Ok(vec![]);
        };
        let (Some(_), Some(member_of)) =
            (singleton(focus, "memberOf")?, self.environment.member_of)
        else {
            return Ok(vec![]);
        };
        // The item itself: `singleton` gives a primitive's value, without its FHIR type.
        let item = &focus[0];
        let coding_member = |coding: &Value| {
            let system = field_string(coding, "system")?;
            member_of(&url, Some(&system), &field_string(coding, "code")?)
        };
        let member = match item {
            // shortcut: only a `code` element is looked up without a system; a
            // plain String is empty, as fhirpath.html answers it only for a
            // ValueSet of one code system, which `member_of` cannot tell.
            Value::Primitive(code, _, Some(fhir_type)) if &**fhir_type == "code" => code
                .as_string()
                .and_then(|code| member_of(&url, None, &code)),
            // shortcut: objects carry no type, so a CodeableConcept is told by its
            // `coding` and a Coding by its `code` (a Quantity's system and code are
            // looked up too); upgrade when objects carry their FHIR type.
            Value::Object(fields) if fields.contains_key("coding") => {
                // fhirpath.html: true if any code in the concept is a member.
                let codings = fields.get("coding").into_iter().flatten();
                let answers: Vec<_> = codings.map(coding_member).collect();
                if answers.contains(&Some(true)) {
                    Some(true)
                } else if !answers.is_empty() && answers.iter().all(|a| *a == Some(false)) {
                    Some(false)
                } else {
                    None
                }
            }
            Value::Object(fields) if fields.contains_key("code") => coding_member(item),
            _ => None,
        };
        Ok(member.map(Value::Bool).into_iter().collect())
    }
}

/// `htmlChecks()`: whether the single xhtml (string) input meets the narrative rules;
/// empty for an empty or non-string input (official HTMLChecks test htmlTest04).
fn html_checks(focus: &[Value]) -> Result<Collection, EvalError> {
    Ok(match singleton(focus, "htmlChecks")? {
        Some(Value::String(markup)) => vec![Value::Bool(super::html_checks::html_checks(markup))],
        _ => vec![],
    })
}

/// The reference a `resolve()` input item carries: a string (uri, url or
/// canonical), or the `reference` of a Reference.
fn reference(item: &Value) -> Option<Arc<str>> {
    match item.primitive_value() {
        Value::String(reference) => Some(Arc::clone(reference)),
        Value::Object(_) => field_string(item, "reference"),
        _ => None,
    }
}

/// The single String value of the child `name` of an object.
fn field_string(item: &Value, name: &str) -> Option<Arc<str>> {
    let Value::Object(fields) = item else {
        return None;
    };
    match fields.get(name)?.as_slice() {
        [value] => match value.primitive_value() {
            Value::String(s) => Some(Arc::clone(s)),
            _ => None,
        },
        _ => None,
    }
}
