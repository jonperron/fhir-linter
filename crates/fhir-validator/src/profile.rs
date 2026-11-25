use fhir_definitions::{Constraint, ElementDefinition, Registry, StructureDefinition};
use fhir_fhirpath::{Environment, Value as FhirPathValue};
use fhir_parser::{Node, Resource, Span, Value};
use indexmap::IndexMap;
use std::cell::RefCell;
use std::sync::Arc;

use crate::cardinality::{check_run_cardinality, own_descendants, subtree, without_own_slices};
use crate::diagnostic::{Diagnostic, Severity};
use crate::fixed_pattern::{self, occurrences};
use crate::invariants;
use crate::resolve::{canonical_parts, complex_type};
use crate::slicing::{self, Assigned, Item};
use fhir_definitions::capitalise;

/// Validate the profiles declared in `meta.profile`.
///
/// Profiles known to the registry are checked for resource-type match
/// (`PROFILE_002`) and their cardinality, `fixed[x]`/`pattern[x]` and value-slice rules are
/// applied at every depth (`check_profile_object`); `environment` evaluates discriminators and
/// slice invariants. Unknown profiles produce a `PROFILE_001` warning because they cannot be
/// validated against; a profile supplied without a snapshot (`Registry::needs_snapshot`) gives
/// a `PROFILE_010` error. A `|version` other than the loaded definition's gives a
/// `PROFILE_003` warning.
/// `resource_path` is the resource's instance path.
pub fn validate(
    resource: &Resource,
    base: &StructureDefinition,
    registry: &Registry,
    environment: &Environment<'_>,
    resource_path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let layer = Layer {
        registry,
        environment,
    };
    let resource_type = resource.resource_type.as_ref();
    let Some(Value::Object(meta)) = resource.fields.get("meta").map(|n| &n.value) else {
        return;
    };
    // The XML reshape keeps a single `<profile>` scalar; it is still the first profile.
    let profiles = match meta.get("profile") {
        Some(Node {
            value: Value::Array(items),
            ..
        }) => items.as_slice(),
        Some(node) => std::slice::from_ref(node),
        None => return,
    };

    for (i, node) in profiles.iter().enumerate() {
        let Value::Str(url) = &node.value else {
            continue;
        };
        let path = format!("{resource_path}.meta.profile[{i}]");
        let (canonical, version) = canonical_parts(url);
        let Some(sd) = registry.structure_definition(canonical) else {
            let diagnostic = if registry.needs_snapshot(canonical) {
                no_snapshot(url, path)
            } else {
                Diagnostic::warning("PROFILE_001", format!("Profile '{url}' is not known"), path)
            };
            diagnostics.push(diagnostic.with_location(node.span));
            continue;
        };
        if let Some(warning) = version_mismatch(url, version, sd, &path) {
            diagnostics.push(warning.with_location(node.span));
        }
        let profile_type = sd.snapshot.first().map(|e| e.path.as_ref());
        // Profiles on abstract parents (e.g. DomainResource) apply to every subtype.
        if profile_type != Some(resource_type) && !sd.is_abstract {
            diagnostics.push(
                Diagnostic::error(
                    "PROFILE_002",
                    format!("Profile '{url}' does not apply to resource type '{resource_type}'"),
                    path,
                )
                .with_location(node.span),
            );
        } else if sd.url != base.url {
            // Keep only findings the base check did not already report.
            let mut found = Vec::new();
            if let (Some(root), Some(value)) = (sd.snapshot.first(), environment.resource) {
                let base_root = base.element(resource_type);
                check_root_invariants(
                    root,
                    std::slice::from_ref(value),
                    &|c| has_constraint(base_root, c),
                    (resource_path, None),
                    layer,
                    &mut found,
                );
            }
            check_profile_object(
                &resource.fields,
                sd,
                own_descendants(sd, resource_type),
                resource_type,
                resource_path,
                layer,
                &mut found,
            );
            found.retain(|f| {
                !diagnostics
                    .iter()
                    .any(|d| d.code == f.code && d.path == f.path)
            });
            diagnostics.extend(found);
        }
    }
}

