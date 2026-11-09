## Why

Le linter et le validateur n'étaient accessibles que comme bibliothèques. Il manquait un binaire en ligne de commande utilisable en local et en CI, avec des formats de sortie exploitables (texte, JSON, SARIF) et des codes de sortie stables.

## What Changes

- Nouveau binaire `fhir-lint` (crate `fhir-cli`) : collecte de fichiers et répertoires, validation puis lint de chaque ressource, rapport texte / JSON / SARIF 2.1.0, codes de sortie 0 / 1 / 2.
- Nouvelle méthode `RuleRegistry::check_config(&Config)` pour valider une configuration sans exécuter les règles ; `run` échoue de la même façon.
- Écart volontaire avec PLAN.md : la version FHIR se choisit via `--fhir-version` (`--version` affiche la version de l'outil). Pas d'option `--profile` : le validateur n'applique que `meta.profile`.

## Capabilities

### New Capabilities

- `fhir-cli` : binaire `fhir-lint`.

### Modified Capabilities

- `fhir-linter` : ajout de `RuleRegistry::check_config`.

## Impact

- `crates/fhir-cli/src/{main,files,output}.rs`, `crates/fhir-cli/tests/cli_tests.rs`
- `crates/fhir-linter/src/lib.rs`
