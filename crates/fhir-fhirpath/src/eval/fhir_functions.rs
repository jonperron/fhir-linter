//! FHIR additions to FHIRPath (R5 fhirpath.html §2.1.9.1): `resolve()`,
//! `memberOf()`, `conformsTo()` and `htmlChecks()`. None guesses: a reference `resolve()`
//! cannot find, membership `memberOf()` cannot decide and a profile `conformsTo()` cannot
//! check are errors.
use std::sync::{Arc, LazyLock};

use regex_lite::Regex;

use crate::ast::Expr;
use crate::error::EvalError;
use crate::value::{Collection, Value};

use super::Evaluator;
use super::helpers::{check_arity, singleton};

/// The literal reference regex of references.html, with the resource type
/// list left to the Registry: an optional http(s) base, `Type/id` and an
/// optional `/_history/vid`. `None` (never, the pattern is fixed) resolves nothing.
static TYPED_REFERENCE: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(concat!(
        r"^(?:https?://(?:[A-Za-z0-9\-\\.:%$]*/)+)?",
        r"([A-Za-z]+)/[A-Za-z0-9\-.]{1,64}",
        r"(?:/_history/[A-Za-z0-9\-.]{1,64})?$",
    ))
    .ok()
});

impl Evaluator<'_> {
    /// Handle `resolve()`, `memberOf()`, `conformsTo()` and `htmlChecks()`.
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
            "conformsTo" => Some(self.conforms_to(args, focus, ctx)),
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
    /// `#id` (a resource in `%rootResource.contained`) and `#` (`%rootResource`)
    /// resolve (references.html, ref-1). A `[Type]/[id]` reference, relative or
    /// absolute, gives a [`Value::UnresolvedResource`] of that type, so type
    /// tests on it work; this needs [`super::Environment::is_subtype`] to know
    /// the resource types.
    ///
    /// shortcut: other references (`urn:uuid:`, Bundle-local by fullUrl) are
    /// errors. Upgrade when the environment carries the enclosing Bundle.
    fn resolve(&self, focus: &[Value]) -> Result<Collection, EvalError> {
        // `%rootResource` falls back to `%resource`; with neither, nothing resolves.
        let root = self
            .environment_variable("rootResource")
            .unwrap_or_default();
        let root = root
            .first()
            .filter(|root| matches!(root, Value::Object(..)));
        let contained = root
            .and_then(|root| root.fields()?.get("contained"))
            .map_or(&[][..], Vec::as_slice);
        focus
            .iter()
            .map(|item| {
                let reference = reference(item);
                let found = match reference.as_deref().map(|r| (r, r.strip_prefix('#'))) {
                    Some((_, Some(""))) => root.cloned(),
                    Some((_, Some(id))) => contained
                        .iter()
                        .find(|resource| field_string(resource, "id").as_deref() == Some(id))
                        .cloned(),
                    Some((typed, None)) => self.typed_reference(typed),
                    None => None,
                };
                found.ok_or_else(|| {
                    EvalError::Type(format!(
                        "resolve() cannot follow '{}': only #id references to contained \
                         resources and [Type]/[id] references are resolved",
                        reference.as_deref().unwrap_or("<no reference>"),
                    ))
                })
            })
            .collect()
    }

    /// The resource a literal `[Type]/[id]` reference names, known by its type
    /// only; `None` unless `Type` is a resource type.
    fn typed_reference(&self, reference: &str) -> Option<Value> {
        let is_subtype = self.environment.is_subtype?;
        let resource_type = TYPED_REFERENCE
            .as_ref()?
            .captures(reference)?
            .get(1)?
            .as_str();
        is_subtype(resource_type, "Resource").then(|| Value::UnresolvedResource {
            resource_type: Arc::from(resource_type),
            reference: Arc::from(reference),
        })
    }

    /// `memberOf(valueset)`: whether the single code, Coding or
    /// CodeableConcept input is in the ValueSet, through
    /// [`super::Environment::member_of`]. An error when that cannot be decided.
    fn member_of(
        &self,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Result<Collection, EvalError> {
        check_arity("memberOf", args, 1)?;
        let url = self.eval(&args[0], ctx)?;
        // Both cardinalities are checked before either empty result.
        let url = singleton(&url, "memberOf")?.and_then(Value::as_string);
        let (Some(url), Some(_)) = (url, singleton(focus, "memberOf")?) else {
            return Ok(vec![]);
        };
        let undecided = || {
            EvalError::Type(format!(
                "memberOf() cannot decide membership in '{url}': the terminology is not loaded \
                 or does not cover it",
            ))
        };
        // Without the hook every lookup is undecided, but a type memberOf
        // is not defined on stays empty: it never reaches a lookup.
        let member_of = |system: Option<&str>, code: &str| {
            self.environment
                .member_of
                .and_then(|member_of| member_of(&url, system, code))
        };
        // The item itself: `singleton` gives a primitive's value, without its FHIR type.
        let item = &focus[0];
        let coding_member = |coding: &Value| {
            let system = field_string(coding, "system")?;
            member_of(Some(&system), &field_string(coding, "code")?)
        };
        let member = match (item, coded_type(item)) {
            // shortcut: only a `code` element is looked up without a system; a
            // plain String is empty, as fhirpath.html answers it only for a
            // ValueSet of one code system, which `member_of` cannot tell.
            (Value::Primitive(code, _, Some(fhir_type)), _) if &**fhir_type == "code" => {
                code.as_string().and_then(|code| member_of(None, &code))
            }
            (Value::Object(fields, _), Some("CodeableConcept")) => {
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
            (_, Some("Coding")) => coding_member(item),
            // A plain String, or a type memberOf is not defined on, is empty
            // (see the shortcut above): not a lookup that failed.
            _ => return Ok(vec![]),
        };
        // Empty would fail an invariant on membership this evaluator merely cannot decide.
        Ok(vec![Value::Bool(member.ok_or_else(undecided)?)])
    }

    /// `conformsTo(structure)`: whether the single input conforms to the
    /// StructureDefinition at `structure`, through
    /// [`super::Environment::conforms_to`]. fhirpath.html: "If the input is not a
    /// single item, the structure is empty, or the structure cannot be resolved to
    /// a valid profile, the result is empty." An error when it cannot be checked.
    fn conforms_to(
        &self,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Result<Collection, EvalError> {
        check_arity("conformsTo", args, 1)?;
        let url = self.eval(&args[0], ctx)?;
        let url = singleton(&url, "conformsTo")?
            .map(|url| {
                url.as_string().ok_or_else(|| {
                    EvalError::Type("conformsTo() takes a string structure URL".to_string())
                })
            })
            .transpose()?;
        let (Some(url), [item]) = (url, focus) else {
            return Ok(vec![]);
        };
        // A null placeholder (a JSON `null` array entry) is no item.
        if matches!(item, Value::Null) {
            return Ok(vec![]);
        }
        let unchecked =
            |reason: &str| EvalError::Type(format!("conformsTo() cannot check '{url}': {reason}"));
        let conforms_to = self
            .environment
            .conforms_to
            .ok_or_else(|| unchecked("no StructureDefinitions are loaded"))?;
        let fhir_type = match item {
            Value::Object(_, fhir_type) | Value::Primitive(_, _, fhir_type) => fhir_type.as_deref(),
            Value::UnresolvedResource { resource_type, .. } => Some(&**resource_type),
            _ => None,
        }
        .ok_or_else(|| unchecked("the input's FHIR type is unknown"))?;
        match conforms_to(fhir_type, &url) {
            None => Ok(vec![]),
            Some(conforms) => Ok(vec![Value::Bool(conforms.ok_or_else(|| {
                unchecked(&format!(
                    "{fhir_type} is not checked against a constraint profile"
                ))
            })?)]),
        }
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
        Value::Object(..) => field_string(item, "reference"),
        _ => None,
    }
}

/// An object's FHIR type; an untyped one is told by its fields, `coding` for
/// a CodeableConcept and `code` for a Coding.
pub(super) fn coded_type(item: &Value) -> Option<&str> {
    match item {
        Value::Object(_, Some(fhir_type)) => Some(fhir_type),
        Value::Object(fields, None) if fields.contains_key("coding") => Some("CodeableConcept"),
        Value::Object(fields, None) if fields.contains_key("code") => Some("Coding"),
        _ => None,
    }
}

/// The single String value of the child `name` of an object.
pub(super) fn field_string(item: &Value, name: &str) -> Option<Arc<str>> {
    let Value::Object(fields, _) = item else {
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
