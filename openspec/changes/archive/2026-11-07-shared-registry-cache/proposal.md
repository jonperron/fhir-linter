## Why

Les bindings Python et Node dupliquaient chacun un cache de `Registry` par répertoire et la conversion `Severity` → chaîne. Ces deux mécanismes sont remontés dans les crates Rust pour n'exister qu'une fois.

## What Changes

- Nouvelle fonction publique `Registry::shared(dir) -> Result<Arc<Registry>>` dans `fhir-definitions` : un seul `Arc` par répertoire canonicalisé, à l'échelle du processus
- Nouvelle méthode publique `Severity::as_str()` dans `fhir-validator`
- Les bindings Python et Node utilisent ces deux API ; leur comportement et leur API publique sont inchangés

## Capabilities

### New Capabilities

Aucune.

### Modified Capabilities

- `fhir-definitions-registry` : ajout de `Registry::shared`
- `fhir-validator` : ajout de `Severity::as_str`

## Impact

- `crates/fhir-definitions/src/registry.rs`, `crates/fhir-validator/src/diagnostic.rs`
- `bindings/python/src/lib.rs`, `bindings/typescript/src/lib.rs`
- Aucun changement d'API côté bindings
