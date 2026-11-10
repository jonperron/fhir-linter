## Why

Audit 11.3-F1 : FHIRPath R5 §2.1.9.1.1 impose qu'un élément choix (`value[x]`) soit navigable par son nom de base (`value` trouve `valueQuantity`). L'évaluateur l'ignorait, d'où de faux échecs pour fhs-2 et mrp-2 et un obs-6 qui ne se déclenchait jamais.

## What Changes

- `fhir-fhirpath` : nouvelle API `evaluate_parsed_with_choices(expr, context, is_choice_variant)` ; `evaluate` / `evaluate_parsed` inchangées.
- `fhir-definitions` : `Registry::is_choice_variant(base, suffix)`, adossé à un index construit au chargement à partir des éléments `*.base[x]`.
- `fhir-validator` : les invariants sont évalués avec la navigation des choix.

## Capabilities

### New Capabilities

(aucune)

### Modified Capabilities

- `fhir-fhirpath` : navigation des éléments choix.
- `fhir-definitions-registry` : index des variantes de choix.
- `fhir-validator` : invariants évalués avec navigation des choix.

## Impact

Code : `crates/fhir-fhirpath/src/{lib.rs,eval/mod.rs}`, `crates/fhir-definitions/src/registry.rs`, `crates/fhir-validator/src/{invariants.rs,lib.rs}`. Pas de changement cassant d'API.
