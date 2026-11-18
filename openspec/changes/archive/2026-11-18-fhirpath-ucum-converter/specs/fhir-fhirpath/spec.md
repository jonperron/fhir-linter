## ADDED Requirements

### Requirement: Conversion d'unités UCUM (interne)
Le crate SHALL fournir un convertisseur UCUM interne `convert(from, to) -> Option<Ratio>` donnant un facteur rationnel exact (i128) tel que `montant_to = montant_from * facteur`, et `comparable(a, b)` SHALL valoir `convert(a, b).is_some()`. Les définitions SHALL provenir de `data/ucum-essence.xml` (copie non modifiée de l'essence UCUM v2.0.1, notice `data/UCUM-NOTICE.txt`), analysée une seule fois au premier usage. La grammaire SHALL accepter `.`, `/`, `/` initial, exposants entiers signés, parenthèses (imbrication limitée à 32), `{annotations}` (valeur 1), facteurs entiers (0 rejeté), atomes entre crochets et préfixes métriques uniquement sur les atomes métriques. Le convertisseur ne SHALL PAS encore être utilisé par l'évaluation FHIRPath (comparaison, égalité, `comparable()` et arithmétique inchangées).

#### Scenario: Unités convertibles
- **WHEN** `convert("km", "m")` est appelé
- **THEN** le résultat SHALL être le facteur exact 1000

#### Scenario: Unités non convertibles
- **WHEN** les unités sont mal formées, inconnues, de dimensions différentes, spéciales (`Cel`, `[degF]`), arbitraires (`[IU]`) ou de facteur hors i128 (`[pi]`, `[h]`)
- **THEN** le résultat SHALL être `None`
