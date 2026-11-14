## MODIFIED Requirements

### Requirement: Exécution de la suite de tests officielle FHIRPath

Le crate `fhir-fhirpath` SHALL fournir un test d'intégration exécutant `r5/fhirpath/tests-fhir-r5.xml` de `fhir-test-cases`. Chaque cas SHALL être évalué avec `evaluate_parsed_with_environment` sur son `inputfile` (passé aussi comme `%resource` ; un nom de fichier nu, sans partie répertoire, sous `r5/`, JSON ou XML analysé par `fhir-parser` puis converti par `resource_to_typed_value` avec `Registry::child_element` du Registry R5, et avec le crochet `is_subtype` issu du Registry (les éléments choix naviguent par le Registry comme dans le validateur)) ou sur un contexte vide ; un attribut `context` SHALL être évalué d'abord et son résultat sert de contexte. Un cas réussit quand les sorties correspondent en type et en valeur (ordre conservé sauf `ordered="false"` ; `predicate="true"` convertit le résultat en booléen), ou, pour une expression `invalid`, quand l'analyse ou l'évaluation échoue avec une erreur autre que `EvalError::UndefinedFunction` (une fonction absente ne prouve pas l'invalidité). Une panique, une entrée non chargeable, un `inputfile` comportant une partie répertoire ou une erreur de contexte compte comme un échec. Les cas dont le mode (du test ou du groupe) est `cda`, `tx` ou `custom` SHALL être ignorés et comptés. Le test SHALL afficher réussites/exécutés, ignorés, total et chaque cas en échec, et SHALL échouer si les nombres de cas exécutés et ignorés diffèrent exactement de (1226, 121) (dérive du chargeur ou du filtre d'ignorance) ou si moins de 992 cas réussissent (plancher de régression). Le répertoire de fixtures SHALL pouvoir être surchargé par `FHIR_FIXTURES_DIR` ; si le fichier de la suite est introuvable, le test SHALL être ignoré, sauf en CI (`CI` défini) où il SHALL échouer ; toute autre erreur de lecture de ce fichier SHALL faire échouer le test.

#### Scenario: Réussites sous le plancher

- **WHEN** moins de 992 cas de la suite réussissent
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

### Requirement: Typage des primitives par le type déclaré

Le système SHALL exposer `resource_to_typed_value(resource, child_element: &ChildElement) -> Value`, où `ChildElement` est `dyn Fn(&str, &str) -> Option<(Arc<str>, Arc<str>)>` (chemin de définition du parent, nom de l'enfant) vers (chemin de définition, code de type). Une chaîne dont le type déclaré est `date` SHALL devenir `Value::Date`, `dateTime` et `instant` `Value::DateTime`, `time` `Value::Time` (FHIRPath R5 §2.1.9.1). Une ressource imbriquée (contained, entrée de Bundle) SHALL être typée par son propre `resourceType`. Le système SHALL aussi exposer `node_to_typed_value(value, element, child, child_element)`, qui convertit un nœud et son élément JSON `_nom` avec le même typage que `resource_to_typed_value`. Les valeurs construites depuis une `Resource` XML (`from_xml`) SHALL aussi typer le texte `boolean`, `integer`, `unsignedInt`, `positiveInt` et `decimal` par le type déclaré de l'élément, comme les valeurs JSON, selon les regex officielles ; un texte qui ne correspond pas SHALL rester une `String`. Un élément XML de type `Resource` enveloppant une ressource (`<contained><Patient>`) SHALL être converti avec `resourceType` égal au nom de l'élément enveloppé et ses enfants typés contre cette ressource. Les types System (`System.Boolean`, etc.) SHALL garder les règles System sur les valeurs typées. Le système SHALL exposer l'assistant public `typed_xml_text`. `resource_to_value` et `from_parser_value` SHALL rester non typées (les dates restent des `String`).

#### Scenario: Date typée par le Registry

- **WHEN** `birthDate` est évalué sur un Patient converti par `resource_to_typed_value` avec `Registry::child_element`
- **THEN** le résultat SHALL être `[Date(...)]` et `birthDate < @2000-01-01` SHALL être comparé comme une date

#### Scenario: Conversion non typée

- **WHEN** le même Patient est converti par `resource_to_value`
- **THEN** `birthDate` SHALL être une `String`

#### Scenario: Conversion d'un nœud

- **WHEN** un nœud et son `_nom` sont convertis par `node_to_typed_value`
- **THEN** le résultat SHALL être typé comme par `resource_to_typed_value`

#### Scenario: Primitives XML typées

- **WHEN** `active` et `multipleBirthInteger` sont évalués sur un Patient XML converti par `resource_to_typed_value`
- **THEN** le résultat SHALL être `Bool(true)` et un `Integer`, et un texte `decimal` non conforme SHALL rester une `String`

#### Scenario: Ressource XML enveloppée

- **WHEN** `contained.resourceType` est évalué sur un Patient XML portant `<contained><Patient>`
- **THEN** le résultat SHALL être `Patient` et les enfants de la ressource contenue SHALL être typés contre `Patient`
