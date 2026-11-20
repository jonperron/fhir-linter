## ADDED Requirements

### Requirement: Conversion implicite d'un nombre en Quantity
Selon N1 §5.5, un `Integer`, un `Long` ou un `Decimal` comparé à une Quantity SHALL être converti implicitement en Quantity d'unité `'1'`. Cette conversion SHALL s'appliquer à `=`, `!=`, `~`, `!~`, `<`, `>`, `<=`, `>=`, à `sort`, `min` et `max`, et aux opérations ensemblistes (`union`/`|`, `distinct`, `isDistinct`, `in`, `contains`, `intersect`, `exclude`, `subsetOf`), qui SHALL concorder avec `=`. La comparaison suit ensuite les règles UCUM des Quantity ; des unités incompatibles donnent une collection vide pour `=`, `!=` et les opérateurs d'ordre, et `false` pour `~`. `comparable()` SHALL continuer d'exiger deux Quantity.

#### Scenario: Égalité nombre et Quantity
- **WHEN** `1 = 1 '1'`, `1 = 100 '%'`, `1 = 1 'mg'` sont évalués
- **THEN** les résultats SHALL être `true`, `true` et une collection vide

#### Scenario: Équivalence nombre et Quantity
- **WHEN** `1 ~ 1 'mg'` est évalué
- **THEN** le résultat SHALL être `false`

#### Scenario: Ordre nombre et Quantity
- **WHEN** `2 > 1 '1'` et `2 > 1 'mg'` sont évalués
- **THEN** les résultats SHALL être `true` et une collection vide

#### Scenario: Opérations ensemblistes
- **WHEN** `(1 | 1 '1').count()` et `1 in (1 '1')` sont évalués
- **THEN** les résultats SHALL concorder avec `=` (`1` et `true`)

#### Scenario: comparable inchangé
- **WHEN** `1.comparable(1 '1')` est évalué
- **THEN** le résultat SHALL être une collection vide
