//! Strict navigation (N1 §3): "If the identifier cannot be resolved, the
//! evaluation will end and signal an error to the calling environment."
use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::sync::Arc;

use super::{Evaluator, HasElement};
use crate::error::EvalError;
use crate::value::Value;

/// The instance path (`Patient.contact`) of each object, keyed by the address
/// of its children map: a backbone's type cannot tell its children, its path
/// from the resource can (`Registry::has_element`).
pub(super) type InstancePaths = HashMap<usize, Arc<str>>;

/// `has_element` answers by definition, then element name.
type Declarations = HashMap<Box<str>, HashMap<Box<str>, Option<bool>>>;

/// Strict navigation for [`super::Environment::strict`]: the `has_element` hook
/// and the caches that keep its cost linear in the input. Build one per resource
/// and share it by every evaluation on that resource: the instance paths of the
/// environment resources are then indexed once, not once per evaluation.
pub struct Strict<'a> {
    has_element: &'a HasElement<'a>,
    declared: RefCell<Declarations>,
    /// The `%rootResource` and `%resource` the index was built from, held so their
    /// addresses stay theirs, and the instance paths of every object in them.
    paths: OnceCell<([Option<Value>; 2], InstancePaths)>,
}

impl<'a> Strict<'a> {
    /// Strict navigation answered by `has_element(type, name)` (see
    /// [`super::Environment::strict`]).
    pub fn new(has_element: &'a HasElement<'a>) -> Self {
        Self {
            has_element,
            declared: RefCell::default(),
            paths: OnceCell::new(),
        }
    }

    /// `has_element(definition, name)`, asked once per pair.
    fn declares(&self, definition: &str, name: &str) -> Option<bool> {
        let cached = self
            .declared
            .borrow()
            .get(definition)
            .and_then(|names| names.get(name).copied());
        cached.unwrap_or_else(|| {
            let declared = (self.has_element)(definition, name);
            self.declared
                .borrow_mut()
                .entry(definition.into())
                .or_default()
                .insert(name.into(), declared);
            declared
        })
    }

    /// The instance paths of `roots`, or `None` when the index was built from
    /// other resources (it is built from the first roots asked).
    fn paths_of(&self, roots: [Option<&Value>; 2]) -> Option<&InstancePaths> {
        let (indexed, paths) = self.paths.get_or_init(|| {
            let mut paths = InstancePaths::new();
            for root in roots.into_iter().flatten() {
                index_paths(root, "", &mut paths);
            }
            (roots.map(|root| root.cloned()), paths)
        });
        let same = indexed
            .iter()
            .zip(roots)
            .all(|(held, root)| held.as_ref().map(address) == root.map(address));
        same.then_some(paths)
    }
}

impl std::fmt::Debug for Strict<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Strict").finish_non_exhaustive()
    }
}

