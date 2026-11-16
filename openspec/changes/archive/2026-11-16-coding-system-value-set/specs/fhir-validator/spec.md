## ADDED Requirements

### Requirement: Validation terminologique — system désignant un ValueSet (TERMINOLOGY_003)

Le système SHALL émettre un Diagnostic de sévérité error `TERMINOLOGY_003`, au `path` du `system`, lorsque `Coding.system` (lié ou non, à toute profondeur) ou `ValueSet.compose.include.system` / `ValueSet.compose.exclude.system` est l'URL canonique d'un ValueSet chargé dans le Registry. Une URI inconnue du Registry SHALL NOT être signalée.

#### Scenario: Coding.system est un ValueSet

- **WHEN** un `Coding` (lié ou non, à toute profondeur) a pour `system` l'URL canonique d'un ValueSet du Registry
- **THEN** un Diagnostic d'erreur `TERMINOLOGY_003` SHALL être émis au `path` du `system`

#### Scenario: compose include/exclude system est un ValueSet

- **WHEN** `ValueSet.compose.include.system` ou `exclude.system` est l'URL canonique d'un ValueSet du Registry
- **THEN** un Diagnostic d'erreur `TERMINOLOGY_003` SHALL être émis au `path` de ce `system`

#### Scenario: system inconnu

- **WHEN** le `system` est une URI absente du Registry
- **THEN** aucun Diagnostic `TERMINOLOGY_003` SHALL être émis
