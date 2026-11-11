## Why

La branche `fix/fhirpath-operator-semantics` aligne les opérateurs FHIRPath sur N1 (division par zéro, évaluation singleton, `=`/`~`, logique booléenne, `as`, union/intersect). La spec `fhir-fhirpath` décrit encore l'ancien comportement (`DivisionByZero` produit, plancher de 749 cas).

## What Changes

- `/`, `div`, `mod` par zéro donnent une collection vide ; `EvalError::DivisionByZero` n'est plus produit.
- Union et `intersect` dédupliquent les deux opérandes.
- Évaluation singleton (N1 §4.5) pour les contextes booléens, l'arithmétique, les comparaisons, `in`/`contains` ; critères de `where`/`exists`/`all`/`iif` stricts ; `allTrue` et consorts rejettent les non-booléens.
- `=` compare les collections dans l'ordre ; `~` n'est jamais vide, ignore l'ordre, la casse et normalise les espaces ; dates comparées précision par précision avec fuseaux normalisés.
- `and`/`or`/`implies` ignorent l'erreur du côté non décisif.
- L'opérateur `as` échoue sur plusieurs éléments.
- Plancher de la suite officielle relevé à 817.

## Capabilities

### New Capabilities

Aucune.

### Modified Capabilities

- `fhir-fhirpath` : nouvelles exigences (Évaluation singleton, Égalité et équivalence, Opérateurs booléens, Division par zéro) et exigences existantes modifiées (existence, filtrage, collections, diverses, erreurs, suite officielle).

## Impact

`crates/fhir-fhirpath/src/eval/` et `error.rs`. Les invariants FHIR évalués par `fhir-validator` peuvent désormais lever des erreurs de type là où ils donnaient un résultat deviné. Aucune modification d'API publique.
