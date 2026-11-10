# Spec: fhir-fhirpath

## Purpose

Évaluateur FHIRPath en Rust, utilisé pour valider les invariants `constraint.expression` des StructureDefinitions FHIR R5.
Implémente un sous-ensemble de FHIRPath release 2 (2.0.0, http://hl7.org/fhirpath/N1/), version utilisée par FHIR R5, plus quelques ajouts ultérieurs ; les fonctions non implémentées retournent `EvalError::UndefinedFunction`.

## Requirements
### Requirement: API publique evaluate

Le système SHALL exposer `evaluate(expression: &str, context: &[Value]) -> Result<Collection, EvalError>` pour parser et évaluer une expression FHIRPath en une seule étape.

#### Scenario: Expression valide

- **WHEN** `evaluate` reçoit une expression FHIRPath syntaxiquement valide et un contexte non vide
- **THEN** il SHALL retourner `Ok(Collection)` contenant le résultat de l'évaluation

#### Scenario: Expression syntaxiquement invalide

- **WHEN** `evaluate` reçoit une expression malformée
- **THEN** il SHALL retourner `Err(EvalError::Parse(ParseError::Syntax(...)))`

#### Scenario: Contexte vide

- **WHEN** `evaluate` reçoit un contexte vide (`&[]`)
- **THEN** il SHALL retourner `Ok(vec![])` pour les expressions de navigation

#### Scenario: Expression déjà parsée

- **WHEN** `evaluate_parsed(&Expr, context)` reçoit un `Expr` produit par `parse`
- **THEN** il SHALL évaluer sans reparser, avec le même résultat que `evaluate` ; `fhir-definitions` met en cache le résultat de `parse` par `Constraint` (au plus un parse par contrainte)

---

### Requirement: API publique parse

Le système SHALL exposer `parse(expression: &str) -> Result<ast::Expr, ParseError>` pour parser une expression FHIRPath sans l'évaluer.

#### Scenario: Expression valide retourne un AST

- **WHEN** `parse` reçoit une expression FHIRPath syntaxiquement valide
- **THEN** il SHALL retourner `Ok(Expr)` représentant l'AST complet de l'expression

#### Scenario: Expression invalide retourne ParseError

- **WHEN** `parse` reçoit une expression syntaxiquement invalide
- **THEN** il SHALL retourner `Err(ParseError::Syntax(...))` avec un message décrivant l'erreur

---

### Requirement: Grammaire PEG FHIRPath release 2

Le système SHALL implémenter une grammaire PEG via `pest` couvrant la syntaxe de FHIRPath release 2 (2.0.0), version utilisée par FHIR R5, plus quelques ajouts ultérieurs (par ex. le littéral Long).

#### Scenario: Littéraux pris en charge

- **WHEN** l'expression contient un littéral (booléen, entier, Long `45L`, décimal, chaîne entre guillemets simples, date `@YYYY-MM-DD`, dateTime complet `@YYYY-MM-DDThh:mm:ss`, dateTime partiel `@YYYY-MM-DDT` / `@YYYY-MMT` / `@YYYYT`, heure `@Thh:mm:ss`, quantité `1.5 'mg'`)
- **THEN** le parser SHALL produire le nœud AST correspondant sans erreur

#### Scenario: Opérateurs binaires

- **WHEN** l'expression contient un opérateur (`+`, `-`, `*`, `/`, `div`, `mod`, `&`, `=`, `!=`, `~`, `!~`, `<`, `<=`, `>`, `>=`, `and`, `or`, `xor`, `implies`, `in`, `contains`, `is`, `as`, `|`)
- **THEN** le parser SHALL produire le nœud AST avec la bonne associativité et priorité

#### Scenario: Priorité de is/as et de l'union

- **WHEN** l'expression combine `|` avec `is` ou `as` (ex. `a | b is X`)
- **THEN** `is`/`as` SHALL lier plus fort que `|` (`a | (b is X)`)

#### Scenario: Limites de taille et d'imbrication

- **WHEN** l'expression dépasse 1 KiB ou plus de 16 niveaux d'imbrication de `(` / `[` (hors chaînes, backticks et commentaires)
- **THEN** `parse` SHALL retourner `Err(ParseError::Syntax(...))` sans dépasser la pile ; toute expression acceptée au maximum de ces limites (chaînes `a.a.a…`, `1+1+…`, parenthèses imbriquées) SHALL pouvoir être parsée, évaluée et libérée dans un thread de 2 Mo en debug

#### Scenario: Identifiants délimités par backticks

- **WHEN** l'expression contient un DELIMITEDIDENTIFIER (ex. ``text.`div` ``) dans un chemin, un nom de fonction ou un spécificateur de type
- **THEN** il SHALL être évalué exactement comme l'identifiant simple correspondant

#### Scenario: Séquences d'échappement

- **WHEN** une chaîne littérale ou un identifiant délimité contient une séquence d'échappement
- **THEN** le parser SHALL accepter l'ensemble ESC de la spécification : `` \` ``, `\'`, `\"`, `\\`, `\/`, `\f`, `\n`, `\r`, `\t`, `\uXXXX`

#### Scenario: Identifiants débutant par true/false

- **WHEN** l'expression contient un identifiant tel que `trueValue`
- **THEN** il SHALL être analysé comme identifiant et non comme littéral booléen

#### Scenario: Appels de fonctions

- **WHEN** l'expression contient un appel de fonction ou méthode (ex. `name.exists()`, `where(active = true)`)
- **THEN** le parser SHALL produire `Expr::Method` ou `Expr::FuncCall` avec les arguments corrects

---

### Requirement: AST Expr

Le système SHALL exposer un enum `Expr` couvrant toutes les constructions de la grammaire prise en charge.

#### Scenario: Variantes de l'AST

- **WHEN** un AST est construit
- **THEN** `Expr` SHALL couvrir : littéraux (`Null`, `Bool`, `Integer`, `Long`, `Decimal`, `String`, `Temporal`, `Quantity`), navigation (`DollarThis`, `DollarIndex`, `Ident`, `Dot`, `Index`), appels (`FuncCall`, `Method`), arithmétique (`Add`, `Sub`, `Mul`, `Div`, `DivInt`, `Mod`, `Concat`, `Neg`), comparaisons (`Eq`, `Neq`, `Lt`, `Lte`, `Gt`, `Gte`, `Equiv`, `NotEquiv`), booléens (`And`, `Or`, `Xor`, `Implies`), types (`Is`, `As`), ensembles (`Union`, `In`, `Contains`)

---

### Requirement: Type Value

Le système SHALL exposer un enum `Value` représentant les valeurs FHIRPath à l'exécution.

#### Scenario: Variantes couvertes

- **WHEN** un résultat d'évaluation est produit
- **THEN** `Value` SHALL avoir les variantes : `Null`, `Bool(bool)`, `Integer(i64)`, `Long(i64)`, `Decimal(f64)`, `String(Arc<str>)`, `Date(Arc<str>)`, `DateTime(Arc<str>)`, `Time(Arc<str>)`, `Quantity(f64, Arc<str>)`, `Object(Arc<IndexMap<Arc<str>, Collection>>)`

#### Scenario: Conversion depuis un Resource parser

- **WHEN** `resource_to_value(resource: &Resource) -> Value` est appelé
- **THEN** il SHALL retourner `Value::Object` représentant la ressource FHIR, utilisable comme contexte pour `evaluate`

#### Scenario: Conversion depuis un Node parser

- **WHEN** `from_parser_value(node: &fhir_parser::Value) -> Value` est appelé
- **THEN** il SHALL mapper chaque variante `fhir_parser::Value` vers la `Value` FHIRPath correspondante

---

### Requirement: Type Collection

Le système SHALL exposer `type Collection = Vec<Value>` comme type de retour de toute évaluation FHIRPath.

#### Scenario: Collection vide pour absence de résultat

- **WHEN** une expression FHIRPath ne sélectionne aucune valeur
- **THEN** `evaluate` SHALL retourner `Ok(vec![])` (collection vide)

---

### Requirement: Fonctions d'existence

Le système SHALL implémenter `empty()`, `exists()`, `exists(criteria)`, `all(criteria)`, `allTrue()`, `anyTrue()`, `allFalse()`, `anyFalse()`.

#### Scenario: empty sur collection vide

- **WHEN** `empty()` est évalué sur une collection vide
- **THEN** le résultat SHALL être `[Bool(true)]`

#### Scenario: exists avec critère

- **WHEN** `exists(x | x > 0)` est évalué sur une collection d'entiers
- **THEN** le résultat SHALL être `[Bool(true)]` si au moins un élément satisfait le critère

---

### Requirement: Fonctions de filtrage

Le système SHALL implémenter `where(criteria)`, `select(projection)`, `repeat(projection)`, `ofType(type)`, `is(type)`, `as(type)`.

L'argument de `ofType`, `is` et `as` est un identifiant de nom de type. La correspondance de type (partagée par `ofType`, les fonctions `is`/`as` et les opérateurs `is`/`as`) SHALL accepter les types primitifs FHIRPath ainsi qu'un objet ressource dont `resourceType` est exactement égal au nom de type. Elle ne SHALL pas gérer l'héritage (`Resource`, `DomainResource`), les noms de types complexes FHIR, ni les noms qualifiés (`FHIR.X`, `System.X`).

#### Scenario: where filtre les éléments

- **WHEN** `where(active = true)` est évalué sur une collection d'objets
- **THEN** le résultat SHALL contenir uniquement les éléments pour lesquels le critère est vrai

#### Scenario: ofType filtre par type FHIRPath

- **WHEN** `ofType(String)` est évalué sur une collection mixte
- **THEN** le résultat SHALL contenir uniquement les valeurs de type `String`

#### Scenario: ofType filtre par resourceType

- **WHEN** `ofType(Patient)` est évalué sur une collection de ressources
- **THEN** le résultat SHALL contenir uniquement les objets dont `resourceType` vaut exactement `Patient`

#### Scenario: is et as sur un seul élément

- **WHEN** `is(Patient)` ou `as(Patient)` est évalué sur une collection d'un seul élément
- **THEN** `is` SHALL retourner `[Bool(true)]` ou `[Bool(false)]`, et `as` SHALL retourner l'élément s'il correspond, sinon une collection vide

#### Scenario: is et as sur une collection vide

- **WHEN** `is(type)` ou `as(type)` est évalué sur une collection vide
- **THEN** le résultat SHALL être une collection vide

#### Scenario: is et as sur plusieurs éléments

- **WHEN** `is(type)` ou `as(type)` est évalué sur une collection de plus d'un élément
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))` (test officiel `testFHIRPathAsFunction21`)

---

### Requirement: Fonctions de sous-ensemble

Le système SHALL implémenter `first()`, `last()`, `tail()`, `skip(n)`, `take(n)`, `single()`, `count()`.

#### Scenario: first retourne le premier élément

- **WHEN** `first()` est évalué sur une collection non vide
- **THEN** le résultat SHALL être une collection contenant uniquement le premier élément

#### Scenario: single retourne une erreur si plusieurs éléments

- **WHEN** `single()` est évalué sur une collection de plus d'un élément
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`

