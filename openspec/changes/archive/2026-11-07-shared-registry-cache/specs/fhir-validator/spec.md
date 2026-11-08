## ADDED Requirements

### Requirement: Nom textuel de Severity

Le système SHALL fournir `Severity::as_str()` retournant le nom en minuscules de la sévérité : `"error"`, `"warning"`, `"information"`, `"hint"`. C'est la représentation exposée par les bindings.

#### Scenario: Conversion en chaîne

- **WHEN** `as_str()` est appelé sur `Severity::Error`, `Warning`, `Information` et `Hint`
- **THEN** il SHALL retourner respectivement `"error"`, `"warning"`, `"information"` et `"hint"`
