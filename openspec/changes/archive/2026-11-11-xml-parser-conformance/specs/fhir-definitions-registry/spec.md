## ADDED Requirements

### Requirement: Représentation XML d'un ElementDefinition

Chaque `ElementDefinition` SHALL exposer un champ `xml_attr: bool`, vrai lorsque sa `representation` contient `xmlAttr`, faux sinon.

#### Scenario: Élément représenté en attribut

- **WHEN** l'élément `Extension.url` est chargé depuis les définitions R5
- **THEN** son `xml_attr` SHALL être vrai

#### Scenario: Élément ordinaire

- **WHEN** l'élément `Patient.name` est chargé
- **THEN** son `xml_attr` SHALL être faux