#### Scenario: single sur collection vide

- **WHEN** `{}.single()` est évalué
- **THEN** le résultat SHALL être une collection vide

#### Scenario: Indexeur hors limites ou négatif

- **WHEN** `coll[i]` est évalué avec `i` négatif ou `i` >= la taille de la collection
- **THEN** le résultat SHALL être une collection vide (FHIRPath N1 §5.5), sans erreur

#### Scenario: repeat ne retourne que les éléments projetés

- **WHEN** `repeat(expr)` est évalué
- **THEN** le résultat SHALL contenir uniquement les éléments produits par la projection (jamais les éléments d'entrée) et SHALL s'arrêter quand plus aucun nouvel élément n'apparaît

#### Scenario: Concaténation de chaînes avec +

- **WHEN** `'a' + 'b'` est évalué
- **THEN** le résultat SHALL être `[String("ab")]`

#### Scenario: Égalité FHIRPath dans les opérateurs ensemblistes

- **WHEN** `union`, `distinct`, `in`, `contains`, `intersect`, `exclude`, `subsetOf`, `supersetOf` ou `repeat` comparent des valeurs
- **THEN** la comparaison SHALL utiliser l'égalité FHIRPath (`1 = 1.0` vaut vrai)

---

### Requirement: Fonctions de chaînes

Le système SHALL implémenter `toString()`, `length()`, `startsWith(s)`, `endsWith(s)`, `contains(s)`, `upper()`, `lower()`, `trim()`, `substring(start)`, `substring(start, length)`, `replace(pattern, substitution)`, `matches(regex, [flags])`, `matchesFull(regex, [flags])`, `replaceMatches(regex, substitution, [flags])`, `indexOf(s)`, `lastIndexOf(s)`, `split(separator)`, `join(separator)`, `encode(format)`, `decode(format)`, `escape(target)`, `unescape(target)`.

Le paramètre optionnel `flags` accepte `i` (insensible à la casse) et `m` (multiligne).
`escape`/`unescape` supportent les cibles `'html'` et `'json'`.

#### Scenario: length sur une chaîne

- **WHEN** `'hello'.length()` est évalué
- **THEN** le résultat SHALL être `[Integer(5)]`

#### Scenario: matches avec regex

- **WHEN** `'abc123'.matches('[0-9]+')` est évalué
- **THEN** le résultat SHALL être `[Bool(true)]`

#### Scenario: matches avec flag insensible à la casse

- **WHEN** `'Hello'.matches('hello', 'i')` est évalué
- **THEN** le résultat SHALL être `[Bool(true)]`

#### Scenario: matchesFull exige correspondance complète

- **WHEN** `'hello world'.matchesFull('hello')` est évalué
- **THEN** le résultat SHALL être `[Bool(false)]`

#### Scenario: matchesFull reste ancré avec le flag m

- **WHEN** `'a\nb'.matchesFull('a', 'm')` est évalué
- **THEN** le résultat SHALL être `[Bool(false)]` (ancrage sur tout le texte, pas par ligne)

#### Scenario: decode rejette les entrées invalides

- **WHEN** `decode('base64')` ou `decode('urlbase64')` reçoit des caractères hors alphabet (hors espaces ASCII ignorés) ou un contenu non UTF-8
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`

#### Scenario: lastIndexOf retourne la dernière occurrence

- **WHEN** `'abcabc'.lastIndexOf('b')` est évalué
- **THEN** le résultat SHALL être `[Integer(4)]`

#### Scenario: escape HTML

- **WHEN** `'<b>'.escape('html')` est évalué
- **THEN** le résultat SHALL être `[String('&lt;b&gt;')]`

#### Scenario: substring longueur négative ou zéro

- **WHEN** `'hello'.substring(0, -1)` ou `'hello'.substring(0, 0)` est évalué
- **THEN** le résultat SHALL être `[String('')]` (chaîne vide, pas collection vide)

---

### Requirement: Fonctions mathématiques

Le système SHALL implémenter `abs()`, `ceiling()`, `floor()`, `round()`, `round(precision)`, `sqrt()`, `exp()`, `ln()`, `log(base)`, `power(exponent)`, `truncate()`.

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

---

### Requirement: Fonctions de conversion

Le système SHALL implémenter `toInteger()`, `toLong()`, `toDecimal()`, `toBoolean()`, `convertsToInteger()`, `convertsToLong()`, `convertsToDecimal()`, `convertsToBoolean()`, `toDate()`, `toDateTime()`, `toTime()`, `toQuantity()`, `convertsToDate()`, `convertsToDateTime()`, `convertsToTime()`, `convertsToQuantity()`, `toString()`.

`toLong()` retourne une valeur `Long` (type distinct d'`Integer`).

#### Scenario: toInteger depuis une chaîne

- **WHEN** `'42'.toInteger()` est évalué
- **THEN** le résultat SHALL être `[Integer(42)]`

#### Scenario: toLong depuis un Integer

- **WHEN** `42.toLong()` est évalué
- **THEN** le résultat SHALL être `[Long(42)]`

#### Scenario: convertsToInteger sur chaîne non numérique

- **WHEN** `'abc'.convertsToInteger()` est évalué
- **THEN** le résultat SHALL être `[Bool(false)]`

---

### Requirement: Fonctions de collections

Le système SHALL implémenter `distinct()`, `isDistinct()`, `subsetOf(other)`, `supersetOf(other)`, `intersect(other)`, `exclude(other)`, `combine(other, [preserveOrder])`, `coalesce(values...)`, ainsi que l'opérateur `|` (union).

`combine()` diffère de `|` : il conserve les doublons.
`coalesce()` retourne le premier résultat non vide parmi ses arguments.

#### Scenario: distinct élimine les doublons

- **WHEN** `distinct()` est évalué sur une collection avec doublons
- **THEN** le résultat SHALL contenir chaque valeur unique une seule fois, dans l'ordre d'apparition

#### Scenario: intersect retourne les éléments communs

- **WHEN** `intersect(other)` est évalué
- **THEN** le résultat SHALL contenir uniquement les éléments présents dans les deux collections

#### Scenario: combine conserve les doublons

- **WHEN** `(1 | 2).combine(2 | 3)` est évalué
- **THEN** le résultat SHALL être `[1, 2, 2, 3]` (doublons conservés)

#### Scenario: coalesce retourne le premier non vide

- **WHEN** `coalesce({}, 'second', 'third')` est évalué
- **THEN** le résultat SHALL être `[String('second')]`

---

### Requirement: Fonctions d'agrégation

Le système SHALL implémenter `aggregate(aggregator, init)`, `sum()`, `min()`, `max()`, `avg()`, `sort()`, `sort(keySelector)`.

#### Scenario: sum sur une collection d'entiers

- **WHEN** `sum()` est évalué sur `[Integer(1), Integer(2), Integer(3)]`
- **THEN** le résultat SHALL être `[Integer(6)]`

#### Scenario: max sur une collection de décimaux

- **WHEN** `max()` est évalué sur une collection de valeurs numériques
- **THEN** le résultat SHALL retourner la valeur maximale

#### Scenario: avg calcule la moyenne

- **WHEN** `(1 | 3 | 5).avg()` est évalué
- **THEN** le résultat SHALL être `[Decimal(3.0)]`

#### Scenario: sort tri naturel

- **WHEN** `(3 | 1 | 2).sort()` est évalué
- **THEN** le résultat SHALL être `[1, 2, 3]` (ordre croissant)

---

### Requirement: Fonctions de navigation

Le système SHALL implémenter `children()` et `descendants()`.

#### Scenario: children retourne les champs directs d'un objet

- **WHEN** `children()` est évalué sur un `Value::Object`
- **THEN** le résultat SHALL contenir toutes les valeurs des champs de premier niveau de l'objet

#### Scenario: descendants retourne tous les nœuds imbriqués

- **WHEN** `descendants()` est évalué sur un objet avec des objets imbriqués
- **THEN** le résultat SHALL contenir récursivement tous les nœuds enfants et leurs descendants
- **THEN** l'ordre SHALL être un parcours en profondeur préfixé (parent avant enfants, dans l'ordre des champs)

---

### Requirement: Fonctions diverses

Le système SHALL implémenter `not()`, `iif(condition, trueResult, otherwiseResult)`, `trace(name)`, `hasValue()`, `getValue()`, `type()`. Les autres fonctions diverses de FHIRPath ne sont pas implémentées (voir « Limitations connues »).

#### Scenario: not inverse un booléen

- **WHEN** `true.not()` est évalué
- **THEN** le résultat SHALL être `[Bool(false)]`

#### Scenario: iif retourne la branche correcte

- **WHEN** `iif(true, 'yes', 'no')` est évalué
- **THEN** le résultat SHALL être `[String("yes")]`

---

### Requirement: Préfixe du nom de type de ressource

Le système SHALL permettre à une expression de commencer par le nom de type de la ressource de contexte.

#### Scenario: Nom de type correspondant

- **WHEN** `CodeSystem.content` est évalué sur une `CodeSystem` sans champ nommé `CodeSystem`
- **THEN** `evaluate` SHALL retourner la valeur de `content`

#### Scenario: Nom de type différent

- **WHEN** `Patient.id` est évalué sur une `CodeSystem`
- **THEN** `evaluate` SHALL retourner une collection vide

#### Scenario: Nom de type après le premier segment

- **WHEN** `Patient.Patient.id` est évalué sur une `Patient` sans champ nommé `Patient`
- **THEN** `evaluate` SHALL retourner une collection vide, le préfixe n'étant admis qu'en tête

#### Scenario: Champ réel prioritaire

- **WHEN** la ressource possède un champ dont le nom est égal au nom demandé
- **THEN** ce champ SHALL être retourné au lieu de la ressource elle-même

---

### Requirement: Erreurs d'évaluation

Le système SHALL exposer `EvalError` couvrant toutes les erreurs d'exécution possibles.

#### Scenario: Variantes d'EvalError

- **WHEN** une erreur se produit à l'évaluation
- **THEN** `EvalError` SHALL avoir les variantes : `Type(String)` (erreur de type), `UndefinedFunction(String)` (fonction inconnue), `Arity { name, expected, got }` (mauvais nombre d'arguments), `DivisionByZero`, `Parse(ParseError)` (erreur de parse propagée)

---

### Requirement: Limitations connues

Le système SHALL documenter les limitations suivantes de l'implémentation actuelle.

#### Scenario: $index non supporté

- **WHEN** une expression utilise `$index` dans un contexte d'itération
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::UndefinedFunction("$index"))`

#### Scenario: aggregate sans $total

- **WHEN** `aggregate()` est utilisé avec une référence à `$total`
- **THEN** l'accumulateur `$total` ne sera pas accessible (comportement non défini dans l'implémentation actuelle)

#### Scenario: Fonctions non implémentées retournent UndefinedFunction

- **WHEN** `resolve()`, `memberOf()`, `conformsTo()`, `today()`, `now()`, `timeOfDay()`, `lowBoundary()`, `highBoundary()`, `precision()`, `htmlChecks()` ou `comparable()` est évalué
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::UndefinedFunction(name))` où `name` est le nom de la fonction
- **AND** il SHALL NOT retourner de résultat deviné (collection vide, booléen), afin que le validateur signale la contrainte comme non vérifiée (`INVARIANT_SKIPPED`)

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

