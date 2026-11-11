## MODIFIED Requirements

### Requirement: StructureDefinition type

Le système SHALL exposer un type `StructureDefinition` représentant une définition de profil FHIR R5. Il SHALL NOT exposer de champ `base_definition`.

#### Scenario: Champs obligatoires présents

- **WHEN** une `StructureDefinition` est construite
- **THEN** elle SHALL contenir les champs : `url` (URL canonique), `name` (nom logique), `kind` (catégorie : `primitive-type`, `complex-type`, `resource`, `logical`), `is_abstract` (booléen), `snapshot` (liste d'`ElementDefinition`)
