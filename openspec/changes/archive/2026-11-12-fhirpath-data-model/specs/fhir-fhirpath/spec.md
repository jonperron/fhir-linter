## ADDED Requirements

### Requirement: Typage des primitives par le type déclaré

Le système SHALL exposer `resource_to_typed_value(resource, child_element: &ChildElement) -> Value`, où `ChildElement` est `dyn Fn(&str, &str) -> Option<(Arc<str>, Arc<str>)>` (chemin de définition du parent, nom de l'enfant) vers (chemin de définition, code de type). Une chaîne dont le type déclaré est `date` SHALL devenir `Value::Date`, `dateTime` et `instant` `Value::DateTime`, `time` `Value::Time` (FHIRPath R5 §2.1.9.1). Une ressource imbriquée (contained, entrée de Bundle) SHALL être typée par son propre `resourceType`. `resource_to_value` et `from_parser_value` SHALL rester non typées (les dates restent des `String`).

#### Scenario: Date typée par le Registry

- **WHEN** `birthDate` est évalué sur un Patient converti par `resource_to_typed_value` avec `Registry::child_element`
- **THEN** le résultat SHALL être `[Date(...)]` et `birthDate < @2000-01-01` SHALL être comparé comme une date

#### Scenario: Conversion non typée

- **WHEN** le même Patient est converti par `resource_to_value`
- **THEN** `birthDate` SHALL être une `String`

### Requirement: Représentation des primitives FHIR

`Value` SHALL avoir une variante `Primitive(Box<Value>, Arc<IndexMap<Arc<str>, Collection>>)` portant la valeur et les enfants `id`/`extension` d'une primitive FHIR. En JSON, `_x` SHALL être fusionné dans la primitive `x`, appariés par indice pour les tableaux ; `_x` seul SHALL donner une primitive sans valeur (`Value::Null`) ; un `_x` qui n'est pas un objet ne SHALL rien porter. En XML, un élément de type primitif portant des enfants SHALL devenir une `Primitive` (conversion typée). La représentation scindée ne SHALL jamais être visible : aucune clé `_x` n'est navigable.

Les opérateurs et les fonctions de chaînes, mathématiques et de conversion SHALL voir la valeur ; une primitive sans valeur SHALL être traitée comme une entrée vide. La navigation, `extension()`, `children()` et `descendants()` SHALL voir `id` et `extension`.

#### Scenario: Extension d'une primitive JSON

- **WHEN** `birthDate.extension.url` est évalué sur `{"birthDate": "1970-03-30", "_birthDate": {"extension": [{"url": "u"}]}}`
- **THEN** le résultat SHALL être `['u']` et `birthDate` SHALL toujours se comparer comme sa valeur

#### Scenario: Tableau apparié par indice

- **WHEN** `given[1].extension.exists()` est évalué sur `{"given": ["a", "b"], "_given": [null, {"extension": [...]}]}`
- **THEN** le résultat SHALL être `[Bool(true)]` et `given[0].extension.exists()` SHALL être `[Bool(false)]`

#### Scenario: Primitive sans valeur

- **WHEN** `status.length()` est évalué sur `{"_status": {"extension": [...]}}`
- **THEN** le résultat SHALL être une collection vide, et `status.extension.exists()` SHALL être `[Bool(true)]`

#### Scenario: _x non objet

- **WHEN** `status.exists()` est évalué sur `{"_status": 5}`
- **THEN** le résultat SHALL être `[Bool(false)]`

## MODIFIED Requirements

### Requirement: Type Value

Le système SHALL exposer un enum `Value` représentant les valeurs FHIRPath à l'exécution.

#### Scenario: Variantes couvertes

- **WHEN** un résultat d'évaluation est produit
- **THEN** `Value` SHALL avoir les variantes : `Null`, `Bool(bool)`, `Integer(i64)`, `Long(i64)`, `Decimal(f64)`, `String(Arc<str>)`, `Date(Arc<str>)`, `DateTime(Arc<str>)`, `Time(Arc<str>)`, `Quantity(f64, Arc<str>)`, `Object(Arc<IndexMap<Arc<str>, Collection>>)`, `Primitive(Box<Value>, Arc<IndexMap<Arc<str>, Collection>>)`

#### Scenario: Conversion depuis un Resource parser

- **WHEN** `resource_to_value(resource: &Resource) -> Value` est appelé
- **THEN** il SHALL retourner `Value::Object` représentant la ressource FHIR, utilisable comme contexte pour `evaluate`, sans typage des primitives (équivalent à `resource_to_typed_value` avec un `child_element` qui retourne toujours `None`)

#### Scenario: Conversion depuis un Node parser

- **WHEN** `from_parser_value(node: &fhir_parser::Value) -> Vec<Value>` est appelé
- **THEN** il SHALL mapper chaque variante `fhir_parser::Value` vers la `Value` FHIRPath correspondante, sans typage des primitives

#### Scenario: null JSON exclu

- **WHEN** un tableau JSON contient `null`, ou un champ ne contient que `null`
- **THEN** les `null` ne SHALL pas être des éléments de la collection (sauf s'ils sont appariés à un `_x` objet, qui donne une primitive sans valeur) et un champ ne contenant que `null` SHALL être absent de l'objet

### Requirement: Fonctions de navigation

Le système SHALL implémenter `children()` et `descendants()`. Les deux SHALL exclure le champ synthétique `resourceType` et SHALL inclure les enfants `id`/`extension` d'une `Value::Primitive`.

#### Scenario: children retourne les champs directs d'un objet

- **WHEN** `children()` est évalué sur un `Value::Object`
- **THEN** le résultat SHALL contenir toutes les valeurs des champs de premier niveau de l'objet, sauf `resourceType`

#### Scenario: descendants retourne tous les nœuds imbriqués

- **WHEN** `descendants()` est évalué sur un objet avec des objets imbriqués
- **THEN** le résultat SHALL contenir récursivement tous les nœuds enfants et leurs descendants
- **THEN** l'ordre SHALL être un parcours en profondeur préfixé (parent avant enfants, dans l'ordre des champs)

#### Scenario: Extensions d'une primitive parmi les descendants

- **WHEN** `descendants()` est évalué sur une ressource dont une primitive porte `_x.extension`
- **THEN** le résultat SHALL contenir ces extensions, et aucune valeur `resourceType`

### Requirement: Fonctions diverses

Le système SHALL implémenter `not()`, `iif(condition, trueResult, otherwiseResult)`, `trace(name)`, `hasValue()`, `getValue()`, `type()`. Les autres fonctions diverses de FHIRPath ne sont pas implémentées (voir « Limitations connues »).

La condition de `iif` SHALL être un booléen seul ou une collection vide, sinon `Err(EvalError::Type(...))` ; `iif` appelé sur une collection de plusieurs éléments SHALL retourner `Err(EvalError::Type(...))`. `not()` SHALL appliquer l'évaluation singleton dans un contexte booléen.

`hasValue()` SHALL retourner `[Bool(true)]` pour une seule primitive portant une valeur, une collection vide pour une seule valeur complexe, et `[Bool(false)]` pour une primitive sans valeur, plusieurs éléments ou aucun élément. Cela s'écarte du texte R5 (« otherwise empty ») parce que les contraintes R5 `exp-2`/`per-1` et la suite officielle attendent `false`.

#### Scenario: not inverse un booléen

- **WHEN** `true.not()` est évalué
- **THEN** le résultat SHALL être `[Bool(false)]`

#### Scenario: iif retourne la branche correcte

- **WHEN** `iif(true, 'yes', 'no')` est évalué
- **THEN** le résultat SHALL être `[String("yes")]`

#### Scenario: iif avec une condition invalide

- **WHEN** `iif(1 | 2 | 3, true, false)` est évalué
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`

#### Scenario: hasValue sur une primitive

- **WHEN** `birthDate.hasValue()` est évalué sur un Patient avec `birthDate`, puis sans `birthDate`, puis avec `_birthDate` seul
- **THEN** les résultats SHALL être `[Bool(true)]`, `[Bool(false)]` puis `[Bool(false)]`

#### Scenario: hasValue sur une valeur complexe ou plusieurs éléments

- **WHEN** `name.hasValue()` est évalué sur un Patient avec un seul `name`, puis `name.given.hasValue()` avec deux prénoms
- **THEN** les résultats SHALL être une collection vide puis `[Bool(false)]`

### Requirement: Égalité et équivalence

L'opérateur `=` SHALL comparer deux collections élément par élément dans l'ordre ; des longueurs différentes donnent `[Bool(false)]` et une collection vide d'un côté donne une collection vide. Les objets SHALL être comparés propriété par propriété, récursivement. Un objet (valeur complexe) comparé à une primitive SHALL donner `[Bool(false)]` ; une primitive FHIR portant `id`/`extension` (`Value::Primitive`) SHALL être comparée par sa valeur.

L'opérateur `~` SHALL ne jamais retourner de collection vide : `{} ~ {}` vaut `true` et un seul côté vide vaut `false`. Il SHALL ignorer l'ordre des collections, comparer les chaînes sans tenir compte de la casse en traitant chaque caractère d'espacement comme une espace, sans fusionner les suites d'espaces, arrondir les décimaux à la précision de l'opérande le moins précis, et ignorer `id` dans les objets.

Les dates, date-heures et heures SHALL être comparées précision par précision (secondes et millisecondes comptent pour une seule précision), les fuseaux étant normalisés en UTC. Une précision différente avec des parties communes égales SHALL donner une collection vide, de même qu'un fuseau présent d'un seul côté, y compris une `Date` comparée à une `DateTime` avec fuseau. `sort()`, `min()` et `max()` SHALL continuer d'ordonner les date-heures par leur texte.

#### Scenario: Collections de longueurs différentes

- **WHEN** `(1 | 2) ~ (1 | 2 | 3)` ou `(1 | 1) = (1 | 2 | {})` est évalué
- **THEN** le résultat SHALL être `[Bool(false)]`

#### Scenario: Collections dans un ordre différent

- **WHEN** `(1 | 2) = (2 | 1)` puis `(1 | 2 | 3) ~ (3 | 2 | 1)` sont évalués
- **THEN** les résultats SHALL être `[Bool(false)]` puis `[Bool(true)]`

#### Scenario: Équivalence avec collections vides

- **WHEN** `{} ~ {}` puis `1 ~ {}` sont évalués
- **THEN** les résultats SHALL être `[Bool(true)]` puis `[Bool(false)]`

#### Scenario: Équivalence de chaînes

- **WHEN** `'a B\tc' ~ 'A b c'` puis `'a  b' ~ 'a b'` sont évalués
- **THEN** les résultats SHALL être `[Bool(true)]` puis `[Bool(false)]`

#### Scenario: Équivalence de décimaux

- **WHEN** `1.2 / 1.8 ~ 0.67` est évalué
- **THEN** le résultat SHALL être `[Bool(true)]` (arrondi à la précision de `0.67`)

#### Scenario: Date-heures avec fuseaux différents

- **WHEN** `@2012-04-15T15:00:00+02:00 = @2012-04-15T16:00:00+03:00` est évalué
- **THEN** le résultat SHALL être `[Bool(true)]`

#### Scenario: Secondes et millisecondes

- **WHEN** `@2012-04-15T15:30:31 = @2012-04-15T15:30:31.0` est évalué
- **THEN** le résultat SHALL être `[Bool(true)]`

#### Scenario: Précisions différentes

- **WHEN** `@2012-01 = @2012` ou `@2012-04-15 = @2012-04-15T10:00:00` est évalué
- **THEN** le résultat SHALL être une collection vide

#### Scenario: Fuseau d'un seul côté

- **WHEN** `@2012-04-15T15:00:00Z != @2012-04-15T10:00:00` ou `@2012-04-16 = @2012-04-15T23:00:00-05:00` est évalué
- **THEN** le résultat SHALL être une collection vide

#### Scenario: Valeur complexe comparée à une primitive

- **WHEN** `name.first() = 'Peter'` puis `name.first() != 'Peter'` sont évalués sur un Patient dont `name` est un HumanName
- **THEN** les résultats SHALL être `[Bool(false)]` puis `[Bool(true)]`

### Requirement: Exécution de la suite de tests officielle FHIRPath

Le crate `fhir-fhirpath` SHALL fournir un test d'intégration exécutant `r5/fhirpath/tests-fhir-r5.xml` de `fhir-test-cases`. Chaque cas SHALL être évalué avec `evaluate` sur son `inputfile` (un nom de fichier nu, sans partie répertoire, sous `r5/`, JSON ou XML analysé par `fhir-parser` puis converti par `resource_to_typed_value` avec `Registry::child_element` du Registry R5 (sans navigation des choix)) ou sur un contexte vide ; un attribut `context` SHALL être évalué d'abord et son résultat sert de contexte. Un cas réussit quand les sorties correspondent en type et en valeur (ordre conservé sauf `ordered="false"` ; `predicate="true"` convertit le résultat en booléen), ou, pour une expression `invalid`, quand l'analyse ou l'évaluation échoue avec une erreur autre que `EvalError::UndefinedFunction` (une fonction absente ne prouve pas l'invalidité). Une panique, une entrée non chargeable, un `inputfile` comportant une partie répertoire ou une erreur de contexte compte comme un échec. Les cas dont le mode (du test ou du groupe) est `cda`, `tx` ou `custom` SHALL être ignorés et comptés. Le test SHALL afficher réussites/exécutés, ignorés, total et chaque cas en échec, et SHALL échouer si les nombres de cas exécutés et ignorés diffèrent exactement de (1226, 121) (dérive du chargeur ou du filtre d'ignorance) ou si moins de 877 cas réussissent (plancher de régression). Le répertoire de fixtures SHALL pouvoir être surchargé par `FHIR_FIXTURES_DIR` ; si le fichier de la suite est introuvable, le test SHALL être ignoré, sauf en CI (`CI` défini) où il SHALL échouer ; toute autre erreur de lecture de ce fichier SHALL faire échouer le test.

#### Scenario: Réussites sous le plancher

- **WHEN** moins de 877 cas de la suite réussissent
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
