## MODIFIED Requirements

### Requirement: Validation de type — caractère de contrôle dans une chaîne (TYPE_006)

Le système SHALL émettre un `Diagnostic` avec code `TYPE_006` et severity `Warning` lorsqu'une valeur de type `string`, `markdown`, `code` ou `id` contient un caractère inférieur à U+0020 autre que tabulation (U+0009), LF (U+000A) et CR (U+000D) (FHIR R5 datatypes.html : « SHOULD not contain »). Le `path` SHALL être le chemin de l'instance, la localisation SHALL être le span de la valeur, et le message SHALL être `String at '<path>' contains a control character below U+0020`. La règle SHALL s'appliquer aux ressources JSON et aux ressources XML 1.1 dont les valeurs proviennent de références de caractère (le parseur XML rejetant les autres cas). L'`id` de la ressource (extrait des champs par le parseur) SHALL également être vérifié : il SHALL produire un `TYPE_006` au chemin `<Type>.id`, sans localisation source (même convention que `TYPE_005` sur l'`id` de ressource), en plus du `TYPE_005`.

#### Scenario: Caractère de contrôle en JSON

- **WHEN** un Patient JSON contient `"name": [{"family": "a\u0001b"}]`
- **THEN** un Diagnostic `TYPE_006` de severity `Warning` SHALL être émis au chemin `Patient.name[0].family` avec le message `String at 'Patient.name[0].family' contains a control character below U+0020` et la localisation de la valeur

#### Scenario: Caractère de contrôle dans l'id de ressource

- **WHEN** un Patient JSON contient `"id": "a\u0001"`
- **THEN** un Diagnostic `TYPE_005` de severity `Error` et un Diagnostic `TYPE_006` de severity `Warning` SHALL être émis au chemin `Patient.id`, sans localisation source

#### Scenario: Tabulation, CR et LF autorisés

- **WHEN** une valeur `string` contient uniquement `\t`, `\r` ou `\n` comme caractères inférieurs à U+0020
- **THEN** aucun Diagnostic `TYPE_006` ne SHALL être émis

#### Scenario: Référence de caractère en XML 1.1

- **WHEN** un document `<?xml version="1.1"?>` contient une valeur `string` avec `&#1;`
- **THEN** le parse SHALL réussir et un Diagnostic `TYPE_006` SHALL être émis

#### Scenario: Fixtures officielles

- **WHEN** les fixtures `unicode-problem.json` et `xml_v11.xml` sont validées
- **THEN** un Diagnostic `TYPE_006` de severity `Warning` SHALL être émis
