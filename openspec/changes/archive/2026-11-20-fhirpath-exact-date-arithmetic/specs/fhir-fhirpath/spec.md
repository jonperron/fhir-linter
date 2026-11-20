## MODIFIED Requirements

### Requirement: Arithmétique de dates avec durées calendaires

Le système SHALL implémenter N1 §6.7.1/§6.7.2 : `Date`, `DateTime` et `Time` `+`/`-` une `Quantity` de durée calendaire (mots-clés `year`..`millisecond` au singulier ou pluriel, ou UCUM `'d'`, `'wk'`, `'h'`, `'min'`, `'s'`, `'ms'` ; `'month'` etc. entre guillemets sont acceptés comme le mot-clé). Le résultat SHALL conserver la précision et le texte du fuseau de la valeur. Un décalage en mois ou années SHALL ramener le jour à la fin du mois (années bissextiles comprises). Les montants SHALL être des décimaux exacts, sans virgule flottante binaire, tronqués vers zéro (négatifs compris) : `@2015-02-04 + 1000000.99999999999999999 days` ajoute 1000000 jours et `+ 0.0009999999999 's'` sur une valeur à la milliseconde n'ajoute rien. Un montant dont le nombre dépasse i64 SHALL être une erreur, jamais saturé. Pour les précisions au-dessus des secondes, la partie décimale du montant SHALL être ignorée, quelle que soit la forme de l'unité (mot-clé calendaire ou UCUM `'d'`, `'wk'`, `'h'`, `'min'`) : `1.5 'h'` et `1.5 hours` ajoutent 1 heure ; les secondes et millisecondes gardent leurs décimales. Sur une `Date`, l'unité SHALL être `year`, `month`, `week` ou `day` (N1) ; heure, minute, seconde et milliseconde (y compris `'h'`, `'min'`, `'s'`, `'ms'`) SHALL être une erreur. Une unité plus fine que la valeur SHALL être convertie à la précision de la valeur, par troncature sans arrondi vers le pas supérieur (`@2014 + 24 months` donne `@2016` ; `@T10:00:00 + 0.9999996 seconds` inchangé). Une unité d'un jour ou moins sur une valeur à précision année SHALL être convertie à 365 jours par an (`@2016 + 365 days` donne `@2017`) ; sur une valeur à précision mois, à 30 jours par mois (table de conversion de N2, N1 muette : `@2026-02 + 5 weeks` donne `@2026-03`). Un `Time` SHALL boucler modulo 24 h. Les fractions SHALL être réécrites sur 3 chiffres.

Les cas suivants SHALL être des erreurs : UCUM `'a'` et `'mo'`, unités hors temps (`'cm'`), nombre sans unité, `Quantity + Date` (sens inverse), unités de date (année, mois, semaine, jour) sur un `Time`, unités horaires sur une `Date`, montant NaN ou infini, montant dont le nombre dépasse i64, résultat hors années 0001 à 9999, opérande invalide (mois, jour ou heure/minute hors bornes, seconde au-dessus de 60, année hors 0001 à 9999, horaire sur une date partielle comme `@2014T10:00`). La seconde 60 SHALL être acceptée en opérande ; un décalage d'un jour ou moins sur une seconde 60 SHALL être une erreur (N2 est muette), un décalage en mois ou années SHALL la conserver.

#### Scenario: Décalage en mois avec ajustement du jour

- **WHEN** `@2020-01-31 + 1 month` puis `@2019-03-31 - 1 month` sont évalués
- **THEN** les résultats SHALL être `@2020-02-29` et `@2019-02-28`

#### Scenario: Troncature à la précision de la valeur

- **WHEN** `@2019-03-01T00 + 24 hours`, `@2014 + 24 months` et `@2019-03-01 + 7.7 days` sont évalués
- **THEN** les résultats SHALL être `@2019-03-02T00`, `@2016` et `@2019-03-08`

#### Scenario: Montant décimal exact

- **WHEN** `@2015-02-04 + 1000000.99999999999999999 days` et `@2015-02-04T10:00:00.000Z + 0.0009999999999 's'` sont évalués
- **THEN** le premier SHALL ajouter exactement 1000000 jours et le second SHALL rester inchangé

#### Scenario: Décimales ignorées au-dessus des secondes

- **WHEN** `@2019-03-01T00 + 1.5 'h'` et `@2019-03-01T00 + 1.5 hours` sont évalués
- **THEN** les deux résultats SHALL être `@2019-03-01T01`

#### Scenario: Montant hors i64

- **WHEN** `@2019-03-01 + 99999999999999999999 days` est évalué
- **THEN** l'évaluateur SHALL retourner une erreur

#### Scenario: Unité horaire sur une Date

- **WHEN** `@2019-03-01 + 23 hours`, `@2019-03-01 + 1 'min'` ou `@2019-03-01 + 1 's'` est évalué
- **THEN** l'évaluateur SHALL retourner une erreur

#### Scenario: Unité de jour sur valeur partielle

- **WHEN** `@2016 + 365 days` et `@2026-02 + 5 weeks` sont évalués
- **THEN** les résultats SHALL être `@2017` et `@2026-03`

#### Scenario: Montant inférieur au pas sans arrondi

- **WHEN** `@T10:00:00 + 0.9999996 seconds` et `@2015-02-04T10:00:00.000Z + 1.005 seconds` sont évalués
- **THEN** les résultats SHALL être `@T10:00:00` et `@2015-02-04T10:00:01.005Z`

#### Scenario: Time modulo 24 h

- **WHEN** `@T23:30:00 + 1 hour` est évalué
- **THEN** le résultat SHALL être `@T00:30:00`

#### Scenario: Unités et opérandes invalides

- **WHEN** `@2019 + 1 'a'`, `@2019-01-01 + 1 'cm'`, `@2019-01-01 + 1`, `1 day + @2019-01-01`, `@T10:00 + 1 day` ou `@9999-12-31 + 1 day` est évalué
- **THEN** l'évaluateur SHALL retourner une erreur

#### Scenario: Opérande de date ou d'heure invalide

- **WHEN** `@1900-02-29 + 1 day`, `@2015-13-01 + 1 day`, `@2015-02-30 + 1 day`, `@2015-02-04T24:00:00Z + 1 hour`, `@2015-02-04T10:60:00Z + 1 hour`, `@0000-01-01 + 1 day` ou `@2014T10:00 + 1 hour` est évalué
- **THEN** l'évaluateur SHALL retourner une erreur
- **AND** `@2000-02-29 + 1 day` et `@1900-02-28 + 1 day` SHALL donner `@2000-03-01` et `@1900-03-01`

#### Scenario: Seconde intercalaire

- **WHEN** `@2016-12-31T23:59:60Z + 0 seconds` puis `@2016-12-31T23:59:60Z + 1 year` sont évalués
- **THEN** le premier SHALL être une erreur et le second SHALL donner `@2017-12-31T23:59:60Z`
