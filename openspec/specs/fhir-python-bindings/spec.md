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

`Diagnostic` SHALL être immuable avec les attributs `severity` (`error`, `warning`, `information`, `hint`), `code`, `message`, `path`, `line`, `col`, `offset` (`None` sans localisation) et `rule` (`None` pour le validateur).
