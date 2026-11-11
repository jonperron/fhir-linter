## 1. Validateur

- [x] 1.1 Remplacer les variants de `Severity` par `Fatal`, `Error`, `Warning`, `Information` et ajouter `Severity::ALL`
- [x] 1.2 Mettre à jour `as_str` et ajouter `FromStr` (inverse exact)
- [x] 1.3 Mapper la severity des invariants : seule `warning` donne Warning

## 2. Linter

- [x] 2.1 Accepter `fatal|error|warning|information` et l'alias `hint` dans la configuration
- [x] 2.2 Supprimer `RuleRegistry::register`

## 3. Bindings et tests

- [x] 3.1 Mettre à jour les bindings Python et Node et leurs tests
- [x] 3.2 Ajouter les tests Rust (ordre, aller-retour, rejet, configuration)
