## Why

L'`id` de ressource est extrait des champs par le parseur ; la règle `TYPE_006` (caractère de contrôle) ne le voyait donc jamais, alors que la contrainte de FHIR R5 (datatypes.html) s'applique aussi à l'`id` racine.

## What Changes

- `validate` (nested.rs) émet désormais `TYPE_006` au chemin `<Type>.id`, sans localisation source, pour un `id` de ressource contenant un caractère de contrôle, en plus de `TYPE_005`.

## Capabilities

### New Capabilities

Aucune.

### Modified Capabilities

- `fhir-validator` : requirement « Validation de type — caractère de contrôle dans une chaîne (TYPE_006) ».

## Impact

- `crates/fhir-validator/src/nested.rs`, `crates/fhir-validator/src/type_check.rs`.
- Un diagnostic `Warning` supplémentaire pour les ressources dont l'`id` contient un caractère de contrôle.
