## Why

Audit 11.2-A2/A5 : le contrôle `code` rejetait tout whitespace, alors que la regex R5 `[^\s]+( [^\s]+)*` autorise des mots séparés par un espace simple (ex. `text/plain; charset=utf-8`) ; le contrôle `dateTime` rejetait un fuseau sur une valeur réduite à la date, que la regex R5 accepte sur une année-mois ou une date complète.

## What Changes

- `code` : `TYPE_004` dès que la valeur ne suit pas `[^\s]+( [^\s]+)*` (vide, espace initial/final ou double, whitespace autre que l'espace).
- `dateTime` : fuseau accepté sur `YYYY-MM` et `YYYY-MM-DD` (`2020-01Z`, `2020-01-01+02:00`), rejeté sur une année seule (`2020Z`).

## Capabilities

### New Capabilities

### Modified Capabilities

- `fhir-validator` : exigences TYPE_003 (dateTime) et TYPE_004 (code).

## Impact

`crates/fhir-validator/src/type_check.rs` et ses tests unitaires. Des codes à espaces simples et des dateTime date+fuseau deviennent valides ; les codes à espaces mal placés restent rejetés.
