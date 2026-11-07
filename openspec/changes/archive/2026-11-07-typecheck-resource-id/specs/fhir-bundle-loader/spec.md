## ADDED Requirements

### Requirement: Résolution des types système FHIRPath

Lors du parsing d'un `ElementDefinition`, le système SHALL remplacer tout code de type commençant par `http://hl7.org/fhirpath/` (par ex. `http://hl7.org/fhirpath/System.String`) par la valeur `valueUrl` de l'extension `http://hl7.org/fhir/StructureDefinition/structuredefinition-fhir-type` de ce type. En l'absence de cette extension, le code d'origine SHALL être conservé. Les autres codes SHALL être conservés tels quels.

#### Scenario: Type système avec extension fhir-type

- **WHEN** un élément `Patient.id` a un type de code `http://hl7.org/fhirpath/System.String` portant l'extension `structuredefinition-fhir-type` avec `valueUrl` `"id"`
- **THEN** le type parsé de l'élément SHALL être `"id"`

#### Scenario: Type système sans extension

- **WHEN** un type a le code `http://hl7.org/fhirpath/System.String` sans extension `structuredefinition-fhir-type`
- **THEN** le type parsé SHALL rester `http://hl7.org/fhirpath/System.String`
