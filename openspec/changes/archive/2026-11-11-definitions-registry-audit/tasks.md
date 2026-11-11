## 1. Implémentation

- [x] 1.1 Retirer `base_definition` de `StructureDefinition` et de son constructeur
- [x] 1.2 Collecter les concepts `notSelectable`/`abstract` dans `CodeSystem.not_selectable`
- [x] 1.3 Charger `Include.version` et `CodeSystem.version` et les appliquer dans `value_set_contains`
- [x] 1.4 Rejeter les éléments de snapshot sans `path` ou `max` avec `Error::MalformedBundle`
- [x] 1.5 Adapter les appels à `StructureDefinition::new` dans `fhir-validator`

## 2. Tests

- [x] 2.1 Tests de régression dans `crates/fhir-definitions/src/{loader,registry}.rs`
