## MODIFIED Requirements

### Requirement: Validation de cardinalité — élément requis absent (CARDINALITY_001)

Le système SHALL émettre un `Diagnostic` avec code `CARDINALITY_001` et severity `Error` pour chaque élément déclaré avec `min > 0` dans le snapshot et absent de la ressource. Une valeur JSON qui est un tableau vide (`[]`) ou un tableau ne contenant que des `null` SHALL compter comme absente, à toute profondeur (parcours unique, réutilisé par la couche de cardinalité de profil) : `"compose": {"include": []}` ou `"include": [null]` dans un ValueSet, ou `"author": []` dans une Composition, produit `CARDINALITY_001`. Un élément requis de type primitif (nom de type commençant par une minuscule) dont la valeur est absente (ou un tableau vide ou de seuls `null`) SHALL être considéré présent via `_nom` (extension de primitif) seulement si la valeur de `_nom` est un objet, ou un tableau d'objets et de `null` contenant au moins un objet ; de même qu'une valeur `[]` ou `[null]`, un `_nom` scalaire tel que `"_status": 5`, `[]` ou `[null]` ne compte pas et produit `CARDINALITY_001` ; ainsi `"status": [null], "_status": [{"id": "a"}]` reste présent. Pour un type complexe, `_nom` ne SHALL PAS compter comme présence. Lorsque la valeur est absente et que `_nom` est un tableau plus long que le maximum fini, un `CARDINALITY_002` SHALL être émis (path et position de `_nom`). Un nom d'élément contenant `.`, `[` ou `]` (y compris préfixé par `_`) SHALL produire `STRUCTURE_001`. Un choix `[x]` SHALL être vérifié selon le seul type désigné par son suffixe ; sa cardinalité porte sur l'ensemble de ses variantes (champs `base` + type autorisé capitalisé, `_variante` comptant seulement pour un type primitif et sous la même forme objet que `_nom`), à toute profondeur : aucune variante et `min > 0` produit `CARDINALITY_001` au chemin du choix (ex. `Immunization.occurrence[x]`), plus d'une variante produit `CARDINALITY_002` à la seconde variante. La règle du tableau vide ne s'applique pas aux variantes `[x]` (de `max="1"`, un tableau y produit déjà `STRUCTURE_002`). Un `integer64` SHALL être une chaîne conforme à `[0]|[-+]?[1-9][0-9]*` et représentable sur 64 bits signés (`TYPE_003` sinon, `TYPE_001` si non chaîne).

#### Scenario: Élément requis absent

- **WHEN** une ressource Observation est fournie sans le champ `status` (min=1)
- **THEN** un Diagnostic `CARDINALITY_001` SHALL être émis mentionnant `status`

#### Scenario: Élément optionnel absent ne produit pas d'erreur

- **WHEN** une ressource est fournie sans un champ optionnel (min=0)
- **THEN** aucun Diagnostic `CARDINALITY_001` ne SHALL être émis pour ce champ

#### Scenario: Primitif requis présent uniquement via _champ

- **WHEN** un primitif requis est fourni uniquement sous la forme `_nom` (sans `nom`)
- **THEN** aucun Diagnostic `CARDINALITY_001` ne SHALL être émis pour ce champ

#### Scenario: Élément complexe requis présent uniquement via _champ

- **WHEN** `Observation.code` est fourni uniquement sous la forme `_code`
- **THEN** un Diagnostic `CARDINALITY_001` SHALL être émis pour `Observation.code`

#### Scenario: Extension de primitif en tableau au-delà du maximum

- **WHEN** `Patient` contient `_birthDate: [{}, {}]` sans `birthDate`
- **THEN** un Diagnostic `CARDINALITY_002` SHALL être émis avec `path` = `Patient.birthDate`

#### Scenario: Clé préfixée par _ avec ponctuation

- **WHEN** une ressource ou un `Observation.component[0]` contient la clé `_a.b` ou `_value[x]`
- **THEN** un Diagnostic `STRUCTURE_001` SHALL être émis, alors que `_status` reste accepté

#### Scenario: Choix requis absent

- **WHEN** une Immunization ne contient aucune variante de `occurrence[x]` (min=1), ou un `Claim.diagnosis[0]` aucune variante de `diagnosis[x]`
- **THEN** un Diagnostic `CARDINALITY_001` SHALL être émis avec `path` = `Immunization.occurrence[x]` (resp. `Claim.diagnosis[0].diagnosis[x]`)

