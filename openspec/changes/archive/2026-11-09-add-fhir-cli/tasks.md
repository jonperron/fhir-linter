## 1. fhir-linter

- [x] 1.1 Ajouter `RuleRegistry::check_config` et l'appeler depuis `run`

## 2. fhir-cli

- [x] 2.1 Arguments clap (`PATHS`, `--format`, `--config`, `--fhir-version`, `--definitions`, `--strict`)
- [x] 2.2 Collecte des fichiers (`files.rs`)
- [x] 2.3 Validation + lint par ressource, `PARSE_001` pour les échecs de parsing
- [x] 2.4 Rendu texte, JSON et SARIF (`output.rs`)
- [x] 2.5 Codes de sortie 0 / 1 / 2
- [x] 2.6 Tests d'intégration (`tests/cli_tests.rs`)
