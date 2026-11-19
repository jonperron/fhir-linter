## MODIFIED Requirements

### Requirement: Fonctions mathématiques

Le système SHALL implémenter `abs()`, `ceiling()`, `floor()`, `round()`, `round(precision)`, `sqrt()`, `exp()`, `ln()`, `log(base)`, `power(exponent)`, `truncate()`. Conformément à FHIRPath N2, `sqrt()`, `exp()`, `ln()`, `log(base)` et `power(exponent)` SHALL retourner une collection vide lorsque le résultat n'est pas représentable (NaN ou infini).

#### Scenario: abs sur valeur négative

- **WHEN** `(-3).abs()` est évalué
- **THEN** le résultat SHALL être `[Integer(3)]`

#### Scenario: Dépassement sur i64::MIN

- **WHEN** `abs()` ou la négation unaire est appliqué à `i64::MIN` (Integer ou Long)
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))` sans paniquer

#### Scenario: Arithmétique entière exacte

- **WHEN** `+`, `-`, `*` ou `mod` est appliqué à deux `Integer`/`Long` (Integer/Long mixte donne `Long`)
- **THEN** le calcul SHALL être fait en `i64` (exact au-delà de 2^53), et un dépassement SHALL retourner `Err(EvalError::Type(...))`; un `Decimal` impliqué passe par l'arithmétique décimale exacte

#### Scenario: Négation d'un Decimal nul

- **WHEN** la négation unaire est appliquée à un Decimal zéro (`-0.0`)
- **THEN** le résultat SHALL être `0.0`, jamais `-0.0`

#### Scenario: Arithmétique décimale exacte

- **WHEN** `+`, `-`, `*` ou `mod` est appliqué à des Decimal (`1.2 mod 0.1`, `1.8 - 1.2`), ou `div`, `/`, `sum()` et `avg()` sur des Decimal
- **THEN** le calcul SHALL être exact sur `fhir_parser::Decimal` (`0.0` et `0.6`) ; `div` SHALL tronquer exactement ; `/` SHALL arrondir à la moitié en s'éloignant de zéro à au moins 8 décimales (ou l'échelle des opérandes) puis retirer les zéros finaux ; `mod` ne SHALL jamais dépasser, quelles que soient les échelles des opérandes (`|a mod b| < |b|`, calculé par arithmétique modulaire ; `i128::MIN mod -1` donne `0`) ; `div` et `/` SHALL rester définis quand les échelles sont très éloignées (`1e-30 div 1e30` donne `0`) et ne SHALL retourner le dépassement que si le quotient exact dépasse la mantisse `i128` de 38 chiffres (`1e20 div 1e-18` donne le résultat exact même si l'intermédiaire de mise à l'échelle dépasse, `1e30 div 1e-30` est un dépassement) ; un autre dépassement SHALL retourner `Err(EvalError::Type(...))` sans paniquer, y compris le dépassement d'un `sum()` d'entiers

#### Scenario: sqrt sur entier positif

- **WHEN** `4.sqrt()` est évalué
- **THEN** le résultat SHALL être `[Decimal(2.0)]`

#### Scenario: Résultat non représentable

- **WHEN** `(-1).sqrt()`, `(-1).power(0.5)`, `10.0.power(400)` ou `0.power(-1)` est évalué
- **THEN** le résultat SHALL être une collection vide
