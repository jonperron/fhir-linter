## MODIFIED Requirements

### Requirement: Trait Rule et RuleRegistry

Le crate `fhir-linter` SHALL exposer le trait `Rule` (`id()`, `check(&Resource, &Registry) -> Vec<Diagnostic>`) et `RuleRegistry`, dont l'ensemble de règles est fixé aux règles intégrées par `Default` (aucune méthode `register`). `RuleRegistry::run` SHALL renseigner `Diagnostic::rule` avec l'identifiant de la règle.

#### Scenario: Identifiant de règle renseigné

- **WHEN** `RuleRegistry::default().run` est appliqué à une `Patient` nue
- **THEN** chaque Diagnostic SHALL avoir `rule` égal à l'identifiant de la règle émettrice

### Requirement: Configuration TOML

`Config::from_toml` SHALL lire des sections `[rules.<id>]` avec `enabled` (bool) et `severity`, un code FHIR R5 IssueSeverity (`fatal|error|warning|information`). La valeur `hint` SHALL être acceptée comme alias de `information` (compatibilité avec les versions antérieures) et le diagnostic SHALL alors être émis avec la sévérité `Information`. Une règle désactivée SHALL être ignorée ; la sévérité configurée SHALL remplacer celle par défaut. Un identifiant de règle ou une sévérité inconnus (y compris `success`, qui n'est pas un niveau de diagnostic) SHALL faire échouer `run` avec un message d'erreur.

#### Scenario: Désactivation et sévérité

- **WHEN** la configuration contient `[rules.missing-identifier] enabled = false` et `[rules.narrative-absent] severity = "error"`
- **THEN** `missing-identifier` SHALL ne rien émettre et `narrative-absent` SHALL émettre avec la sévérité `Error`

#### Scenario: Sévérité fatal

- **WHEN** la configuration contient `[rules.narrative-absent] severity = "fatal"`
- **THEN** `narrative-absent` SHALL émettre avec la sévérité `Fatal`

#### Scenario: Alias hint

- **WHEN** la configuration contient `[rules.narrative-absent] severity = "hint"`
- **THEN** `narrative-absent` SHALL émettre avec la sévérité `Information`

#### Scenario: Identifiant ou sévérité inconnus

- **WHEN** la configuration contient `[rules.nope]`, `severity = "loud"` ou `severity = "success"`
- **THEN** `run` SHALL renvoyer une erreur
