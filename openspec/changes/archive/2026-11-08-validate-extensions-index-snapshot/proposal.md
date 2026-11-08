## Why

Le parcours imbriqué ignorait `extension` et `modifierExtension` par leur nom :
une extension sans `url`, un `value[x]` mal typé ou un `modifierExtension`
interdit n'étaient jamais signalés. Une valeur scalaire à la place d'un objet
complexe passait aussi silencieusement. Par ailleurs, la recherche d'éléments du
snapshot était linéaire à chaque nœud.

## What Changes

- `extension` / `modifierExtension` sont typés `Extension` et parcourus comme
  tout autre élément, à toute profondeur ; seuls les `_nom` restent ignorés.
- Un `extension` / `modifierExtension` absent du snapshot (ex. `Bundle.extension`,
  `Patient.name[0].modifierExtension`) produit `STRUCTURE_001`.
- Un scalaire pour un type complexe, `BackboneElement` ou `Element` produit
  `TYPE_001` « Expected <type> object at '<path>' ».
- `StructureDefinition::new` indexe le snapshot par chemin ; `element(path)` et
  `descendants(path)` exposent la recherche (aucun changement de comportement).

## Capabilities

### Modified Capabilities

- `fhir-validator` : STRUCTURE_001, TYPE_001 et validation des éléments imbriqués.
- `fhir-definitions-registry` : données snapshot de StructureDefinition.

## Impact

- `crates/fhir-validator/src/nested.rs`, `crates/fhir-definitions/src/model.rs`.
- Davantage de diagnostics sur des ressources auparavant acceptées à tort.
