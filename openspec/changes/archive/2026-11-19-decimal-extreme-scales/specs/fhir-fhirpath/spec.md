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
- **THEN** le calcul SHALL être exact sur `fhir_parser::Decimal` (`0.0` et `0.6`) ; `div` SHALL tronquer exactement ; `/` SHALL arrondir à la moitié en s'éloignant de zéro à au moins 8 décimales (ou l'échelle des opérandes) puis retirer les zéros finaux ; `div`, `/` et `mod` SHALL rester définis quand les échelles des opérandes sont très éloignées (`1e-30 div 1e30` donne `0`, `mod` rend le dividende quand le diviseur dépasse `i128`, `i128::MIN mod -1` donne `0`) ; un autre dépassement SHALL retourner `Err(EvalError::Type(...))` sans paniquer, y compris le dépassement d'un `sum()` d'entiers

#### Scenario: sqrt sur entier positif

- **WHEN** `4.sqrt()` est évalué
- **THEN** le résultat SHALL être `[Decimal(2.0)]`

#### Scenario: Résultat non représentable

- **WHEN** `(-1).sqrt()`, `(-1).power(0.5)`, `10.0.power(400)` ou `0.power(-1)` est évalué
- **THEN** le résultat SHALL être une collection vide

### Requirement: Fonctions de conversion

Le système SHALL implémenter `toInteger()`, `toLong()`, `toDecimal()`, `toBoolean()`, `convertsToInteger()`, `convertsToLong()`, `convertsToDecimal()`, `convertsToBoolean()`, `toDate()`, `toDateTime()`, `toTime()`, `toQuantity()`, `convertsToDate()`, `convertsToDateTime()`, `convertsToTime()`, `convertsToQuantity()`, `toString()`.

