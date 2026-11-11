## Why

Trois écarts entre la spécification et le comportement XML : le Warning d'une valeur d'attribut épurée partageait le code `TYPE_006` réservé aux caractères de contrôle (#68) ; le namespace XMLSchema-instance faisait échouer `parse_xml` alors qu'il relève d'un diagnostic (xml.html 2.1.6.3.4) ; les références de caractère de contrôle dans le texte brut du div narratif n'étaient pas vérifiées en XML 1.0.

## What Changes

- Le Warning de valeur d'attribut XML épurée devient `TYPE_007` ; `TYPE_006` reste aux caractères de contrôle.
- `parse_xml` n'échoue plus sur le namespace XMLSchema-instance : chaque élément concerné est enregistré une fois dans `Resource::schema_instance: Vec<Span>` (vide pour le JSON), et le validateur émet `STRUCTURE_003` (Error) par élément enregistré.
- La règle XML 1.0 des caractères de contrôle résout aussi les références de caractère du texte brut du div narratif.

## Capabilities

### New Capabilities

(aucune)

### Modified Capabilities

- `fhir-xml-parser` : namespace schema instance enregistré au lieu d'une erreur ; références de caractère du div vérifiées.
- `fhir-validator` : `TYPE_007` pour les valeurs épurées ; nouveau `STRUCTURE_003`.

## Impact

- `crates/fhir-parser` (`types.rs`, `xml.rs`, `xml_checks.rs`, `json.rs`) : nouveau champ public `Resource::schema_instance`.
- `crates/fhir-validator/src/xml_shape.rs` : `TYPE_007`, `STRUCTURE_003`.
- Les consommateurs qui filtraient `TYPE_006` pour les valeurs épurées doivent filtrer `TYPE_007`.
