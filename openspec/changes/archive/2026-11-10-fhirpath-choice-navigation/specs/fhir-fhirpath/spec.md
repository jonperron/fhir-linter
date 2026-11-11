## ADDED Requirements

### Requirement: Navigation des éléments choix

Le système SHALL exposer `evaluate_parsed_with_choices(expr: &Expr, context: &[Value], is_choice_variant: &dyn Fn(&str, &str) -> bool) -> Result<Collection, EvalError>`. Lorsqu'un identifiant `name` est absent d'un objet, l'évaluation SHALL retourner les valeurs des champs `name` + suffixe capitalisé pour chaque suffixe tel que `is_choice_variant(name, suffix)` est vrai (FHIRPath R5 §2.1.9.1.1). Une clé explicite `name` SHALL être prioritaire et les clés commençant par `_` ne SHALL jamais correspondre. `evaluate` et `evaluate_parsed` SHALL conserver leur comportement sans navigation des choix.

#### Scenario: Le nom de base trouve la variante

- **WHEN** `value` est évalué avec `evaluate_parsed_with_choices` sur `{"valueQuantity": {...}}` et que `is_choice_variant("value", "Quantity")` est vrai
- **THEN** le résultat SHALL contenir la valeur de `valueQuantity`

#### Scenario: Clé explicite prioritaire

- **WHEN** l'objet contient à la fois `value` et `valueQuantity`
- **THEN** seul le champ `value` SHALL être retourné

#### Scenario: Suffixe non déclaré ignoré

- **WHEN** `modifier` est évalué sur un objet contenant `modifierExtension` et que `is_choice_variant("modifier", "Extension")` est faux
- **THEN** le résultat SHALL être vide

#### Scenario: Élément primitif étendu ignoré

- **WHEN** l'objet ne contient que `_valueString`
- **THEN** `value` SHALL retourner une collection vide

#### Scenario: evaluate sans navigation des choix

- **WHEN** `evaluate("value", ...)` est appelé sur `{"valueQuantity": {...}}`
- **THEN** le résultat SHALL être vide
