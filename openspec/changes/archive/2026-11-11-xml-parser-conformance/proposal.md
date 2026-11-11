## Why

Le parseur XML acceptait des documents que la spécification FHIR XML (xml.html) interdit (DTD, encodage non UTF-8, texte entre éléments, mauvais namespaces, attribut au lieu d'élément) et réduisait le div narratif à son texte, ce qui empêchait `narrative-absent` de voir une image.

## What Changes

- `parse_xml` rejette par `XmlError` les constructions interdites par xml.html (DTD, encodage, texte/entités/CDATA entre éléments, attributs non autorisés, namespaces, plus de 8 déclarations de namespace, profondeur dans le div).
- Le div narratif est conservé comme sa tranche de source brute.
- Les valeurs d'attribut sont épurées des blancs XML en tête/fin et enregistrées dans `Resource::trimmed_attributes`.
- Le remodelage XML du validateur signale `STRUCTURE_001` pour un élément dont la forme attribut/élément contredit `representation`, et `TYPE_006` (Warning) pour une valeur d'attribut épurée.
- `ElementDefinition` expose `xml_attr`.
- `narrative-absent` lit le balisage du div en JSON comme en XML.

## Capabilities

### New Capabilities

Aucune.

### Modified Capabilities

- `fhir-xml-parser` : conformité xml.html, div brut, épuration des attributs.
- `fhir-validator` : forme attribut/élément et `TYPE_006` au remodelage XML.
- `fhir-definitions-registry` : `ElementDefinition.xml_attr`.
- `fhir-linter` : div XML analysé comme XHTML.

## Impact

`crates/fhir-parser` (xml.rs, xml_checks.rs, types.rs), `crates/fhir-validator` (xml_shape.rs), `crates/fhir-definitions` (model.rs, loader.rs), `crates/fhir-linter` (rules.rs). Des documents XML auparavant acceptés sont désormais rejetés.
