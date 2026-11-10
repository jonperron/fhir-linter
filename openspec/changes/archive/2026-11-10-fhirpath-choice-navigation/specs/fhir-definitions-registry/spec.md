## ADDED Requirements

### Requirement: Index des variantes d'éléments choix

Le `Registry` SHALL construire au chargement un index associant le nom de base de chaque élément `*.base[x]` aux codes de type déclarés, capitalisés, et SHALL exposer `is_choice_variant(base: &str, suffix: &str) -> bool`. La correspondance SHALL se faire par nom de base, indépendamment du chemin de l'élément (raccourci documenté).

#### Scenario: Variante déclarée

- **WHEN** `is_choice_variant("value", "Quantity")` est appelé sur le Registry R5
- **THEN** le résultat SHALL être `true` (`Observation.value[x]` déclare `Quantity`)

#### Scenario: Correspondance par nom, pas par chemin

- **WHEN** `is_choice_variant("dose", "Quantity")` est appelé
- **THEN** le résultat SHALL être `true` (grâce à `Dosage.doseAndRate.dose[x]`), si bien que `dose` trouve aussi `Immunization.doseQuantity`, qui n'est pas un élément choix

#### Scenario: Élément non-choix

- **WHEN** `is_choice_variant("modifier", "Extension")` est appelé
- **THEN** le résultat SHALL être `false`
