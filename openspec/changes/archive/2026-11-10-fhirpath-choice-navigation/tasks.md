## 1. FHIRPath

- [x] 1.1 Ajouter `evaluate_parsed_with_choices` et la navigation des choix dans `eval_ident`
- [x] 1.2 Tests : `value` trouve `valueQuantity`, clé explicite prioritaire, `_value` ignoré

## 2. Registry

- [x] 2.1 Construire l'index des variantes de choix au chargement
- [x] 2.2 Exposer `Registry::is_choice_variant(base, suffix)`

## 3. Validator

- [x] 3.1 Évaluer les invariants avec `evaluate_parsed_with_choices`
- [x] 3.2 Tests de régression fhs-2, mrp-2, obs-6