impl Evaluator<'_> {
    /// Under [`super::Environment::strict`], fail when no item of `ctx`
    /// resolves `name`: every item is a typed node that declares no such
    /// element and (leading the expression) is not of that type or a subtype
    /// of it. A backbone is asked by its instance path. An item that cannot
    /// tell (an untyped node, a primitive, a backbone outside the input and
    /// environment resources) may resolve it, so a mixed collection such as
    /// `descendants()` never fails. Inside an expression parameter, a leading
    /// name is resolved against the whole input collection `$this` comes from,
    /// as N1 types `$this` by that collection: `descendants().where(reference = '#')`
    /// does not fail on a descendant without `reference`.
    pub(super) fn check_strict_step(
        &self,
        ctx: &[Value],
        name: &str,
        allow_type_name: bool,
    ) -> Result<(), EvalError> {
        // shortcut: primitives and untyped nodes are not checked; check them
        // once a strict caller needs their unknown names reported.
        let Some(strict) = self.environment.strict else {
            return Ok(());
        };
        let unresolved = |item: &Value| {
            matches!(item, Value::Object(_, Some(fhir_type))
                if !(allow_type_name && self.names_type(fhir_type, name)))
                && self
                    .definition(item, name, strict)
                    .is_some_and(|definition| strict.declares(definition, name) == Some(false))
        };
        if !ctx.iter().all(unresolved) {
            return Ok(());
        }
        // shortcut: only the leading name sees the whole input; a later step
        // (`where(code.text)`) is still asked of this item's values alone. The
        // scan runs per unresolved item: a long unresolved prefix before the first
        // resolving one costs O(n) each; cache it per iteration if that shows up.
        if allow_type_name
            && self
                .focus
                .is_some_and(|focus| !focus.iter().all(unresolved))
        {
            return Ok(());
        }
        // Unresolved items all have a definition; an empty `ctx` has no first.
        match ctx
            .first()
            .and_then(|first| self.definition(first, name, strict))
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
        strict: &'s Strict<'_>,
    ) -> Option<&'s str> {
        let Value::Object(fields, Some(fhir_type)) = item else {
            return None;
        };
        let key = Arc::as_ptr(fields) as usize;
        let roots = [self.environment.root_resource, self.environment.resource];
        // A resource inside another sits in a `Resource`-typed element (`contained`,
        // `entry.resource`), the type N1 resolves its names by: any resource may have
        // them, so `contained.contained` (core dom-2) does not fail on a Bundle.
        // shortcut: `ofType(Patient)` does not narrow it back; check it once strict
        // callers need names under a type filter reported.
        let is_input = |root: &Value| address(root) == Some(key);
        if fields.contains_key("resourceType")
            && !roots
                .into_iter()
                .flatten()
                .chain(self.context)
                .any(is_input)
        {
            return None;
        }
        if strict.declares(fhir_type, name).is_some() {
            return Some(fhir_type);
        }
        let shared = strict.paths_of(roots);
        if let Some(path) = shared.and_then(|paths| paths.get(&key)) {
            return Some(path);
        }
        // Nodes outside the shared index: the input trees, indexed once per evaluation.
        let paths = self.instance_paths.get_or_init(|| {
            let mut paths = InstancePaths::new();
            // The resources first, so a context node inside one gets its full path.
            let unindexed = roots.into_iter().flatten().filter(|_| shared.is_none());
            for root in unindexed.chain(self.context) {
                index_paths(root, "", &mut paths);
            }
            paths
        });
        paths.get(&key).map(|path| &**path)
    }
}

/// The address of the children map of an object or primitive, if it has one.
fn address(value: &Value) -> Option<usize> {
    match value {
        Value::Object(fields, _) | Value::Primitive(_, fields, _) => {
            Some(Arc::as_ptr(fields) as usize)
        }
        _ => None,
    }
}

/// Record the instance path of `node` and of every object below it. A resource
/// (the root, a contained resource, a Bundle entry) starts its own path at its
/// `resourceType`; the first path recorded for an object wins.
fn index_paths(node: &Value, path: &str, paths: &mut InstancePaths) {
    let (fields, is_object) = match node {
        Value::Object(fields, _) => (fields, true),
        // A primitive's `extension` may hold a backbone (`valueTiming.repeat`).
        Value::Primitive(_, fields, _) => (fields, false),
        _ => return,
    };
    let address = Arc::as_ptr(fields) as usize;
    // Already reached from an earlier root, with everything below it.
    if is_object && paths.contains_key(&address) {
        return;
    }
    let path = match fields.get("resourceType").map(Vec::as_slice) {
        Some([Value::String(resource_type)]) => Arc::clone(resource_type),
        _ => Arc::from(path),
    };
    for (key, values) in fields.iter() {
        // Skip the allocation for fields with nothing below them.
        let has_children = |value: &Value| {
            matches!(value, Value::Object(..)) || value.fields().is_some_and(|f| !f.is_empty())
        };
        if values.iter().any(has_children) {
            let child = format!("{path}.{key}");
            for value in values {
                index_paths(value, &child, paths);
            }
        }
    }
    if is_object {
        paths.entry(address).or_insert(path);
    }
}
