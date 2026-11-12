## MODIFIED Requirements

### Requirement: Typage des primitives par le type déclaré

Le système SHALL exposer `resource_to_typed_value(resource, child_element: &ChildElement) -> Value`, où `ChildElement` est `dyn Fn(&str, &str) -> Option<(Arc<str>, Arc<str>)>` (chemin de définition du parent, nom de l'enfant) vers (chemin de définition, code de type). Une chaîne dont le type déclaré est `date` SHALL devenir `Value::Date`, `dateTime` et `instant` `Value::DateTime`, `time` `Value::Time` (FHIRPath R5 §2.1.9.1). Une ressource imbriquée (contained, entrée de Bundle) SHALL être typée par son propre `resourceType`. Le système SHALL aussi exposer `node_to_typed_value(value, element, child, child_element)`, qui convertit un nœud et son élément JSON `_nom` avec le même typage que `resource_to_typed_value`. `resource_to_value` et `from_parser_value` SHALL rester non typées (les dates restent des `String`).

#### Scenario: Date typée par le Registry

- **WHEN** `birthDate` est évalué sur un Patient converti par `resource_to_typed_value` avec `Registry::child_element`
- **THEN** le résultat SHALL être `[Date(...)]` et `birthDate < @2000-01-01` SHALL être comparé comme une date

#### Scenario: Conversion non typée

- **WHEN** le même Patient est converti par `resource_to_value`
- **THEN** `birthDate` SHALL être une `String`

#### Scenario: Conversion d'un nœud

- **WHEN** un nœud et son `_nom` sont convertis par `node_to_typed_value`
- **THEN** le résultat SHALL être typé comme par `resource_to_typed_value`
