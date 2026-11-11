# Spec: fhir-python-bindings

## Purpose

Bindings Python (PyO3 + maturin) exposant le validateur et le linter. Paquet `fhir-linter` (wheels et sdist publiés sur PyPI et joints aux releases GitHub), module `fhir_linter`.
## Requirements
### Requirement: Fonction validate

Le module SHALL exposer `validate(json: str, definitions_dir: str | None = None) -> list[Diagnostic]` qui analyse une ressource FHIR R5 JSON et retourne les diagnostics de `fhir_validator::validate`.

#### Scenario: Type de ressource inconnu

- **WHEN** `validate('{"resourceType": "Nope"}')` est appelé
- **THEN** le résultat SHALL contenir un unique Diagnostic `STRUCTURE_000` de sévérité `error`

#### Scenario: JSON invalide

- **WHEN** le JSON ne peut pas être analysé
- **THEN** une `ValueError` SHALL être levée

### Requirement: Fonction lint

Le module SHALL exposer `lint(json: str, config_toml: str | None = None, definitions_dir: str | None = None) -> list[Diagnostic]` qui exécute les règles intégrées de `fhir-linter`. `config_toml` est le contenu d'un `fhir-linter.toml`.

#### Scenario: Configuration invalide

- **WHEN** `config_toml` n'est pas du TOML valide, ou référence une règle ou une sévérité inconnue
- **THEN** une `ValueError` SHALL être levée

### Requirement: Source des définitions

Les définitions FHIR R5 ne sont pas embarquées dans le wheel. Le répertoire SHALL être `definitions_dir`, à défaut la variable d'environnement `FHIR_DEFINITIONS_DIR`. Le `Registry` SHALL être construit une fois par répertoire puis partagé.

#### Scenario: Aucun répertoire

- **WHEN** ni `definitions_dir` ni `FHIR_DEFINITIONS_DIR` ne sont définis
- **THEN** une `ValueError` SHALL être levée

#### Scenario: Répertoire illisible

- **WHEN** le répertoire ne contient pas les définitions
- **THEN** une `RuntimeError` SHALL être levée

### Requirement: Type Diagnostic

`Diagnostic` SHALL être immuable avec les attributs `severity` (code FHIR R5 IssueSeverity : `fatal`, `error`, `warning` ou `information`), `code`, `message`, `path`, `line`, `col`, `offset` (`None` sans localisation) et `rule` (`None` pour le validateur). `col` (1-indexée) et `offset` (0-indexé) SHALL être des comptes d'octets UTF-8 de l'entrée telle que fournie (BOM inclus), et non des indices de caractères Python : pour retrouver la position, il faut indexer `resource_json.encode()`.

#### Scenario: Offset en octets UTF-8

- **WHEN** un Diagnostic localisé est produit pour une source contenant des caractères non ASCII avant la position signalée
- **THEN** `resource_json.encode()[d.offset:]` SHALL commencer au nœud signalé

#### Scenario: Sévérité configurée en code IssueSeverity

- **WHEN** `lint` est appelé avec `[rules.narrative-absent] severity = "fatal"`, respectivement `"hint"`
- **THEN** le Diagnostic `narrative-absent` SHALL avoir `severity` égal à `"fatal"`, respectivement `"information"`

