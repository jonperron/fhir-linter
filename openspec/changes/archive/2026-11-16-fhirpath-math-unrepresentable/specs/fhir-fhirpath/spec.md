## MODIFIED Requirements

### Requirement: Fonctions mathématiques

Le système SHALL implémenter `abs()`, `ceiling()`, `floor()`, `round()`, `round(precision)`, `sqrt()`, `exp()`, `ln()`, `log(base)`, `power(exponent)`, `truncate()`. Conformément à FHIRPath N2, `sqrt()` et `power(exponent)` SHALL retourner une collection vide lorsque le résultat n'est pas représentable (NaN ou infini).

#### Scenario: abs sur valeur négative

- **WHEN** `(-3).abs()` est évalué
- **THEN** le résultat SHALL être `[Integer(3)]`

#### Scenario: Dépassement sur i64::MIN

- **WHEN** `abs()` ou la négation unaire est appliqué à `i64::MIN` (Integer ou Long)
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))` sans paniquer

#### Scenario: Arithmétique entière exacte

- **WHEN** `+`, `-`, `*` ou `mod` est appliqué à deux `Integer`/`Long` (Integer/Long mixte donne `Long`)
- **THEN** le calcul SHALL être fait en `i64` (exact au-delà de 2^53), et un dépassement SHALL retourner `Err(EvalError::Type(...))`; `f64` n'est utilisé que si un `Decimal` est impliqué

#### Scenario: sqrt sur entier positif

- **WHEN** `4.sqrt()` est évalué
- **THEN** le résultat SHALL être `[Decimal(2.0)]`

#### Scenario: Résultat non représentable

- **WHEN** `(-1).sqrt()`, `(-1).power(0.5)`, `10.0.power(400)` ou `0.power(-1)` est évalué
- **THEN** le résultat SHALL être une collection vide
