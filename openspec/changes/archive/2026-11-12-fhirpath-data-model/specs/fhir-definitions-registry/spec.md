## ADDED Requirements

### Requirement: Élément enfant typé

Le `Registry` SHALL exposer `child_element(parent: &str, name: &str) -> Option<(Arc<str>, Arc<str>)>`, où `parent` est un chemin de définition (nom de type comme `Patient` ou `HumanName`, ou chemin d'élément backbone comme `Patient.contact`). Il SHALL retourner le chemin de définition de l'enfant et son code de type : le chemin de l'élément lui-même pour `BackboneElement` et `Element`, sinon le code de type. Pour un élément choix, le suffixe capitalisé SHALL choisir le type parmi les types déclarés de `base[x]`. Il SHALL retourner `None` pour un parent, un élément ou un suffixe inconnu, et pour un élément sans type ; les éléments `contentReference` ne sont pas suivis (raccourci documenté).

#### Scenario: Primitive et backbone

- **WHEN** `child_element("Patient", "birthDate")` puis `child_element("Patient", "contact")` sont appelés
- **THEN** les résultats SHALL être `("date", "date")` puis `("Patient.contact", "BackboneElement")`

#### Scenario: Variante d'un élément choix

- **WHEN** `child_element("Observation", "valueTime")` puis `child_element("Observation", "valueBogus")` sont appelés
- **THEN** les résultats SHALL être `("time", "time")` puis `None`
