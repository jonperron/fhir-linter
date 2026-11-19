//! Strict navigation (N1 §3): "If the identifier cannot be resolved, the
//! evaluation will end and signal an error to the calling environment."
use std::collections::HashMap;
use std::sync::Arc;

use super::{Evaluator, HasElement};
use crate::error::EvalError;
use crate::value::Value;

/// The instance path (`Patient.contact`) of each object, keyed by the address
/// of its children map: a backbone's type cannot tell its children, its path
/// from the resource can (`Registry::has_element`).
pub(super) type InstancePaths = HashMap<usize, Arc<str>>;

impl Evaluator<'_> {
    /// Under [`super::Environment::strict`], fail when no item of `ctx`
    /// resolves `name`: every item is a typed node that declares no such
    /// element and (leading the expression) is not of that type or a subtype
    /// of it. A backbone is asked by its instance path. An item that cannot
    /// tell (an untyped node, a primitive, a backbone outside the input and
    /// environment resources) may resolve it, so a mixed collection such as
    /// `descendants()` never fails.
    pub(super) fn check_strict_step(
        &self,
        ctx: &[Value],
        name: &str,
        allow_type_name: bool,
    ) -> Result<(), EvalError> {
        // shortcut: primitives and untyped nodes are not checked; check them
        // once a strict caller needs their unknown names reported.
        let Some(has_element) = self.environment.strict else {
            return Ok(());
        };
        let unresolved = |item: &Value| {
            matches!(item, Value::Object(_, Some(fhir_type))
                if !(allow_type_name && self.names_type(fhir_type, name)))
                && self
                    .definition(item, name, has_element)
                    .is_some_and(|definition| has_element(definition, name) == Some(false))
        };
        if !ctx.iter().all(unresolved) {
            return Ok(());
        }
        // Unresolved items all have a definition; an empty `ctx` has no first.
        match ctx
            .first()
            .and_then(|first| self.definition(first, name, has_element))
        {
            Some(definition) => Err(EvalError::Type(format!(
                "{name} is not an element of {definition}"
            ))),
            None => Ok(()),
        }
    }

    /// What `has_element` is asked about the typed object `item`: its type, or
    /// when the type cannot tell (a backbone), its instance path.
    fn definition<'s>(
        &'s self,
        item: &'s Value,
        name: &str,
        has_element: &HasElement<'_>,
    ) -> Option<&'s str> {
        let Value::Object(fields, Some(fhir_type)) = item else {
            return None;
        };
        if has_element(fhir_type, name).is_some() {
            return Some(fhir_type);
        }
        let paths = self.instance_paths.get_or_init(|| {
            let mut paths = InstancePaths::new();
            let roots = [self.environment.root_resource, self.environment.resource];
            // The resources first, so a context node inside one gets its full path.
            for root in roots.into_iter().flatten().chain(self.context) {
                index_paths(root, "", &mut paths);
            }
            paths
        });
        paths
            .get(&(Arc::as_ptr(fields) as usize))
            .map(|path| &**path)
    }
}

/// Record the instance path of `node` and of every object below it. A resource
/// (the root, a contained resource, a Bundle entry) starts its own path at its
/// `resourceType`; the first path recorded for an object wins.
fn index_paths(node: &Value, path: &str, paths: &mut InstancePaths) {
    let Value::Object(fields, _) = node else {
        return;
    };
    // Already reached from an earlier root, with everything below it.
    if paths.contains_key(&(Arc::as_ptr(fields) as usize)) {
        return;
    }
    let path = match fields.get("resourceType").map(Vec::as_slice) {
        Some([Value::String(resource_type)]) => Arc::clone(resource_type),
        _ => Arc::from(path),
    };
    for (key, values) in fields.iter() {
        // Only objects get paths: skip the allocation for primitive fields.
        if values
            .iter()
            .any(|value| matches!(value, Value::Object(..)))
        {
            let child = format!("{path}.{key}");
            for value in values {
                index_paths(value, &child, paths);
            }
        }
    }
    paths.entry(Arc::as_ptr(fields) as usize).or_insert(path);
}
