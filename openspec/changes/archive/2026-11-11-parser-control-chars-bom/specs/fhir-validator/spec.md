## MODIFIED Requirements

### Requirement: Validation de type — erreur de type de base (TYPE_001)

Le système SHALL émettre un `Diagnostic` avec code `TYPE_001` et severity `Error` lorsqu'une valeur n'est pas du type JSON attendu par la StructureDefinition (ex. : nombre fourni pour un champ `string`, ou chaîne pour un champ `boolean`). Pour un champ polymorphique de premier niveau (`value[x]`), le type vérifié SHALL être celui sélectionné par le suffixe du champ. Une valeur scalaire (chaîne, nombre, booléen ou `null`) fournie pour un élément dont le type est un datatype complexe, `BackboneElement` ou `Element` SHALL produire `TYPE_001` avec le message `Expected <type> object at '<path>'` ; un tableau SHALL être vérifié élément par élément.

#### Scenario: Champ polymorphique typé par son suffixe

- **WHEN** une Observation contient `valueBoolean` avec une valeur JSON string
- **THEN** un Diagnostic `TYPE_001` SHALL être émis pour `valueBoolean`

#### Scenario: Boolean attendu, string reçu

- **WHEN** un champ de type `boolean` contient une valeur JSON string
- **THEN** un Diagnostic `TYPE_001` SHALL être émis

#### Scenario: String attendue, nombre reçu

- **WHEN** un champ de type `string` contient une valeur JSON numérique
- **THEN** un Diagnostic `TYPE_001` SHALL être émis

#### Scenario: Entier attendu, décimal reçu

- **WHEN** un champ de type `integer`, `unsignedInt` ou `positiveInt` contient une valeur parsée en `Value::Decimal` (par exemple `1.5` ou un entier dépassant i64)
- **THEN** un Diagnostic `TYPE_001` SHALL être émis

#### Scenario: Scalaire à la place d'un objet complexe

- **WHEN** un Patient contient `"extension": "oops"` ou `"meta": null`
- **THEN** un Diagnostic `TYPE_001` SHALL être émis avec le message `Expected Extension object at 'Patient.extension'` (respectivement `Expected Meta object at 'Patient.meta'`)

#### Scenario: Scalaire dans un tableau d'objets complexes

- **WHEN** un Patient contient `"name": [{"family": "Doe"}, 42]`
- **THEN** un Diagnostic `TYPE_001` SHALL être émis avec `path` = `Patient.name[1]` et aucun pour `Patient.name[0]`

#### Scenario: Entier attendu, moins zéro reçu

- **WHEN** un champ de type `integer` contient le littéral JSON `-0` (parsé en `Value::Decimal(-0.0)`)
- **THEN** un Diagnostic `TYPE_001` SHALL être émis

## ADDED Requirements

### Requirement: Validation de type — caractère de contrôle dans une chaîne (TYPE_006)

Le système SHALL émettre un `Diagnostic` avec code `TYPE_006` et severity `Warning` lorsqu'une valeur de type `string`, `markdown`, `code` ou `id` contient un caractère inférieur à U+0020 autre que tabulation (U+0009), LF (U+000A) et CR (U+000D) (FHIR R5 datatypes.html : « SHOULD not contain »). Le `path` SHALL être le chemin de l'instance, la localisation SHALL être le span de la valeur, et le message SHALL être `String at '<path>' contains a control character below U+0020`. La règle SHALL s'appliquer aux ressources JSON et aux ressources XML 1.1 dont les valeurs proviennent de références de caractère (le parseur XML rejetant les autres cas).

#### Scenario: Caractère de contrôle en JSON

- **WHEN** un Patient JSON contient `"name": [{"family": "a\u0001b"}]`
- **THEN** un Diagnostic `TYPE_006` de severity `Warning` SHALL être émis au chemin `Patient.name[0].family` avec le message `String at 'Patient.name[0].family' contains a control character below U+0020` et la localisation de la valeur

#### Scenario: Tabulation, CR et LF autorisés

- **WHEN** une valeur `string` contient uniquement `\t`, `\r` ou `\n` comme caractères inférieurs à U+0020
- **THEN** aucun Diagnostic `TYPE_006` ne SHALL être émis

#### Scenario: Référence de caractère en XML 1.1

- **WHEN** un document `<?xml version="1.1"?>` contient une valeur `string` avec `&#1;`
- **THEN** le parse SHALL réussir et un Diagnostic `TYPE_006` SHALL être émis

#### Scenario: Fixtures officielles

- **WHEN** les fixtures `unicode-problem.json` et `xml_v11.xml` sont validées
- **THEN** un Diagnostic `TYPE_006` de severity `Warning` SHALL être émis
