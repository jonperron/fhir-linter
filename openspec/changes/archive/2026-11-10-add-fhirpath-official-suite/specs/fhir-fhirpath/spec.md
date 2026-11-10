## ADDED Requirements

### Requirement: Exécution de la suite de tests officielle FHIRPath

Le crate `fhir-fhirpath` SHALL fournir un test d'intégration exécutant `r5/fhirpath/tests-fhir-r5.xml` de `fhir-test-cases`. Chaque cas SHALL être évalué avec `evaluate` sur son `inputfile` (un nom de fichier nu, sans partie répertoire, sous `r5/`, JSON ou XML analysé par `fhir-parser` puis converti par `resource_to_value`) ou sur un contexte vide ; un attribut `context` SHALL être évalué d'abord et son résultat sert de contexte. Un cas réussit quand les sorties correspondent en type et en valeur (ordre conservé sauf `ordered="false"` ; `predicate="true"` convertit le résultat en booléen), ou, pour une expression `invalid`, quand l'analyse ou l'évaluation échoue avec une erreur autre que `EvalError::UndefinedFunction` (une fonction absente ne prouve pas l'invalidité). Une panique, une entrée non chargeable, un `inputfile` comportant une partie répertoire ou une erreur de contexte compte comme un échec. Les cas dont le mode (du test ou du groupe) est `cda`, `tx` ou `custom` SHALL être ignorés et comptés. Le test SHALL afficher réussites/exécutés, ignorés, total et chaque cas en échec, et SHALL échouer si les nombres de cas exécutés et ignorés diffèrent exactement de (1226, 121) (dérive du chargeur ou du filtre d'ignorance) ou si moins de 757 cas réussissent (plancher de régression). Le répertoire de fixtures SHALL pouvoir être surchargé par `FHIR_FIXTURES_DIR` ; si le fichier de la suite est introuvable, le test SHALL être ignoré, sauf en CI (`CI` défini) où il SHALL échouer ; toute autre erreur de lecture de ce fichier SHALL faire échouer le test.

#### Scenario: Réussites sous le plancher

- **WHEN** moins de 757 cas de la suite réussissent
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
