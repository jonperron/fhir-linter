## ADDED Requirements

### Requirement: Validation structurelle — forme tableau JSON (STRUCTURE_002)

Pour une ressource issue du JSON (`Resource.from_xml` faux), le système SHALL émettre un `Diagnostic` avec code `STRUCTURE_002` et severity `Error` pour chaque champ résolu, à toute profondeur (premier niveau, `BackboneElement`, types complexes, `Extension`, variantes `[x]` comme `valueString`), dont la forme contredit l'élément de la définition de base : un élément de `max` différent de `"1"` SHALL être un tableau JSON, un élément de `max="1"` ne SHALL PAS être un tableau. Le `path` SHALL être le chemin d'instance (ex. `Patient.name`, `Patient.name[0].given`, `Patient.extension[0].valueString`). Une valeur `null` ne SHALL PAS être signalée. Les champs `_nom` (extensions de primitif) ne SHALL PAS être soumis à ce contrôle ; leur contenu, parcouru comme un `Element`, l'est. La forme SHALL être déterminée uniquement par la définition de base : les couches de cardinalité de profil n'appliquent pas ce contrôle. Une ressource issue de l'XML ne SHALL jamais être soumise à ce contrôle, le remodelage XML conservant le regroupement XML (un élément XML unique reste scalaire). `STRUCTURE_002` et `CARDINALITY_002` PEUVENT être émis pour le même champ (ex. `"gender": ["male", "female"]`).

#### Scenario: Élément répétable sous forme d'objet

- **WHEN** un Patient JSON contient `"name": {"family": "Doe"}`
- **THEN** un Diagnostic `STRUCTURE_002` SHALL être émis avec `path` = `Patient.name`

#### Scenario: Élément max="1" sous forme de tableau

- **WHEN** un Patient JSON contient `"gender": ["male"]`
- **THEN** un Diagnostic `STRUCTURE_002` SHALL être émis avec `path` = `Patient.gender`

#### Scenario: Forme incorrecte en profondeur et dans une extension

- **WHEN** un Patient JSON contient `"name": [{"given": "A"}]` et `"extension": [{"url": "http://example.org/x", "valueString": ["a"]}]`
- **THEN** un Diagnostic `STRUCTURE_002` SHALL être émis avec `path` = `Patient.name[0].given`, et un autre avec `path` = `Patient.extension[0].valueString`

#### Scenario: Élément XML unique non signalé

- **WHEN** un Patient XML contient un unique élément `<name>`
- **THEN** aucun Diagnostic `STRUCTURE_002` ne SHALL être émis

## MODIFIED Requirements

### Requirement: Validation des éléments imbriqués

Un seul parcours récursif SHALL appliquer ces vérifications, en partant du premier niveau de la ressource ; il s'exécute avant les couches terminologie, invariants, références et profil. À toute profondeur, un champ `_nom` SHALL être accepté seulement si `nom` résout un élément (ou une variante de choix) de type primitif, sinon `STRUCTURE_001` au chemin de `_nom` (ex. `Patient.name[0]._bogus`). Un `_nom` accepté SHALL être parcouru via la StructureDefinition du type complexe `Element` (contrôle de `id` et `extension`) ; un tableau SHALL être parcouru élément par élément (index dans le `path`, éléments `null` ignorés) ; une valeur non objet, ou un élément de tableau ni objet ni `null`, SHALL produire `TYPE_001` (ex. `"_birthDate": "x"` → `Patient._birthDate`, `"_given": [1]` → `Patient.name[0]._given[0]`). Le champ `_nom` lui-même n'est pas soumis à STRUCTURE_002 ; son contenu l'est pour une entrée JSON. Les champs `extension` et `modifierExtension` SHALL être traités comme tout autre élément de type `Extension`, à toute profondeur : résolus dans le snapshot (STRUCTURE_001 s'ils n'y figurent pas, ex. `Patient.name[0].modifierExtension`), puis parcourus via la StructureDefinition `Extension` (cardinalité de `url`, type de `value[x]`, extensions imbriquées). Un élément `[x]` SHALL être vérifié comme un tout, au premier niveau comme en profondeur : requis sans variante → `CARDINALITY_001` (au chemin `…[x]`), deux variantes → `CARDINALITY_002`, une variante présente soumise à `CARDINALITY_002` selon son `max`. Le système SHALL descendre dans les `BackboneElement` (via le même snapshot) et dans les types complexes (via leur StructureDefinition `complex-type`) et y appliquer les vérifications structurelles (STRUCTURE_001, et STRUCTURE_002 pour les entrées JSON), de cardinalité (CARDINALITY_001/002) et de type (TYPE_*), avec un `path` d'instance incluant les index (ex. `Patient.name[0].bogus`). Les éléments `[x]` SHALL être résolus par leur suffixe. Les couches terminologie, invariants, références et profil restent limitées au premier niveau ; les `contentReference` (ignorés sans erreur, jamais signalés STRUCTURE_001) et les ressources contenues ne sont pas parcourus. La règle `_nom` s'applique à toute profondeur : un enfant requis présent uniquement sous la forme `_nom` SHALL être considéré présent seulement si son type est primitif et si `_nom` est un objet ou un tableau d'objets et de `null` contenant au moins un objet, et un `_nom` en tableau au-delà du maximum fini SHALL produire `CARDINALITY_002`.

#### Scenario: Choix [x] en tableau au-delà du maximum

- **WHEN** une Observation contient `"valueString": ["a", "b"]` (`value[x]` a `max="1"`)
- **THEN** un Diagnostic CARDINALITY_002 SHALL être émis avec `path` = `Observation.valueString`

#### Scenario: Élément inconnu dans un type complexe

- **WHEN** `Patient.name[0]` contient un champ `bogus`
- **THEN** un Diagnostic STRUCTURE_001 SHALL être émis avec `path` = `Patient.name[0].bogus`

#### Scenario: Forme tableau incorrecte dans un type complexe

- **WHEN** `Patient.name[0]` (JSON) contient `"given": "A"`
- **THEN** un Diagnostic STRUCTURE_002 SHALL être émis avec `path` = `Patient.name[0].given`

#### Scenario: Élément requis manquant dans un BackboneElement

- **WHEN** `Observation.component[0]` n'a pas de `code`
- **THEN** un Diagnostic CARDINALITY_001 SHALL être émis avec `path` = `Observation.component[0].code`

#### Scenario: Extension sans url

- **WHEN** un Patient contient `"extension": [{"valueString": "x"}]`
- **THEN** un Diagnostic CARDINALITY_001 SHALL être émis avec `path` = `Patient.extension[0].url`

#### Scenario: Valeur d'extension mal typée ou inconnue

- **WHEN** `Patient.extension[0]` contient `"valueBoolean": "yes"` ou `"valueBogus": 1`
- **THEN** un Diagnostic `TYPE_001` SHALL être émis pour `Patient.extension[0].valueBoolean`, respectivement `STRUCTURE_001` pour `Patient.extension[0].valueBogus`

#### Scenario: Extension imbriquée parcourue

- **WHEN** `Patient.extension[0]` contient une extension imbriquée sans `url`
- **THEN** un Diagnostic CARDINALITY_001 SHALL être émis avec `path` = `Patient.extension[0].extension[0].url`

#### Scenario: modifierExtension sur un datatype

- **WHEN** `Patient.name[0]` contient un champ `modifierExtension`
- **THEN** un Diagnostic STRUCTURE_001 SHALL être émis avec `path` = `Patient.name[0].modifierExtension`