/// Apply the profiles `elem` declares for type `ty` (`type.profile`, such as SimpleQuantity) to
/// one object (`fields`, its FHIRPath value and span): the profile root's own invariants
/// (`sqty-1`) and its rules below the root ([`check_profile_object`]). The value must conform
/// to one of them; findings the base walk of the node (from `start`) already made are dropped.
/// `environment` evaluates the invariants.
#[allow(clippy::too_many_arguments)] // the node, its definition, its place and the walk state
pub(crate) fn check_type_profiles(
    (fields, value, span): (&IndexMap<Arc<str>, Node>, &[FhirPathValue], Span),
    elem: &ElementDefinition,
    ty: &str,
    path: &str,
    registry: &Registry,
    environment: &Environment<'_>,
    start: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(urls) = elem.type_profiles.iter().find(|t| t.code.as_ref() == ty) else {
        return;
    };
    let base = complex_type(ty, registry)
        .and_then(|type_sd| type_sd.element(ty))
        .map(|root| root.constraints.as_slice())
        .unwrap_or_default();
    let mut results = Vec::with_capacity(urls.profiles.len());
    for url in &urls.profiles {
        let (canonical, version) = canonical_parts(url);
        let Some((profile, root)) = registry
            .structure_definition(canonical)
            .and_then(|profile| Some((profile, profile.snapshot.first()?)))
        else {
            diagnostics.push(if registry.needs_snapshot(canonical) {
                no_snapshot(url, path.to_owned())
            } else {
                let message = format!("Profile '{url}' of '{path}' is not known");
                Diagnostic::warning("PROFILE_001", message, path.to_owned())
            });
            continue;
        };
        diagnostics.extend(version_mismatch(url, version, profile, path));
        let mut found = Vec::new();
        let known = |c: &Constraint| {
            base.iter()
                .chain(&elem.constraints)
                .any(|b| same_constraint(b, c))
        };
        let layer = Layer {
            registry,
            environment,
        };
        check_root_invariants(root, value, &known, (path, Some(span)), layer, &mut found);
        let run = own_descendants(profile, &root.path);
        check_profile_object(fields, profile, run, &root.path, path, layer, &mut found);
        found.retain(|f| {
            !diagnostics[start..]
                .iter()
                .any(|d| d.code == f.code && d.path == f.path)
        });
        results.push(found);
    }
    // type.profile: conforming to any one of the profiles is enough.
    let is_error = |d: &Diagnostic| d.severity == Severity::Error;
    let chosen = match results.iter().position(|r| !r.iter().any(is_error)) {
        Some(i) => results.swap_remove(i),
        None => results.into_iter().next().unwrap_or_default(),
    };
    diagnostics.extend(chosen);
}

