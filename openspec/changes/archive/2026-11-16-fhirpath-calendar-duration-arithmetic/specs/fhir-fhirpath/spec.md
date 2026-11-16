## ADDED Requirements

### Requirement: Arithmétique de dates avec durées calendaires

Le système SHALL implémenter N2 §6.6.7/§6.6.8 : `Date`, `DateTime` et `Time` `+`/`-` une `Quantity` de durée calendaire (mots-clés `year`..`millisecond` au singulier ou pluriel, ou UCUM `'d'`, `'wk'`, `'h'`, `'min'`, `'s'`, `'ms'` ; `'month'` etc. entre guillemets sont acceptés comme le mot-clé). Le résultat SHALL conserver la précision et le texte du fuseau de la valeur. Un décalage en mois ou années SHALL ramener le jour à la fin du mois (années bissextiles comprises). Un montant au-dessus des secondes SHALL perdre ses décimales (`7.7 days` donne 7). Une unité plus fine que la valeur SHALL être convertie à la précision de la valeur, par troncature (`@2019-03-01 + 23 hours` inchangé ; `@2014 + 24 months` donne `@2016`). Un `Time` SHALL boucler modulo 24 h. Les fractions SHALL être réécrites sur 3 chiffres.

Les cas suivants SHALL être des erreurs : UCUM `'a'` et `'mo'`, unités hors temps (`'cm'`), nombre sans unité, `Quantity + Date` (sens inverse), jours/semaines sur une valeur d'année ou de mois, unités d'un jour ou plus sur un `Time` (non définis par N2), montant NaN ou infini, résultat hors années 0001 à 9999.

#### Scenario: Décalage en mois avec ajustement du jour

- **WHEN** `@2020-01-31 + 1 month` puis `@2019-03-31 - 1 month` sont évalués
- **THEN** les résultats SHALL être `@2020-02-29` et `@2019-02-28`

#### Scenario: Troncature à la précision de la valeur

- **WHEN** `@2019-03-01 + 23 hours`, `@2014 + 24 months` et `@2019-03-01 + 7.7 days` sont évalués
- **THEN** les résultats SHALL être `@2019-03-01`, `@2016` et `@2019-03-08`

#### Scenario: Time modulo 24 h

- **WHEN** `@T23:30:00 + 1 hour` est évalué
- **THEN** le résultat SHALL être `@T00:30:00`

#### Scenario: Unités et opérandes invalides

- **WHEN** `@2019 + 1 'a'`, `@2019-01-01 + 1 'cm'`, `@2019-01-01 + 1`, `1 day + @2019-01-01`, `@2019 + 1 day`, `@T10:00 + 1 day` ou `@9999-12-31 + 1 day` est évalué
- **THEN** l'évaluateur SHALL retourner une erreur

## MODIFIED Requirements

### Requirement: Exécution de la suite de tests officielle FHIRPath

Le crate `fhir-fhirpath` SHALL fournir un test d'intégration exécutant `r5/fhirpath/tests-fhir-r5.xml` de `fhir-test-cases`. Chaque cas SHALL être évalué avec `evaluate_parsed_with_environment` sur son `inputfile` (passé aussi comme `%resource` ; un nom de fichier nu, sans partie répertoire, sous `r5/`, JSON ou XML analysé par `fhir-parser` puis converti par `resource_to_typed_value` avec `Registry::child_element` du Registry R5, et avec le crochet `is_subtype` issu du Registry (les éléments choix naviguent par le Registry comme dans le validateur)) ou sur un contexte vide ; un attribut `context` SHALL être évalué d'abord et son résultat sert de contexte. Un cas réussit quand les sorties correspondent en type et en valeur (ordre conservé sauf `ordered="false"` ; `predicate="true"` convertit le résultat en booléen), ou, pour une expression `invalid`, quand l'analyse ou l'évaluation échoue avec une erreur autre que `EvalError::UndefinedFunction` (une fonction absente ne prouve pas l'invalidité). Une panique, une entrée non chargeable, un `inputfile` comportant une partie répertoire ou une erreur de contexte compte comme un échec. Les cas dont le mode (du test ou du groupe) est `cda`, `tx` ou `custom` SHALL être ignorés et comptés. Le test SHALL afficher réussites/exécutés, ignorés, total et chaque cas en échec, et SHALL échouer si les nombres de cas exécutés et ignorés diffèrent exactement de (1226, 121) (dérive du chargeur ou du filtre d'ignorance) ou si moins de 1106 cas réussissent (plancher de régression). Le répertoire de fixtures SHALL pouvoir être surchargé par `FHIR_FIXTURES_DIR` ; si le fichier de la suite est introuvable, le test SHALL être ignoré, sauf en CI (`CI` défini) où il SHALL échouer ; toute autre erreur de lecture de ce fichier SHALL faire échouer le test.

#### Scenario: Réussites sous le plancher

- **WHEN** moins de 1106 cas de la suite réussissent
- **THEN** le test SHALL échouer

#### Scenario: Dérive du nombre de cas

- **WHEN** le nombre de cas exécutés diffère de 1226 ou le nombre de cas ignorés diffère de 121
- **THEN** le test SHALL échouer

#### Scenario: Expression invalide évaluée sans erreur

- **WHEN** un cas marqué `invalid` est analysé et évalué sans erreur
- **THEN** le cas SHALL compter comme un échec

#### Scenario: Expression invalide utilisant une fonction non implémentée

- **WHEN** l'évaluation d'un cas marqué `invalid` échoue avec `EvalError::UndefinedFunction`
- **THEN** le cas SHALL compter comme un échec

#### Scenario: Fichier d'entrée avec une partie répertoire

- **WHEN** l'`inputfile` d'un cas n'est pas un nom de fichier nu (par exemple `../patient.json`)
- **THEN** le cas SHALL compter comme un échec sans lire le fichier

#### Scenario: Sous-module absent

- **WHEN** le fichier de la suite est introuvable
- **THEN** le test SHALL se terminer sans échec si `CI` n'est pas défini, et SHALL échouer si `CI` est défini

#### Scenario: Erreur de lecture de la suite

- **WHEN** la lecture du fichier de la suite échoue pour une raison autre que son absence
- **THEN** le test SHALL échouer

