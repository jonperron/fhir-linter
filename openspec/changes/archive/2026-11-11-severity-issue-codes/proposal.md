## Why

Les sévérités du validateur et du linter (`Error`, `Warning`, `Information`, `Hint`) ne correspondaient pas au ValueSet FHIR R5 IssueSeverity (`fatal | error | warning | information | success`) : `hint` n'existe pas en FHIR, `fatal` manquait, et les invariants de severity `information` (hors du binding requis constraint-severity `error | warning`) étaient rétrogradés en Warning.

## What Changes

- **BREAKING** `Severity` devient `Fatal`, `Error`, `Warning`, `Information` (suppression de `Hint`), ordonnés `Fatal < Error < Warning < Information`.
- Ajout de `Severity::ALL` et de `FromStr` (inverse exact de `as_str`).
- Invariants : seule la severity de contrainte `warning` donne `Warning` ; tout autre code donne `Error`.
- Configuration du linter : `severity` accepte `fatal|error|warning|information` ; `hint` reste accepté comme alias de `information`.
- **BREAKING** Suppression de `RuleRegistry::register`.
- Bindings Python et Node : `Diagnostic.severity` vaut `fatal`, `error`, `warning` ou `information`.

## Capabilities

### New Capabilities

_Aucune._

### Modified Capabilities

- `fhir-validator` : type `Severity`, nom textuel, mapping de severity des invariants.
- `fhir-linter` : trait Rule / RuleRegistry (plus de `register`), configuration TOML.
- `fhir-python-bindings` : valeurs de `Diagnostic.severity`.
- `fhir-node-bindings` : valeurs de `Diagnostic.severity`.

## Impact

- Crates `fhir-validator`, `fhir-linter`, bindings Python et TypeScript.
- Les consommateurs qui comparaient `severity` à `"hint"` ne le recevront plus ; une configuration `hint` existante continue de fonctionner.
