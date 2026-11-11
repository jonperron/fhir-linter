## 1. Parseur JSON

- [x] 1.1 `-0` produit `Value::Decimal(-0.0)`
- [x] 1.2 Ignorer un BOM UTF-8 initial, offsets inchangés
- [x] 1.3 Documenter `col`/`offset` en octets UTF-8

## 2. Parseur XML

- [x] 2.1 Rejeter un caractère de contrôle littéral
- [x] 2.2 Rejeter les références de contrôle en XML 1.0 dans les valeurs capturées

## 3. Validateur

- [x] 3.1 Émettre `TYPE_006` pour `string`, `markdown`, `code`, `id`
- [x] 3.2 Tests dans `crates/fhir-validator/tests/control_chars_tests.rs`

## 4. Bindings

- [x] 4.1 Documenter les unités de `col`/`offset` (Python et Node)
