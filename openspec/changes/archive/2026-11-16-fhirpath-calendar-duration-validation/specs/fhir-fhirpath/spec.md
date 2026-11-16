## MODIFIED Requirements

### Requirement: Arithmétique de dates avec durées calendaires

Le système SHALL implémenter N2 §6.6.7/§6.6.8 : `Date`, `DateTime` et `Time` `+`/`-` une `Quantity` de durée calendaire (mots-clés `year`..`millisecond` au singulier ou pluriel, ou UCUM `'d'`, `'wk'`, `'h'`, `'min'`, `'s'`, `'ms'` ; `'month'` etc. entre guillemets sont acceptés comme le mot-clé). Le résultat SHALL conserver la précision et le texte du fuseau de la valeur. Un décalage en mois ou années SHALL ramener le jour à la fin du mois (années bissextiles comprises). Un montant au-dessus des secondes SHALL perdre ses décimales (`7.7 days` donne 7). Une unité plus fine que la valeur SHALL être convertie à la précision de la valeur, par troncature sans arrondi vers le pas supérieur (`@2019-03-01 + 23 hours` inchangé ; `@2014 + 24 months` donne `@2016` ; `@T10:00:00 + 0.9999996 seconds` inchangé). Un `Time` SHALL boucler modulo 24 h. Les fractions SHALL être réécrites sur 3 chiffres.

Les cas suivants SHALL être des erreurs : UCUM `'a'` et `'mo'`, unités hors temps (`'cm'`), nombre sans unité, `Quantity + Date` (sens inverse), jours/semaines sur une valeur d'année ou de mois, unités d'un jour ou plus sur un `Time` (non définis par N2), montant NaN ou infini, résultat hors années 0001 à 9999, opérande invalide (mois, jour ou heure/minute hors bornes, seconde au-dessus de 60, année hors 0001 à 9999, horaire sur une date partielle comme `@2014T10:00`). La seconde 60 SHALL être acceptée en opérande ; un décalage d'un jour ou moins sur une seconde 60 SHALL être une erreur (N2 est muette), un décalage en mois ou années SHALL la conserver.

#### Scenario: Décalage en mois avec ajustement du jour

- **WHEN** `@2020-01-31 + 1 month` puis `@2019-03-31 - 1 month` sont évalués
- **THEN** les résultats SHALL être `@2020-02-29` et `@2019-02-28`

#### Scenario: Troncature à la précision de la valeur

- **WHEN** `@2019-03-01 + 23 hours`, `@2014 + 24 months` et `@2019-03-01 + 7.7 days` sont évalués
- **THEN** les résultats SHALL être `@2019-03-01`, `@2016` et `@2019-03-08`

#### Scenario: Montant inférieur au pas sans arrondi

- **WHEN** `@T10:00:00 + 0.9999996 seconds` et `@2015-02-04T10:00:00.000Z + 1.005 seconds` sont évalués
- **THEN** les résultats SHALL être `@T10:00:00` et `@2015-02-04T10:00:01.005Z`

#### Scenario: Time modulo 24 h

- **WHEN** `@T23:30:00 + 1 hour` est évalué
- **THEN** le résultat SHALL être `@T00:30:00`

#### Scenario: Unités et opérandes invalides

- **WHEN** `@2019 + 1 'a'`, `@2019-01-01 + 1 'cm'`, `@2019-01-01 + 1`, `1 day + @2019-01-01`, `@2019 + 1 day`, `@T10:00 + 1 day` ou `@9999-12-31 + 1 day` est évalué
- **THEN** l'évaluateur SHALL retourner une erreur

#### Scenario: Opérande de date ou d'heure invalide

- **WHEN** `@1900-02-29 + 1 day`, `@2015-13-01 + 1 day`, `@2015-02-30 + 1 day`, `@2015-02-04T24:00:00Z + 1 hour`, `@2015-02-04T10:60:00Z + 1 hour`, `@0000-01-01 + 1 day` ou `@2014T10:00 + 1 hour` est évalué
- **THEN** l'évaluateur SHALL retourner une erreur
- **AND** `@2000-02-29 + 1 day` et `@1900-02-28 + 1 day` SHALL donner `@2000-03-01` et `@1900-03-01`

#### Scenario: Seconde intercalaire

- **WHEN** `@2016-12-31T23:59:60Z + 0 seconds` puis `@2016-12-31T23:59:60Z + 1 year` sont évalués
- **THEN** le premier SHALL être une erreur et le second SHALL donner `@2017-12-31T23:59:60Z`