/// Apply the rules of profile `sd` below `def_path` to one object (instance `path`), at every
/// depth: child cardinality, `fixed[x]`/`pattern[x]` and slicing. The items a slice matches
/// also get the slice's own `fixed[x]`/`pattern[x]`, invariants and children.
pub(crate) fn check_profile_object(
    fields: &IndexMap<Arc<str>, Node>,
    sd: &StructureDefinition,
    run: &[ElementDefinition],
    def_path: &str,
    path: &str,
    layer: Layer<'_, '_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    check_run_cardinality(fields, sd, run, def_path, path, diagnostics);
    let prefix = format!("{def_path}.");
    for (k, elem) in run.iter().enumerate() {
        let Some(rest) = elem.path.strip_prefix(&prefix).filter(|r| !r.contains('.')) else {
            continue;
        };
        if elem.slice_name.is_some() {
            continue;
        }
        let start = diagnostics.len();
        let subtree = subtree(run, k);
        let (present, items): (Vec<(&Arc<str>, &Node)>, Vec<Item>) = match rest.strip_suffix("[x]")
        {
            Some(base) => {
                // Every variant the base element allows is an item of the slicing; the rules
                // below apply to the variants the profile keeps.
                let types = choice_types(elem, layer.registry);
                let variants: Vec<(&Arc<str>, &Node, &str)> = fields
                    .iter()
                    .filter_map(|(key, node)| {
                        let suffix = key.strip_prefix(base)?;
                        let ty = types.iter().find(|t| capitalise(t) == suffix)?;
                        Some((key, node, ty.as_ref()))
                    })
                    .collect();
                let present = variants
                    .iter()
                    .filter(|(_, _, ty)| elem.types.iter().any(|t| &**t == *ty))
                    .map(|&(key, node, _)| (key, node))
                    .collect();
                let items = variants
                    .iter()
                    .map(|&(key, node, ty)| Item {
                        node,
                        path: format!("{path}.{key}"),
                        choice_type: Some(ty),
                    })
                    .collect();
                (present, items)
            }
            None => {
                let child_path = format!("{path}.{rest}");
                let items = fields
                    .get(rest)
                    .map(|node| occurrences(node, &child_path))
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(node, path)| Item {
                        node,
                        path,
                        choice_type: None,
                    })
                    .collect();
                (fields.get_key_value(rest).into_iter().collect(), items)
            }
        };
        check_own_invariants(&items, elem, layer, diagnostics);
        let own = without_own_slices(subtree, &elem.path);
        for (name, node) in present {
            let child_path = format!("{path}.{name}");
            fixed_pattern::check_value(node, elem, &child_path, diagnostics);
            if elem.content_reference.is_some() {
                continue;
            }
            for (item, item_path) in occurrences(node, &child_path) {
                if let Value::Object(fields) = &item.value {
                    check_profile_object(
                        fields,
                        sd,
                        own,
                        &elem.path,
                        &item_path,
                        layer,
                        diagnostics,
                    );
                }
            }
        }
        let child_path = format!("{path}.{rest}");
        let mut found = Vec::new();
        check_slices(&items, (elem, subtree), &child_path, sd, layer, &mut found);
        // A slice repeats rules of the sliced element: keep each finding once.
        found.retain(|f| {
            !diagnostics[start..]
                .iter()
                .any(|d| d.code == f.code && d.path == f.path)
        });
        diagnostics.extend(found);
    }
}

/// `PROFILE_003` when the canonical reference `url` asks for a `version` other than the
/// loaded definition `sd`'s. The spec is silent on a version mismatch: warn, then validate
/// against the loaded version (user decision, PLAN 11.7 V10).
fn version_mismatch(
    url: &str,
    version: Option<&str>,
    sd: &StructureDefinition,
    path: &str,
) -> Option<Diagnostic> {
    let version = version.filter(|v| sd.version.as_deref() != Some(*v))?;
    let loaded = sd.version.as_deref().unwrap_or("none");
    Some(Diagnostic::warning(
        "PROFILE_003",
        format!(
            "Profile '{url}' asks for version '{version}' but version '{loaded}' is loaded; \
             validated against the loaded version"
        ),
        path.to_owned(),
    ))
}

/// `PROFILE_010`: the profile at `url` was supplied without a snapshot, so it cannot be
/// validated against (snapshot generation is not supported).
fn no_snapshot(url: &str, path: String) -> Diagnostic {
    let message = format!("Profile '{url}' has no snapshot; snapshot required");
    Diagnostic::error("PROFILE_010", message, path)
}

/// What the profile layer reads besides the profile: the registry, and the FHIRPath
/// environment of the resource for discriminators and slice invariants.
#[derive(Clone, Copy)]
pub(crate) struct Layer<'a, 'e> {
    pub registry: &'a Registry,
    pub environment: &'a Environment<'e>,
}

