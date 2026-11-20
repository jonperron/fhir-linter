## MODIFIED Requirements

### Requirement: Fonctions de filtrage

Le système SHALL implémenter `where(criteria)`, `select(projection)`, `repeat(projection)`, `ofType(type)`, `is(type)`, `as(type)`.

L'argument de `ofType`, `is` et `as` est un identifiant de nom de type. La correspondance de type (partagée par `ofType`, les fonctions `is`/`as` et les opérateurs `is`/`as`) SHALL accepter les types primitifs FHIRPath ainsi qu'un objet (ressource ou élément complexe) dont le type FHIR porté est exactement égal au nom de type et, lorsque `Environment::is_subtype` est fourni, dont n'importe quel type ancêtre correspond (ex. `Questionnaire.is(DomainResource)` et `Questionnaire is FHIR.Resource` valent `true`, `Observation.value is Quantity` et `is Element` aussi, un `Age` est un `Quantity`, N1 §6.3) ; sans ce crochet, un objet ne correspond qu'à son propre type. Un objet non typé ne correspond à aucun nom de type. Elle SHALL accepter les noms qualifiés, comme les opérateurs : `System.X` ne correspond qu'à des valeurs System (littéraux, résultats de fonctions), jamais aux données FHIR (primitives typées, objets FHIR, résultats de `resolve()`) ; les noms System sont capitalisés, `System.Any` correspond à toute valeur, et un `System.X` inconnu (ex. `System.Patient`) vaut `false` sans erreur. `FHIR.X` (y compris `FHIR.``Patient```) ne correspond qu'à des données FHIR ; avec le crochet de types, un `FHIR.X` inconnu SHALL retourner `Err(EvalError::Type(...))` comme un nom non qualifié inconnu, sinon les règles non qualifiées (héritage via le Registry) s'appliquent. Lorsque `Environment::is_subtype` est fourni et que la valeur porte un type FHIR (`Value::Primitive` avec un code de type), `ofType`, `is` et `as` SHALL répondre par l'ancêtrie des types FHIR pour un nom de type FHIR (`code` est une `string`, `canonical` et `url` sont des `uri` ; `string` n'est pas un `code`, `uri` n'est pas un `url`). Les noms System (`String`, `Integer`, ...) et les valeurs sans type FHIR (littéraux) SHALL conserver les règles System. Avec le crochet, un nom non qualifié capitalisé qui n'est pas un type FHIR mais un type System (`Boolean`, `String`, `Integer`, ...) ou `Any` SHALL être résolu comme `System.<nom>` (N1 §10.1 : le modèle d'abord, puis System) : une donnée FHIR n'est donc jamais `is(String)` ni `is(Boolean)` (ex. `Patient.active.is(Boolean)` vaut `false`) et toute valeur est `is(Any)`. Avec le crochet, un autre nom non qualifié qui n'est ni un type System ni un type FHIR (ex. `string1`) SHALL retourner `Err(EvalError::Type(...))`. Sans ce crochet (défaut), tester une `String` contre un sous-type FHIR de `string` SHALL échouer avec `EvalError::Type`.

L'opérateur `is` (`X is Type`) SHALL avoir exactement la même sémantique que la fonction `is(Type)` (N1 §6.3.1) : collection vide en entrée -> collection vide, un seul élément -> `[Bool(true)]` ou `[Bool(false)]`, plus d'un élément -> `Err(EvalError::Type(...))`.

L'opérateur `as` (`X as Type`) SHALL avoir la même sémantique que la fonction `as(Type)` : collection vide en entrée -> collection vide, un seul élément -> l'élément s'il correspond, sinon une collection vide, plus d'un élément -> `Err(EvalError::Type(...))`.

Le critère de `where` SHALL produire pour chaque élément un booléen seul ou une collection vide, sinon `Err(EvalError::Type(...))`.

#### Scenario: where filtre les éléments

- **WHEN** `where(active = true)` est évalué sur une collection d'objets
- **THEN** le résultat SHALL contenir uniquement les éléments pour lesquels le critère est vrai

#### Scenario: $index dans where, select et repeat