#### Scenario: Choix présent uniquement via _variante

- **WHEN** une Immunization contient seulement `_occurrenceString` (type primitif)
- **THEN** aucun Diagnostic `CARDINALITY_001` ne SHALL être émis pour `occurrence[x]`

#### Scenario: Plusieurs variantes d'un même choix

- **WHEN** une Observation contient `valueString` et `valueBoolean`
- **THEN** un Diagnostic `CARDINALITY_002` SHALL être émis avec `path` = `Observation.valueBoolean` (seconde variante)

#### Scenario: Extension de primitif mal formée

- **WHEN** une Observation contient `"_status": 5`, `"_status": []` ou `"_status": [null]` sans `status`, ou une Immunization seulement `"_occurrenceString": 5`
- **THEN** un Diagnostic `CARDINALITY_001` SHALL être émis pour `Observation.status` (resp. `Immunization.occurrence[x]`)

#### Scenario: Élément requis en tableau vide ou de null

- **WHEN** un ValueSet contient `"compose": {"include": []}` ou `"compose": {"include": [null]}`, ou une Composition `"author": []`
- **THEN** un Diagnostic `CARDINALITY_001` SHALL être émis pour `ValueSet.compose.include` (resp. `Composition.author`)

#### Scenario: Valeur de null compensée par _nom

- **WHEN** une Observation contient `"status": [null], "_status": [{"id": "a"}]`
- **THEN** aucun Diagnostic `CARDINALITY_001` ni `CARDINALITY_003` ne SHALL être émis pour `Observation.status`

## ADDED Requirements

### Requirement: Validation de cardinalité — tableau vide d'un élément optionnel (CARDINALITY_003)

Pour un élément déclaré avec `min = 0` dont la valeur JSON est un tableau vide (`[]`) ou un tableau ne contenant que des `null`, le système SHALL émettre un `Diagnostic` avec code `CARDINALITY_003` et severity `Warning`, à toute profondeur, avec `path` = chemin d'instance de l'élément, `location` = span du tableau, et message `Element '<path>' is an array with no values and should be omitted` (R5 json.html : les objets ne sont jamais vides, un tableau de seuls `null` SHOULD être omis). Lorsque le `_nom` d'un élément primitif est un objet ou un tableau contenant au moins un objet, l'élément SHALL être considéré présent et aucun `CARDINALITY_003` ne SHALL être émis. Les variantes `[x]` ne SHALL PAS être soumises à cette règle. Coexistence voulue : un élément de `max="1"` fourni sous la forme `[]` reçoit aussi `STRUCTURE_002` en plus de `CARDINALITY_001` ou `CARDINALITY_003` ; lorsqu'un profil rend requis (`min >= 1`) un élément optionnel de la base, un `[]` reçoit le `CARDINALITY_003` de la base et le `CARDINALITY_001` du profil (la déduplication code + `path` ne fusionne pas des codes différents).

#### Scenario: Élément optionnel en tableau vide

- **WHEN** un Patient JSON contient `"identifier": []`
- **THEN** un Diagnostic `CARDINALITY_003` de severity `Warning` SHALL être émis avec `path` = `Patient.identifier`, `location` = span du tableau, et aucun `CARDINALITY_001`

#### Scenario: Élément optionnel imbriqué en tableau de null

- **WHEN** un Patient JSON contient `"name": [{"given": [null]}]`
- **THEN** un Diagnostic `CARDINALITY_003` SHALL être émis avec `path` = `Patient.name[0].given`

#### Scenario: Tableau de null compensé par _nom

- **WHEN** un Patient JSON contient `"name": [{"given": [null], "_given": [{"id": "a"}]}]`
- **THEN** aucun Diagnostic `CARDINALITY_003` ne SHALL être émis pour `Patient.name[0].given`

#### Scenario: Tableau vide sur un élément max="1"

- **WHEN** un Patient JSON contient `"gender": []`
- **THEN** un Diagnostic `STRUCTURE_002` et un Diagnostic `CARDINALITY_003` SHALL être émis avec `path` = `Patient.gender`

#### Scenario: Profil rendant requis un élément optionnel

- **WHEN** un profil connu fixe `min = 1` sur un élément optionnel de la base, et la ressource fournit `[]` pour cet élément
- **THEN** un `CARDINALITY_003` (base) et un `CARDINALITY_001` (profil) SHALL être émis pour le même `path`