`toLong()` retourne une valeur `Long` (type distinct d'`Integer`).

Pour `toX()` et `convertsToX()` avec X parmi `Boolean`, `Integer`, `Decimal`, `Date`, `DateTime`, `Time` : une entrée vide (ou un seul `Null` de substitution) SHALL donner une collection vide ; plus d'un élément SHALL être une erreur ; un élément non convertible SHALL donner une collection vide (`toX`) ou `false` (`convertsToX`), jamais une erreur. Sources convertibles :

- `toBoolean` : `Boolean` ; `Integer` `1`/`0` ; `Decimal` `1.0`/`0.0` ; `String` `true`/`t`/`yes`/`y`/`1`/`1.0` et `false`/`f`/`no`/`n`/`0`/`0.0`, sans tenir compte de la casse.
- `toInteger` : `Integer`, `Boolean`, `String` `(+|-)?\d+` ; un `Decimal` SHALL donner une collection vide.
- `toDecimal` : `Decimal`, `Integer`, `Boolean`, `String` `(+|-)?\d+(\.\d+)?` uniquement (`NaN`, `inf`, `1e5` donnent une collection vide).
- `toDate` : `Date`, la partie date d'une `DateTime`, `String` `YYYY[-MM[-DD]]`.
- `toDateTime` : `DateTime`, `Date`, `String` `YYYY[-MM[-DD]][Thh[:mm[:ss[.fff]]][Z|(+|-)hh:mm]]` avec un décalage d'au plus `14:00`.
- `toTime` : `Time`, `String` `hh[:mm[:ss[.fff]]]`.

Une date en `String` SHALL être une date calendaire réelle (année `0001` à `9999`, jour valide pour son mois, années bissextiles comprises). `toLong`/`convertsToLong` (absents de N1, FHIRPath 3.0 STU) suivent le modèle de `toInteger` : entrée singleton, sources `Integer`, `Long`, `Boolean` et `String` `(\+|-)?\d+` dans les bornes `i64`, sinon collection vide (`toLong`) ou `false` (`convertsToLong`). Une chaîne de chiffres au-delà de `f64` ne SHALL pas être convertie en infini par `toDecimal` ni `toQuantity`. De même, `toQuantity` ne SHALL pas convertir un `Decimal` non fini (texte XML ou valeur `f64` au-delà de `f64`) : il donne une collection vide. Un littéral décimal FHIRPath de plus de 38 chiffres significatifs SHALL être une erreur de parsing. Le texte XML d'un `decimal` SHALL être exact, conserve la valeur `f64` au-delà de 38 chiffres et reste une `String` au-delà de `f64`.

`toQuantity([unit])` et `convertsToQuantity([unit])` suivent N1 §5.5.7 : `Integer` et `Decimal` donnent une quantité d'unité `'1'`, une `Quantity` est inchangée, un `Boolean` donne `1.0` ou `0.0` d'unité `'1'`, une `String` conforme à `(\+|-)?\d+(\.\d+)?\s*('unité'|mot-clé)?` est convertie (une unité sans guillemets SHALL être un mot-clé calendaire). Plusieurs éléments SHALL être une erreur, une entrée vide donne une collection vide, tout autre élément donne une collection vide (`toQuantity`) ou `false` (`convertsToQuantity`). Un argument `unit` SHALL convertir la quantité d'entrée vers l'unité demandée par le convertisseur UCUM (mot-clé calendaire normalisé au singulier) ; une unité non convertible (inconnue, spéciale, incompatible, absente) donne une collection vide (`toQuantity`) ou `false` (`convertsToQuantity`), un argument vide donne une collection vide et un argument non `String` est une erreur.

Les unités calendaires des littéraux de quantité (`year`, `month`, `week`, `day`, `hour`, `minute`, `second`, `millisecond`, au singulier ou au pluriel) SHALL être stockées au singulier. `=` SHALL apparier `week`, `day`, `hour`, `minute`, `second` et `millisecond` à leurs unités UCUM (`'wk'`, `'d'`, `'h'`, `'min'`, `'s'`, `'ms'`) ; `year` et `month` ne SHALL se comparer, sous `=` et en ordre, qu'avec le même mot-clé, et `~` SHALL les apparier à `'a'` et `'mo'`. Les comparaisons d'ordre SHALL accepter les mêmes paires que `=`.

Pour `=`, `!=`, `~`, `!~`, `<`, `<=`, `>`, `>=`, un objet FHIR Quantity (enfants typés : `value` de type `decimal`, clés limitées à value/unit/system/code/id/extension, texte décimal XML fini) SHALL être traité comme une quantité littérale (R5 §2.1.9.1). Son unité est le `code` si `system` vaut `http://unitsofmeasure.org`, sinon le texte `unit` en l'absence de `code`, sinon aucune. Un `code` avec un `system` absent ou non UCUM, ou un `comparator`, n'est pas reconnu : la comparaison structurelle d'objets s'applique. `=`, `!=`, `~`, `!~`, `<`, `<=`, `>`, `>=`, `comparable()`, `sort`, `min` et `max` SHALL convertir les unités par le convertisseur UCUM : l'opérande droit est converti dans l'unité du gauche, en `f64` avec une tolérance relative de 1e-12 (shortcut). Des unités inconnues, spéciales (`Cel`, `[degF]`), incompatibles ou présentes d'un seul côté SHALL donner une collection vide pour `=`, `!=` et les opérateurs d'ordre, `false` pour `~` et pour `comparable()`, et une erreur de type pour `sort`/`min`/`max` (inchangé). `~` SHALL arrondir à la précision de l'opérande le moins précis, mesurée dans l'unité gauche (`4 'g' ~ 4040 'mg'` est vrai). `*` et `/` sur des Quantity (N1 §6.6) SHALL combiner les montants comme des nombres et les unités par UCUM : les puissances d'un même atome fusionnent et s'annulent (`12 'cm' * 3 'cm'` donne `36 'cm2'`, `1 'm' / 1 'm'` donne `1 '1'`, `4 'g' / 2 'm'` donne `2 'g/m'`, `2.0 'cm' * 2.0 'm'` donne `4.0 'cm.m'`, égal à `0.040 'm2'`). Un opérande nombre a l'unité `'1'` et laisse l'autre unité telle qu'écrite (`6 / 3 's'` donne `2 '1/s'`). Une unité avec parenthèses ou annotations SHALL être conservée entière entre parenthèses. Les mots-clés calendaires `week` et plus courts SHALL valoir leur unité UCUM. Le résultat SHALL être vide pour une division par zéro, une Quantity sans unité, une unité inconnue ou non prise en charge, ou un mot-clé `year`/`month`. `+`, `-`, `div` et `mod` sur des Quantity ne sont pas encore pris en charge (erreur de type). La négation unaire et `abs()` d'une Quantity SHALL conserver l'unité. La négation unaire, `abs()` et `toQuantity(unit)` SHALL aussi accepter un objet FHIR Quantity (même reconnaissance que pour la comparaison : `value` décimal typé, unité UCUM `code`) et le traiter comme la Quantity littérale équivalente.

`toString()` et `convertsToString()` SHALL exiger une entrée singleton (plusieurs éléments : erreur). `String`, `Boolean`, `Integer`, `Long`, `Decimal`, `Date`, `DateTime`, `Time` et `Quantity` sont convertibles, tout autre élément non (collection vide pour `toString`, `false` pour `convertsToString`). Un `Decimal` SHALL toujours s'afficher avec une partie fractionnaire (`1.0` donne `'1.0'` ; un `Decimal` conserve ses décimales écrites, donc `1.50` donne `'1.50'`) et une `Quantity` SHALL s'afficher `valeur 'unité'`, unités calendaires comprises (`4 days` donne `"4 'day'"`). `toString` d'un `Decimal` non fini, ou dont l'écriture exigerait un exposant (échelle au-delà de 1000), SHALL donner une collection vide. `convertsToString` sur une entrée vide SHALL donner une collection vide.

#### Scenario: toInteger depuis une chaîne

- **WHEN** `'42'.toInteger()` est évalué
- **THEN** le résultat SHALL être `[Integer(42)]`

#### Scenario: toLong depuis un Integer

- **WHEN** `42.toLong()` est évalué
- **THEN** le résultat SHALL être `[Long(42)]`

#### Scenario: convertsToInteger sur chaîne non numérique

- **WHEN** `'abc'.convertsToInteger()` est évalué
- **THEN** le résultat SHALL être `[Bool(false)]`

#### Scenario: Conversion sur une entrée vide

- **WHEN** `{}.toInteger()` ou `{}.convertsToBoolean()` est évalué
- **THEN** le résultat SHALL être une collection vide

#### Scenario: Conversion sur plusieurs éléments

- **WHEN** `(1 | 2).toInteger()` ou `(1 | 2).convertsToInteger()` est évalué
- **THEN** l'évaluateur SHALL retourner une erreur

#### Scenario: toBoolean depuis une chaîne

- **WHEN** `'Yes'.toBoolean()` ou `'0.0'.toBoolean()` est évalué
- **THEN** le résultat SHALL être respectivement `[Bool(true)]` et `[Bool(false)]`

#### Scenario: toInteger depuis un Decimal

- **WHEN** `1.5.toInteger()` est évalué
- **THEN** le résultat SHALL être une collection vide

#### Scenario: toDecimal refuse la notation scientifique

- **WHEN** `'1e5'.toDecimal()` ou `'NaN'.toDecimal()` est évalué
- **THEN** le résultat SHALL être une collection vide

#### Scenario: toDate depuis une DateTime

- **WHEN** `@2020-01-02T10:00:00.toDate()` est évalué
- **THEN** le résultat SHALL être `[Date('2020-01-02')]`

#### Scenario: toQuantity depuis les types numériques et Boolean

- **WHEN** `1.toQuantity()`, `1.5.toQuantity()`, `true.toQuantity()` puis `4 'mg'.toQuantity()` sont évalués
- **THEN** les résultats SHALL être `1 '1'`, `1.5 '1'`, `1.0 '1'` et `4 'mg'` (Quantity inchangée)

#### Scenario: toQuantity depuis une chaîne

- **WHEN** `'4 days'.toQuantity()`, `'1.0'.toQuantity()` puis `'1 foo'.toQuantity()` sont évalués
- **THEN** les deux premiers SHALL donner `4 'day'` et `1.0 '1'`, le dernier une collection vide (unité sans guillemets non calendaire)

#### Scenario: toQuantity avec argument unité

- **WHEN** `4 'mg'.toQuantity('mg')`, `4 'mg'.toQuantity('g')`, `4 'mg'.toQuantity('m')`, `4 'mg'.toQuantity({})` puis `4 'mg'.toQuantity(1)` sont évalués
- **THEN** les résultats SHALL être `4 'mg'`, `0.004 'g'`, une collection vide (incompatible), une collection vide, puis `Err(EvalError::Type(...))` ; `4 'mg'.convertsToQuantity('g')` SHALL valoir `true` et `convertsToQuantity('m')` `false`

#### Scenario: Multiplication et division de Quantity

- **WHEN** `12 'cm' * 3 'cm'`, `1 'm' / 1 'm'`, `4 'g' / 2 'm'`, `6 / 3 's'`, `1 'm' / 0 'm'` et `1 year * 2 'm'` sont évalués
- **THEN** les résultats SHALL être `36 'cm2'`, `1 '1'`, `2 'g/m'`, `2 '1/s'`, une collection vide et une collection vide

#### Scenario: Négation et abs d'une Quantity

- **WHEN** `-(3 'mg')` et `(-3 'mg').abs()` sont évalués
- **THEN** les résultats SHALL être `-3 'mg'` et `3 'mg'`

#### Scenario: toQuantity sur plusieurs éléments

- **WHEN** `(1 | 2).toQuantity()` ou `(1 | 2).convertsToQuantity()` est évalué
- **THEN** l'évaluateur SHALL retourner une erreur

#### Scenario: Chaîne de chiffres au-delà de f64

- **WHEN** une chaîne de plus de 310 chiffres est passée à `toDecimal()` ou `toQuantity()`
- **THEN** le résultat SHALL être une collection vide, et `convertsToQuantity()` SHALL valoir `[Bool(false)]`

#### Scenario: toLong depuis les sources convertibles

- **WHEN** `'12'.toLong()`, `true.toLong()`, `1.5.toLong()` puis `{}.convertsToLong()` sont évalués
- **THEN** les résultats SHALL être `[Long(12)]`, `[Long(1)]`, une collection vide puis une collection vide ; `(1 | 2).toLong()` SHALL retourner une erreur

#### Scenario: toString d'un Decimal

- **WHEN** `1.0.toString()` puis `1.50.toString()` sont évalués
- **THEN** les résultats SHALL être `[String('1.0')]` puis `[String('1.50')]`

#### Scenario: Decimal à exposant

- **WHEN** `toString()` est évalué sur un `Decimal` de plus de 1000 décimales
- **THEN** le résultat SHALL être une collection vide

#### Scenario: Decimal non fini

- **WHEN** `toQuantity()` ou `toString()` est évalué sur un `Decimal` non fini (valeur `f64` au-delà de 1,8e308)
- **THEN** le résultat SHALL être une collection vide ; un littéral de plus de 38 chiffres significatifs (par exemple 400 chiffres entiers) SHALL être rejeté au parsing

#### Scenario: toString d'une Quantity

- **WHEN** `4 days.toString()` est évalué
- **THEN** le résultat SHALL être `[String("4 'day'")]`

#### Scenario: toString sur plusieurs éléments

- **WHEN** `(1 | 2).toString()` est évalué
- **THEN** l'évaluateur SHALL retourner une erreur ; `{}.toString()` et `{}.convertsToString()` SHALL donner une collection vide

#### Scenario: Quantités calendaires et UCUM

- **WHEN** `1 second = 1 's'`, `1 year = 1 'a'`, `1 year ~ 1 'a'`, `1 week = 7 'd'` puis `4 'g' ~ 4040 'mg'` sont évalués
- **THEN** les résultats SHALL être `[Bool(true)]`, vide, `[Bool(true)]`, `[Bool(true)]` puis `[Bool(true)]`

#### Scenario: Quantity FHIR contre littéral

- **WHEN** `Observation.valueQuantity` a `value` 185, `system` UCUM et `code` `[lb_av]`, puis `Observation.valueQuantity = 185 '[lb_av]'` et `Observation.valueQuantity = 185 'lbs'` sont évalués
- **THEN** le premier résultat SHALL être `[Bool(true)]` et le second une collection vide

#### Scenario: Unité contre absence d'unité

- **WHEN** une Quantity FHIR `{"value": 1}` sans unité est comparée à une Quantity FHIR `{"value": 1, "unit": "tablets"}`, dans les deux sens, avec `=` puis `~`
- **THEN** `=` SHALL donner une collection vide et `~` SHALL donner `[Bool(false)]` ; deux Quantity FHIR sans unité et de même valeur SHALL être égales

#### Scenario: Date calendaire invalide

- **WHEN** `'2023-02-29'.convertsToDate()` ou `'2020-01-01T10:00+14:30'.convertsToDateTime()` est évalué
- **THEN** le résultat SHALL être `[Bool(false)]`, et `'2024-02-29'.convertsToDate()` SHALL valoir `[Bool(true)]`

---
