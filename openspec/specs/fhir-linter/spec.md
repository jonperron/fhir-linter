# Spec: fhir-linter

## Purpose

Règles de lint configurables appliquées à une ressource FHIR, en complément du validateur.

## Requirements

### Requirement: Trait Rule et RuleRegistry

Le crate `fhir-linter` SHALL exposer le trait `Rule` (`id()`, `check(&Resource, &Registry) -> Vec<Diagnostic>`) et `RuleRegistry` (règles intégrées par `Default`, extensible par `register`). `RuleRegistry::run` SHALL renseigner `Diagnostic::rule` avec l'identifiant de la règle.

### Requirement: Règles intégrées

| Id | Code | Déclencheur |
|---|---|---|
| `narrative-absent` | LINT_001 | `text.div` absent ou vide, pour les types de ressource définissant `text` |
| `missing-identifier` | LINT_002 | `identifier` absent, pour les types définissant `identifier` |
| `missing-meta-profile` | LINT_003 | `meta.profile` absent |

Chaque règle SHALL émettre un Diagnostic de sévérité `Warning` par défaut.

#### Scenario: Ressource nue

- **WHEN** une `Patient` ne contient que `id`
- **THEN** les trois règles SHALL émettre un Diagnostic

#### Scenario: Type sans `text`

- **WHEN** la ressource est `Parameters`
- **THEN** `narrative-absent` et `missing-identifier` SHALL ne rien émettre

### Requirement: Configuration TOML

`Config::from_toml` SHALL lire des sections `[rules.<id>]` avec `enabled` (bool) et `severity` (`error|warning|information|hint`). Une règle désactivée SHALL être ignorée ; la sévérité configurée SHALL remplacer celle par défaut. Un identifiant de règle ou une sévérité inconnus SHALL faire échouer `run` avec un message d'erreur.
