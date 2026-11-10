## MODIFIED Requirements

### Requirement: Trait Rule et RuleRegistry

Le crate `fhir-linter` SHALL exposer le trait `Rule` (`id()`, `check(&Resource, &Registry) -> Vec<Diagnostic>`) et `RuleRegistry` (règles intégrées par `Default`, extensible par `register`). `RuleRegistry::run` SHALL renseigner `Diagnostic::rule` avec l'identifiant de la règle.

#### Scenario: Identifiant de règle renseigné

- **WHEN** `RuleRegistry::default().run` est appliqué à une `Patient` nue
- **THEN** chaque Diagnostic SHALL avoir `rule` égal à l'identifiant de la règle émettrice

### Requirement: Configuration TOML

`Config::from_toml` SHALL lire des sections `[rules.<id>]` avec `enabled` (bool) et `severity` (`error|warning|information|hint`). Une règle désactivée SHALL être ignorée ; la sévérité configurée SHALL remplacer celle par défaut. Un identifiant de règle ou une sévérité inconnus SHALL faire échouer `run` avec un message d'erreur.

#### Scenario: Désactivation et sévérité

- **WHEN** la configuration contient `[rules.missing-identifier] enabled = false` et `[rules.narrative-absent] severity = "error"`
- **THEN** `missing-identifier` SHALL ne rien émettre et `narrative-absent` SHALL émettre avec la sévérité `Error`

#### Scenario: Identifiant ou sévérité inconnus

- **WHEN** la configuration contient `[rules.nope]` ou `severity = "loud"`
- **THEN** `run` SHALL renvoyer une erreur

### Requirement: Règles intégrées

Le `RuleRegistry` par défaut SHALL contenir les règles suivantes.

| Id | Code | Déclencheur |
|---|---|---|
| `narrative-absent` | LINT_001 | narratif absent, pour les types de ressource définissant `text` : `text` absent, `text.status` vaut `empty`, ou `text.div` absent ou sans contenu |
| `missing-identifier` | LINT_002 | `identifier` absent, pour les types définissant `identifier` ; pour `Bundle`, seulement si `type` vaut `document`, `message` ou `collection` |
| `missing-meta-profile` | LINT_003 | `meta.profile` absent |

Chaque règle SHALL émettre un Diagnostic de sévérité `Warning` par défaut.

Un `text.div` SHALL être considéré sans contenu (narrative.html, txt-2) lorsque, une fois les balises retirées, il ne reste que des blancs et qu'il ne contient aucun élément `<img>`. Les références d'entité ou de caractère (`&#160;`) SHALL compter comme du texte.

Un élément répétable (`identifier`, `meta.profile`) SHALL être considéré absent lorsque la clé manque, ou que son tableau est vide ou ne contient que des `null`.

Un `Bundle` sans `type`, ou d'un autre type que `document`, `message` ou `collection`, SHALL NOT déclencher `missing-identifier` (commentaire R5 de `Bundle.identifier`).

#### Scenario: Ressource nue

- **WHEN** une `Patient` ne contient que `id`
- **THEN** les trois règles SHALL émettre un Diagnostic

#### Scenario: Type sans `text`

- **WHEN** la ressource est `Parameters`
- **THEN** `narrative-absent` et `missing-identifier` SHALL ne rien émettre

#### Scenario: Div réduit au balisage et aux blancs

- **WHEN** `text.div` vaut `<div xmlns="http://www.w3.org/1999/xhtml"> <p> </p> </div>`
- **THEN** `narrative-absent` SHALL émettre LINT_001

#### Scenario: Div avec texte ou image

- **WHEN** `text.div` contient du texte non blanc, une référence `&#160;` ou un élément `<img>`
- **THEN** `narrative-absent` SHALL ne rien émettre

#### Scenario: Statut `empty`

- **WHEN** `text.status` vaut `empty`, quel que soit le contenu du div
- **THEN** `narrative-absent` SHALL émettre LINT_001

#### Scenario: Tableaux vides ou nuls

- **WHEN** `identifier` vaut `[]` ou `[null]`, ou `meta.profile` vaut `[]` ou `[null]`
- **THEN** respectivement LINT_002 ou LINT_003 SHALL être émis

#### Scenario: Bundle selon son type

- **WHEN** un `Bundle` sans `identifier` est de type `document`, `message` ou `collection`
- **THEN** LINT_002 SHALL être émis
- **WHEN** un `Bundle` sans `identifier` est de type `searchset` ou n'a pas de `type`
- **THEN** LINT_002 SHALL NOT être émis

## ADDED Requirements

### Requirement: Localisation des diagnostics

LINT_002 et LINT_003 SHALL porter comme `location` le span de `meta` lorsque `meta` est présent, et aucune position sinon. LINT_001 SHALL porter le span de `text` lorsque `text` est présent.

#### Scenario: Position sur `meta`

- **WHEN** une ressource a un `meta` sans `profile` et aucun `identifier`
- **THEN** LINT_002 et LINT_003 SHALL avoir `location` égal au span de `meta`

#### Scenario: Position sur `text`

- **WHEN** une ressource a un `text` dont le div est vide
- **THEN** LINT_001 SHALL avoir `location` égal au span de `text`

### Requirement: Relation avec l'invariant dom-6

`narrative-absent` SHALL être un surensemble délibéré et documenté de l'invariant dom-6 du validateur : une ressource sans `text` SHALL produire INVARIANT_dom-6 (validation) et LINT_001 (lint) ; LINT_001 SHALL aussi être émis pour un narratif vide que dom-6 accepte. Les deux sorties SHALL NOT être dédupliquées.

#### Scenario: Ressource sans `text`

- **WHEN** une `Patient` sans `text` est validée puis lintée
- **THEN** la validation SHALL émettre INVARIANT_dom-6 et le lint SHALL émettre LINT_001

#### Scenario: Narratif vide accepté par dom-6

- **WHEN** une `Patient` a `text.status = "empty"` et un div non vide
- **THEN** dom-6 SHALL être satisfait et LINT_001 SHALL être émis