/// Assign the `items` of the sliced element `elem` (snapshot `run` below it, instance `path`)
/// to their slices ([`slicing::assign`]), then apply to each item its slice's own
/// `fixed[x]`/`pattern[x]`, the invariants the sliced element does not already have, and the
/// slice's children.
fn check_slices(
    items: &[Item<'_>],
    (elem, run): (&ElementDefinition, &[ElementDefinition]),
    path: &str,
    sd: &StructureDefinition,
    layer: Layer<'_, '_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Conformance to the profile's own rules (root and element invariants included); the
    // rules of the core type are the base walk's.
    // Slice profiles supplied without a snapshot, reported once the assignment is done.
    let snapshotless = RefCell::new(Vec::<String>::new());
    // Slice profiles of another version than the loaded one, also reported once.
    let mismatched = RefCell::new(Vec::<Diagnostic>::new());
    let conforms = |fields: &IndexMap<Arc<str>, Node>, value: &[FhirPathValue], url: &str| {
        let (canonical, version) = canonical_parts(url);
        let Some(profile) = layer.registry.structure_definition(canonical) else {
            let mut snapshotless = snapshotless.borrow_mut();
            if layer.registry.needs_snapshot(canonical) && !snapshotless.iter().any(|u| u == url) {
                snapshotless.push(url.to_owned());
            }
            return None;
        };
        if let Some(warning) = version_mismatch(url, version, profile, path) {
            let mut mismatched = mismatched.borrow_mut();
            if !mismatched.iter().any(|m| m.message == warning.message) {
                mismatched.push(warning);
            }
        }
        let root = profile.snapshot.first()?;
        let core_root = root
            .path
            .split('.')
            .next()
            .and_then(|t| layer.registry.core(t))
            .and_then(|sd| sd.element(&root.path));
        let run = own_descendants(profile, &root.path);
        let mut found = Vec::new();
        // The sliced element's own constraints are checked on the item already.
        let known = |c: &Constraint| has_constraint(core_root, c) || has_constraint(Some(elem), c);
        check_root_invariants(root, value, &known, (path, None), layer, &mut found);
        check_profile_object(fields, profile, run, &root.path, path, layer, &mut found);
        Some(!found.iter().any(|d| d.severity == Severity::Error))
    };
    let assigned = slicing::assign(
        items,
        elem,
        run,
        path,
        layer.registry,
        layer.environment,
        &conforms,
        diagnostics,
    );
    for url in snapshotless.into_inner() {
        diagnostics.push(no_snapshot(&url, path.to_owned()));
    }
    diagnostics.extend(mismatched.into_inner());
    let is_choice_variant =
        |base: &str, suffix: &str| layer.registry.is_choice_variant(base, suffix);
    for (item, assigned) in items.iter().zip(assigned) {
        let Some(Assigned {
            slice,
            children,
            value,
        }) = assigned
        else {
            continue;
        };
        fixed_pattern::check_value(item.node, slice, &item.path, diagnostics);
        let known = |key: &str| elem.constraints.iter().any(|c| *c.key == *key);
        for constraint in slice.constraints.iter().filter(|c| !known(&c.key)) {
            invariants::check(
                constraint,
                &value,
                &is_choice_variant,
                layer.environment,
                &item.path,
                Some(item.node.span),
                diagnostics,
            );
        }
        if let Value::Object(fields) = &item.node.value {
            check_profile_object(
                fields,
                sd,
                children,
                &slice.path,
                &item.path,
                layer,
                diagnostics,
            );
        }
    }
}

/// Evaluate on `value` (at instance `path`, `span`) the constraints of the profile root `root`
/// that are not `known`, the ones another layer already checks.
fn check_root_invariants(
    root: &ElementDefinition,
    value: &[FhirPathValue],
    known: &dyn Fn(&Constraint) -> bool,
    (path, span): (&str, Option<Span>),
    layer: Layer<'_, '_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for constraint in root.constraints.iter().filter(|c| !known(c)) {
        invariants::check(
            constraint,
            value,
            &|base, suffix| layer.registry.is_choice_variant(base, suffix),
            layer.environment,
            path,
            span,
            diagnostics,
        );
    }
}

/// Evaluate on each item of the profile element `elem` the constraints the profile adds: the
/// keys of the base element and of the item type's root are skipped, the base walk checks them.
/// A choice variant the profile does not keep has no constraints of `elem` to meet.
fn check_own_invariants(
    items: &[Item<'_>],
    elem: &ElementDefinition,
    layer: Layer<'_, '_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if elem.constraints.is_empty() {
        return;
    }
    let registry = layer.registry;
    let base = elem
        .path
        .split('.')
        .next()
        .and_then(|t| registry.core(t))
        .and_then(|sd| sd.element(&elem.path));
    let is_choice_variant = |base: &str, suffix: &str| registry.is_choice_variant(base, suffix);
    for item in items {
        // A contentReference element has no type: it is a backbone element (as in item_value).
        let ty = item
            .choice_type
            .or_else(|| elem.types.first().map(|t| t.as_ref()))
            .unwrap_or("BackboneElement");
        if item.choice_type.is_some() && !elem.types.iter().any(|t| &**t == ty) {
            continue;
        }
        let type_root = registry.core(ty).and_then(|sd| sd.element(ty));
        let own: Vec<_> = elem
            .constraints
            .iter()
            .filter(|c| !has_constraint(base, c) && !has_constraint(type_root, c))
            .collect();
        if own.is_empty() {
            continue;
        }
        let value = slicing::item_value(item, elem, registry);
        // A JSON null is no node to constrain (as in the base walk).
        if value.is_empty() {
            continue;
        }
        for constraint in own {
            invariants::check(
                constraint,
                &value,
                &is_choice_variant,
                layer.environment,
                &item.path,
                Some(item.node.span),
                diagnostics,
            );
        }
    }
}

/// Whether `elem` exists and has `constraint` (same key and expression): a profile that
/// redefines an inherited key with another expression still has its own checked.
fn has_constraint(elem: Option<&ElementDefinition>, constraint: &Constraint) -> bool {
    elem.is_some_and(|e| e.constraints.iter().any(|c| same_constraint(c, constraint)))
}

fn same_constraint(a: &Constraint, b: &Constraint) -> bool {
    a.key == b.key && a.expression == b.expression
}

/// The types the base definition allows for the choice element `elem` (a profile may keep
/// fewer); the profile's own types when the base is not found.
fn choice_types<'a>(elem: &'a ElementDefinition, registry: &'a Registry) -> &'a [Arc<str>] {
    elem.path
        .split('.')
        .next()
        .and_then(|t| registry.core(t))
        .and_then(|base| base.element(&elem.path))
        .map_or(&elem.types, |base| &base.types)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn constraint(key: &str, expression: &str) -> Constraint {
        let human = Arc::from("human");
        Constraint::new(
            Arc::from(key),
            Arc::from("error"),
            human,
            Some(Arc::from(expression)),
        )
    }

    fn definitions() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5")
    }

    /// The registry with `sd` supplied as one file.
    fn supplied(name: &str, sd: &serde_json::Value) -> Registry {
        let dir = std::env::temp_dir().join(format!("profile-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("sd.json");
        std::fs::write(&file, sd.to_string()).unwrap();
        let base = Arc::new(Registry::from_definitions_dir(definitions()).unwrap());
        let registry = Registry::with_extra(base, &[file.as_path()]).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        registry
    }

    /// `(code, severity, message)` of the diagnostics `Patient.maritalStatus` gets (no value)
    /// when its CodeableConcept type names the profile `reference`.
    fn marital_status_profiled(registry: &Registry, reference: &str) -> Vec<Diagnostic> {
        let mut elem = registry
            .core("Patient")
            .and_then(|sd| sd.element("Patient.maritalStatus"))
            .unwrap()
            .clone();
        elem.type_profiles = vec![fhir_definitions::TypeProfiles {
            code: Arc::from("CodeableConcept"),
            profiles: vec![Arc::from(reference)],
            target_profiles: vec![],
        }];
        let mut diagnostics = Vec::new();
        check_type_profiles(
            (&IndexMap::new(), &[], Span::default()),
            &elem,
            "CodeableConcept",
            "Patient.maritalStatus",
            registry,
            &Environment::default(),
            0,
            &mut diagnostics,
        );
        diagnostics
    }

    fn summary(diagnostics: &[Diagnostic]) -> Vec<(&str, Severity, &str)> {
        diagnostics
            .iter()
            .map(|d| (d.code.as_ref(), d.severity, d.message.as_str()))
            .collect()
    }

    const CC: &str = "http://example.org/StructureDefinition/cc";

    /// CodeableConcept with `text` required, at version 1.0, supplied in directory `name`.
    fn cc_profile(name: &str) -> Registry {
        let core = Registry::from_definitions_dir(definitions()).unwrap();
        let mut elements: Vec<serde_json::Value> = core
            .core("CodeableConcept")
            .unwrap()
            .snapshot
            .iter()
            .map(|e| {
                serde_json::json!({
                    "id": &*e.path,
                    "path": &*e.path,
                    "min": e.min,
                    "max": &*e.max,
                })
            })
            .collect();
        for e in &mut elements {
            if e["path"] == "CodeableConcept.text" {
                e["min"] = serde_json::json!(1);
            }
        }
        let sd = serde_json::json!({
            "resourceType": "StructureDefinition",
            "url": CC,
            "version": "1.0",
            "name": "TextCC",
            "status": "draft",
            "kind": "complex-type",
            "abstract": false,
            "type": "CodeableConcept",
            "baseDefinition": "http://hl7.org/fhir/StructureDefinition/CodeableConcept",
            "derivation": "constraint",
            "snapshot": {"element": elements},
        });
        supplied(name, &sd)
    }

    #[test]
    fn type_profile_without_snapshot_is_profile_010() {
        // No core element names a profile the bundle lacks, so only an edited element
        // reaches this path with a supplied snapshotless profile.
        let url = "http://example.org/StructureDefinition/no-snapshot-cc";
        let sd = serde_json::json!({
            "resourceType": "StructureDefinition",
            "url": url,
            "name": "NoSnapshotCC",
            "status": "draft",
            "kind": "complex-type",
            "abstract": false,
            "type": "CodeableConcept",
            "derivation": "constraint",
            "differential": {"element": [{"path": "CodeableConcept"}]},
        });
        let registry = supplied("010-type", &sd);
        for reference in [url.to_owned(), format!("{url}|1.0")] {
            let diagnostics = marital_status_profiled(&registry, &reference);
            // The diagnostic names the reference as written.
            let message = format!("Profile '{reference}' has no snapshot; snapshot required");
            assert_eq!(
                summary(&diagnostics),
                [("PROFILE_010", Severity::Error, message.as_str())]
            );
        }
    }

    #[test]
    fn versioned_type_profile_of_the_loaded_version_is_applied() {
        let registry = cc_profile("cc-loaded");
        let diagnostics = marital_status_profiled(&registry, &format!("{CC}|1.0"));
        let codes: Vec<_> = diagnostics.iter().map(|d| (&*d.code, &*d.path)).collect();
        assert_eq!(codes, [("CARDINALITY_001", "Patient.maritalStatus.text")]);
    }

    #[test]
    fn versioned_type_profile_of_another_version_is_profile_003() {
        let registry = cc_profile("cc-other");
        let reference = format!("{CC}|2.0");
        let diagnostics = marital_status_profiled(&registry, &reference);
        let codes: Vec<_> = diagnostics.iter().map(|d| (&*d.code, &*d.path)).collect();
        // Validated against the loaded version, as for meta.profile.
        assert_eq!(
            codes,
            [
                ("PROFILE_003", "Patient.maritalStatus"),
                ("CARDINALITY_001", "Patient.maritalStatus.text"),
            ]
        );
        assert!(diagnostics[0].message.contains(&format!("'{reference}'")));
        assert_eq!(diagnostics[0].severity, Severity::Warning);
    }

    #[test]
    fn a_redefined_expression_is_another_constraint() {
        let base = constraint("vs-2", "value.exists()");
        assert!(same_constraint(
            &base,
            &constraint("vs-2", "value.exists()")
        ));
        assert!(!same_constraint(
            &base,
            &constraint("vs-2", "value.exists() and code.exists()")
        ));
        assert!(!same_constraint(
            &base,
            &constraint("vs-3", "value.exists()")
        ));
    }
}
