# Spec: fhir-linter

## Purpose

Règles de lint configurables appliquées à une ressource FHIR, en complément du validateur.
## Requirements
### Requirement: Trait Rule et RuleRegistry

Le crate `fhir-linter` SHALL exposer le trait `Rule` (`id()`, `check(&Resource, &Registry) -> Vec<Diagnostic>`) et `RuleRegistry` (règles intégrées par `Default`, extensible par `register`). `RuleRegistry::run` SHALL renseigner `Diagnostic::rule` avec l'identifiant de la règle.

#### Scenario: Identifiant de règle renseigné

- **WHEN** `run` produit un Diagnostic `LINT_003`
- **THEN** son champ `rule` SHALL valoir `missing-meta-profile`

### Requirement: Règles intégrées

`RuleRegistry::default()` SHALL enregistrer les règles suivantes :

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

`Config::from_toml` SHALL lire des sections `[rules.<id>]` avec `enabled` (bool) et `severity` (`error|warning|information|hint`). Une règle désactivée SHALL être ignorée ; la sévérité configurée SHALL remplacer celle par défaut. `run` SHALL appeler `check_config` avant d'exécuter les règles : un identifiant de règle ou une sévérité inconnus SHALL faire échouer `run` avec le même message d'erreur.

#### Scenario: Règle désactivée

- **WHEN** la configuration contient `[rules.missing-meta-profile]` avec `enabled = false`
- **THEN** `run` SHALL ne produire aucun diagnostic `LINT_003`

#### Scenario: Configuration invalide

- **WHEN** `run` reçoit une configuration référençant une règle inconnue
- **THEN** `run` SHALL renvoyer une erreur sans exécuter de règle

### Requirement: Vérification de la configuration

`RuleRegistry::check_config(&Config) -> Result<(), String>` SHALL vérifier, sans exécuter aucune règle, que chaque identifiant de `[rules.<id>]` désigne une règle enregistrée et que chaque `severity` vaut `error|warning|information|hint`. La vérification SHALL porter aussi sur les règles désactivées. Le message d'erreur SHALL nommer la règle fautive.

#### Scenario: Règle inconnue

- **WHEN** la configuration contient `[rules.no-such-rule]`
- **THEN** `check_config` SHALL renvoyer une erreur contenant `no-such-rule`

#### Scenario: Sévérité invalide sur une règle désactivée

- **WHEN** la configuration contient `[rules.narrative-absent]` avec `enabled = false` et `severity = "fatal"`
- **THEN** `check_config` SHALL renvoyer une erreur

#### Scenario: Configuration valide

- **WHEN** la configuration ne référence que des règles intégrées avec des sévérités valides
- **THEN** `check_config` SHALL renvoyer `Ok(())`