- **WHEN** `('a' | 'b' | 'c').where($index > 0)` ou `('a' | 'b' | 'c').select($index)` est évalué
- **THEN** `$index` SHALL valoir la position (Integer, base 0) de l'élément dans la collection itérée, soit `['b', 'c']` puis `[0, 1, 2]` ; dans `repeat`, il SHALL valoir la position dans la ronde courante (la spécification N1 est muette)

#### Scenario: $index de l'itération la plus interne

- **WHEN** `(1 | 2).select((10 | 20 | 30).where($index = 2))` est évalué
- **THEN** `$index` SHALL désigner la position dans l'itération la plus interne, soit `[30, 30]`

#### Scenario: ofType filtre par type FHIRPath

- **WHEN** `ofType(String)` est évalué sur une collection mixte
- **THEN** le résultat SHALL contenir uniquement les valeurs de type `String`

#### Scenario: is, as et ofType sur un objet complexe typé

- **WHEN** `Observation.value is Quantity`, `Observation.value is Element` (valeur `Age`, `is Quantity`) ou `name.ofType(HumanName)` est évalué avec le crochet `is_subtype`
- **THEN** `is` SHALL valoir `true` et `ofType` SHALL conserver l'élément ; sans le crochet, l'objet ne correspond qu'à son propre type

#### Scenario: type() sur un objet complexe typé

- **WHEN** `name.type()` est évalué sur un `HumanName` typé
- **THEN** le résultat SHALL porter `namespace` = `FHIR` et `name` = `HumanName` ; un objet non typé SHALL donner `Err(EvalError::Type(...))`

#### Scenario: ofType filtre par resourceType

- **WHEN** `ofType(Patient)` est évalué sur une collection de ressources
- **THEN** le résultat SHALL contenir uniquement les objets dont `resourceType` vaut exactement `Patient`

#### Scenario: is et as sur un seul élément

- **WHEN** `is(Patient)` ou `as(Patient)` est évalué sur une collection d'un seul élément
- **THEN** `is` SHALL retourner `[Bool(true)]` ou `[Bool(false)]`, et `as` SHALL retourner l'élément s'il correspond, sinon une collection vide

#### Scenario: Héritage des types FHIR

- **WHEN** `Questionnaire.status.is(string)`, `Questionnaire.status is code` ou `Questionnaire.status.as(code)` est évalué avec le crochet `is_subtype` du Registry
- **THEN** le résultat SHALL être `[Bool(true)]`, `[Bool(true)]` puis la valeur `draft`

#### Scenario: Un supertype n'est pas son sous-type

- **WHEN** `Questionnaire.title.is(code)`, `Questionnaire.title as code` ou `Questionnaire.url.is(url)` est évalué avec le crochet
- **THEN** le résultat SHALL être `[Bool(false)]`, une collection vide, puis `[Bool(false)]`

#### Scenario: ofType par type FHIR

- **WHEN** `Questionnaire.descendants().ofType(canonical)` puis `ofType(uri)` sont évalués avec le crochet
- **THEN** le premier SHALL ne retourner que les canonical, le second les `uri` et leurs sous-types (`url`, `canonical`, l'`url` d'extension)

#### Scenario: Type FHIR de l'id de ressource et des primitives sans valeur

- **WHEN** `Questionnaire.id.is(id)` ou `Questionnaire.version.is(string)` (où `version` n'a que `_version`) est évalué avec le crochet
- **THEN** le résultat SHALL être `[Bool(true)]` (l'`id` est typé par `child_element(resourceType, "id")`), et `version.is(code)` SHALL être `[Bool(false)]`

#### Scenario: Primitives booléennes, entières et décimales typées

- **WHEN** un champ `boolean`, `integer` ou `decimal` est converti avec `child_element`
- **THEN** la valeur SHALL être une `Primitive` portant son type FHIR (`integer`), et l'arithmétique et les comparaisons SHALL la traiter comme sa valeur nue

#### Scenario: Primitive XML avec extensions

- **WHEN** `Questionnaire.status.is(code)` est évalué avec le crochet sur un `status` XML portant une extension
- **THEN** le résultat SHALL être `[Bool(true)]` (la `Primitive` porte son type FHIR)

#### Scenario: Noms System et littéraux

- **WHEN** `Questionnaire.status.is(String)`, `Questionnaire.status.ofType(integer)` ou `'draft'.is(code)` est évalué avec le crochet
- **THEN** le premier SHALL valoir `[Bool(false)]` (donnée FHIR, jamais `System.String`) et le deuxième SHALL retourner une collection vide et `'draft'.is(code)`, littéral sans type FHIR, SHALL retourner `Err(EvalError::Type(...))` (`'draft'.is(String)` vaut `[Bool(true)]`)

#### Scenario: Sans crochet is_subtype

- **WHEN** `Questionnaire.status.is(code)` ou `ofType(canonical)` est évalué sans `is_subtype`
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))` (le validateur fournit toujours le crochet)

#### Scenario: is et as sur une collection vide

- **WHEN** `is(type)` ou `as(type)` est évalué sur une collection vide
- **THEN** le résultat SHALL être une collection vide

#### Scenario: is et as sur plusieurs éléments

- **WHEN** `is(type)` ou `as(type)` est évalué sur une collection de plus d'un élément
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))` (test officiel `testFHIRPathAsFunction21`)

