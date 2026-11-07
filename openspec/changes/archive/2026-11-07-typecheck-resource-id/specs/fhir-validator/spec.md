## MODIFIED Requirements

### Requirement: Validation de type — format identifiant (TYPE_005)

Le système SHALL émettre un `Diagnostic` avec code `TYPE_005` et severity `Error` lorsqu'un champ de type `id` ne respecte pas le format FHIR (regex : `[A-Za-z0-9\-\.]{1,64}`).

Le système SHALL également vérifier l'`id` de la ressource selon ce format ; un `id` de ressource invalide SHALL produire un `TYPE_005` au chemin `<Type>.id`, sans localisation source.

Les enfants `id` des éléments imbriqués SHALL être vérifiés comme `string` (type de base de `Element.id`), quel que soit le type déclaré dans le snapshot : une valeur non chaîne SHALL produire un `TYPE_001`, et aucun `TYPE_005` ne SHALL être émis pour leur format.

#### Scenario: Identifiant invalide

- **WHEN** un champ de type `id` contient `"invalid id!"`
- **THEN** un Diagnostic `TYPE_005` SHALL être émis

#### Scenario: Identifiant valide

- **WHEN** un champ de type `id` contient `"patient-123"`
- **THEN** aucun Diagnostic `TYPE_005` ne SHALL être émis

#### Scenario: Id de ressource invalide

- **WHEN** une ressource `{"resourceType":"Patient","id":"has space!"}` est validée
- **THEN** un Diagnostic `TYPE_005` SHALL être émis au chemin `Patient.id`, sans localisation source

#### Scenario: Id de ressource valide

- **WHEN** une ressource `{"resourceType":"Patient","id":"p-1.a"}` est validée
- **THEN** aucun Diagnostic `TYPE_*` ne SHALL être émis

#### Scenario: Id d'élément non chaîne

- **WHEN** un élément imbriqué contient `"id": 5` (par ex. `Patient.name[0].id`)
- **THEN** un Diagnostic `TYPE_001` SHALL être émis au chemin `Patient.name[0].id`

#### Scenario: Id d'élément au format string

- **WHEN** un élément imbriqué contient `"id": "n:1"`
- **THEN** aucun Diagnostic `TYPE_*` ne SHALL être émis
