## Why

Plusieurs cas limites du parse JSON/XML s'écartaient de FHIR R5 : `-0` était accepté comme `integer`, un BOM UTF-8 initial faisait échouer le parse JSON, et les caractères de contrôle sous U+0020 n'étaient signalés ni en JSON ni en XML. La signification de `col` et `offset` (octets UTF-8) n'était pas documentée côté bindings.

## What Changes

- `-0` JSON devient `Value::Decimal(-0.0)` ; un champ `integer` contenant `-0` produit `TYPE_001`.
- Un BOM UTF-8 initial est ignoré par `parse_json` ; les spans comptent ses 3 octets. Un BOM final ou un second BOM reste une erreur.
- Nouveau diagnostic `TYPE_006` (Warning) pour un `string`/`markdown`/`code`/`id` contenant un caractère de contrôle sous U+0020 autre que tabulation, CR, LF.
- `parse_xml` rejette un caractère de contrôle littéral (XML 1.0 et 1.1) et, en XML 1.0, une référence de caractère (`&#1;`) vers un tel caractère dans une valeur capturée.
- `Span.col` et `Span.offset` sont documentés comme des comptes d'octets UTF-8 ; les bindings Python et Node le précisent.

## Capabilities

### New Capabilities

Aucune.

### Modified Capabilities

- `fhir-json-parser` : Type Span, Parse JSON, Conversion des nombres JSON.
- `fhir-validator` : TYPE_001 (scénario `-0`), ajout de TYPE_006.
- `fhir-xml-parser` : Parse XML (caractères de contrôle, version XML).
- `fhir-python-bindings` : Type Diagnostic (unités de `col`/`offset`).
- `fhir-node-bindings` : Type Diagnostic (unités de `col`/`offset`).

## Impact

- `crates/fhir-parser/src/{json.rs,xml.rs,lib.rs,line_index.rs,types.rs}`
- `crates/fhir-validator/src/type_check.rs`, `crates/fhir-validator/tests/control_chars_tests.rs`
- `bindings/python/{README.md,src/lib.rs}`, `bindings/typescript/src/lib.rs`
- Fixtures officielles : `unicode-problem.xml` devient fatal, `unicode-problem.json` et `xml_v11.xml` produisent un warning.