#### Scenario: noms de type qualifiés

- **WHEN** `1.is(System.Integer)`, `1.is(System.Patient)`, `Questionnaire.status.is(System.String)` et `Questionnaire.status.is(FHIR.code)` sont évalués avec le crochet
- **THEN** les résultats SHALL être `[Bool(true)]`, `[Bool(false)]`, `[Bool(false)]` et `[Bool(true)]`, et `Questionnaire.status.is(FHIR.string1)` SHALL retourner `Err(EvalError::Type(...))`

#### Scenario: l'opérateur is suit la sémantique de is()

- **WHEN** `X is Type` est évalué avec `X` vide, d'un seul élément, ou de plusieurs éléments
- **THEN** le résultat SHALL être respectivement une collection vide, `[Bool(true)]`/`[Bool(false)]`, ou `Err(EvalError::Type(...))`, comme pour `X.is(Type)`

#### Scenario: l'opérateur as suit la sémantique de as()

- **WHEN** `(1 | 2) as Integer` est évalué
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`, comme pour `(1 | 2).as(Integer)`

### Requirement: Navigation stricte opt-in

`Environment::strict: Option<&Strict>` (défaut `None`) SHALL activer l'erreur de navigation (N1 §3 : « If the identifier cannot be resolved, the evaluation will end and signal an error »). `pub struct Strict` est exporté et construit par `Strict::new(&has_element)`, où `pub type HasElement = dyn Fn(&str, &str) -> Option<bool>` (type, nom ; `None` = indécidable, toujours exporté) est construit par l'appelant à partir de `Registry::has_element`. Un seul `Strict` SHALL être partagé par toutes les évaluations d'un arbre de ressource (la ressource et ses ressources imbriquées) : il met en cache `has_element` par (définition, nom) et son index des chemins d'instance est construit à partir des premières ressources d'environnement demandées (`%rootResource`/`%resource`). Cet index SHALL être réutilisé par toute évaluation dont `%resource`/`%rootResource` y figurent déjà (une ressource `contained` figure dans l'index de son conteneur) ; sans `%resource` ni `%rootResource`, l'index partagé SHALL NOT être utilisé. Les évaluations avec d'autres racines, et les nœuds de contexte hors de l'index, utilisent un index construit une fois par évaluation. `fhir_validator::validate` SHALL construire un seul `Strict` par appel (voir la spécification fhir-validator). Le coût strict SHALL rester linéaire (une `List` de 32k références SHALL coûter le même temps qu'en non strict).

Lorsqu'il est fourni, une étape de chemin SHALL retourner `Err(EvalError::Type("<nom> is not an element of <type>"))` si et seulement si l'entrée est non vide et que chaque élément est un objet typé dont le type ne déclare pas l'élément et, pour un identifiant initial, n'est ni ce type ni un sous-type. Un objet typé dont le type ne peut pas trancher (`has_element(type) == None` : `BackboneElement`, `Element`) SHALL être interrogé par son chemin d'instance depuis sa ressource (ex. `Patient.contact`, `Questionnaire.item.item`) ; le message est alors `<nom> is not an element of <chemin d'instance>`. Une ressource (racine, `contained`, entrée de Bundle) SHALL redémarrer le chemin à son `resourceType`. Il n'y a pas de recherche de suffixe de choix : `Observation.component.valueString` SHALL donner une erreur en mode strict. Les backbones hors des arbres indexés, les primitives et les nœuds non typés SHALL NOT provoquer d'erreur. Une ressource imbriquée dans une autre (`contained`, entrée de Bundle, ...) qui n'est ni l'entrée ni `%resource`/`%rootResource` SHALL NOT provoquer d'erreur (son élément est typé `Resource`) ; `ofType()` ne la restreint pas à nouveau (limitation).

