## MODIFIED Requirements

### Requirement: ElementDefinition type

Le système SHALL exposer un type `ElementDefinition` représentant un élément dans le snapshot d'une `StructureDefinition`.

#### Scenario: Champs d'une ElementDefinition

- **WHEN** une `ElementDefinition` est construite
- **THEN** elle SHALL contenir : `path` (chemin FHIRPath), `min` (cardinalité minimale, entier non négatif), `max` (cardinalité maximale : `"*"` ou entier représenté en string), `types` (liste des codes de type FHIR)

#### Scenario: Champs de définition étendus

- **WHEN** une `ElementDefinition` est construite
- **THEN** elle SHALL aussi contenir : `content_reference`, `type_profiles`, `fixed`, `pattern`, `min_value`, `max_value`, `slicing` et `max_length`, décrits dans la spec fhir-definitions-registry