Dans un paramètre d'expression (`where`, `select`, `exists`, `all`, `repeat`, `aggregate`, `sort`, ...), un identifiant initial SHALL être résolu contre toute la collection d'entrée dont provient `$this` (erreur seulement si aucun élément ne le résout) ; cette règle de collection entière ne s'applique qu'à un nom initial évalué sur `$this` lui-même : un nom évalué sur une autre entrée (ex. arguments de `iif()` sur `code`) SHALL être vérifié contre cette seule entrée, et la question « un élément de l'entrée résout-il le nom » SHALL être tranchée une seule fois par itération ; dans `repeat()`, cette collection est l'entrée plus toutes les projections déjà produites. Les étapes suivantes restent vérifiées sur les valeurs de l'élément (limitation documentée). Sans `strict`, un nom inconnu SHALL donner une collection vide. `fhir_validator::validate` SHALL toujours être strict (il n'existe pas de `validate_strict`) ; le test `core_invariants_resolve_every_name_on_a_corpus_sample` SHALL constater, sur une fixture JSON sur 40 de `tests/fixtures`, qu'aucun diagnostic ne contient « is not an element of », et `crates/fhir-validator/tests/strict_navigation_tests.rs` SHALL couvrir les fautes de frappe.

#### Scenario: Élément inconnu en mode strict

- **WHEN** `Patient.bogus` est évalué avec `strict` fourni sur un `Patient`
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`

#### Scenario: Enfant de backbone inconnu

- **WHEN** `Patient.contact.bogus` ou `Observation.component.valueString` est évalué avec `strict` sur la ressource correspondante
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type("<nom> is not an element of <chemin d'instance>"))`

#### Scenario: Backbone imbriqué

- **WHEN** `Questionnaire.item.item.linkId` est évalué avec `strict` sur un Questionnaire
- **THEN** aucune erreur SHALL être retournée

#### Scenario: Collection mixte

- **WHEN** `descendants().bogus` est évalué avec `strict` sur une collection mêlant objets typés et éléments backbone, primitives ou nœuds hors des arbres indexés
- **THEN** aucune erreur SHALL être retournée

#### Scenario: Collection mixte avec unité

- **WHEN** `descendants().where(unit = 'lbs')` est évalué avec `strict` sur une Observation
- **THEN** le résultat SHALL être la Quantity, sans erreur

#### Scenario: Identifiant inconnu dans un paramètre

- **WHEN** `Observation.code.where(bogus.exists())` est évalué avec `strict`
- **THEN** l'évaluateur SHALL retourner l'erreur « bogus is not an element of CodeableConcept »

#### Scenario: Nom évalué sur une autre entrée

- **WHEN** `descendants().where(code.iif(unit.exists(), true, false))` est évalué avec `strict` sur une Observation ayant un component
- **THEN** l'évaluateur SHALL retourner l'erreur « unit is not an element of CodeableConcept »

#### Scenario: repeat sur backbones

- **WHEN** `QuestionnaireResponse.item.repeat(answer | item)` est évalué avec `strict`
- **THEN** aucune erreur SHALL être retournée

#### Scenario: Ressource contenue imbriquée

- **WHEN** `contained.contained.empty()` est évalué avec `strict` sur une ressource contenant un Bundle contenu
- **THEN** le résultat SHALL être `true`

#### Scenario: dom-3 sous strict

- **WHEN** l'invariant dom-3 du noyau est évalué avec `strict` sur un Patient dont un `Basic` contenu n'est pas référencé
- **THEN** le validateur SHALL produire une Error `INVARIANT_dom-3`

#### Scenario: Défaut non strict

- **WHEN** `Patient.bogus` est évalué sans `strict`
- **THEN** le résultat SHALL être vide
