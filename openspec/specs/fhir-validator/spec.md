# fhir-validator Specification

## Purpose
TBD - created by archiving change fhir-validator-phase5. Update Purpose after archive.
## Requirements
### Requirement: API publique de validation

Le système SHALL exposer une fonction `validate(resource: &Resource, registry: &Registry) -> Vec<Diagnostic>` qui applique toutes les couches de validation dans l'ordre et retourne l'ensemble des findings.

#### Scenario: Ressource valide ne retourne aucun finding

- **WHEN** `validate` est appelé avec une ressource Patient valide et un registry R5 chargé
- **THEN** le `Vec<Diagnostic>` retourné SHALL ne contenir aucun Diagnostic autre que des `INVARIANT_SKIPPED` de severity `Information` (contraintes non vérifiées, qui ne sont pas des findings)

#### Scenario: Ressource invalide retourne des diagnostics

- **WHEN** `validate` est appelé avec une ressource contenant des erreurs (éléments inconnus, cardinalité manquante, etc.)
- **THEN** la fonction SHALL retourner au moins un `Diagnostic` avec un code approprié

---

### Requirement: Type Diagnostic

Le système SHALL fournir un type `Diagnostic` public avec les champs : `severity: Severity`, `code: Arc<str>`, `message: String`, `path: String`, `location: Option<Span>`, `rule: Option<Arc<str>>`.

#### Scenario: Construction d'un diagnostic d'erreur

- **WHEN** `Diagnostic::error(code, message, path)` est appelé
- **THEN** le Diagnostic SHALL avoir `severity = Severity::Error`, le code, le message et le path fournis, et `location = None`, `rule = None`

#### Scenario: Construction d'un diagnostic de warning

- **WHEN** `Diagnostic::warning(code, message, path)` est appelé
- **THEN** le Diagnostic SHALL avoir `severity = Severity::Warning`

#### Scenario: Attachement d'une localisation source

- **WHEN** `.with_location(span)` est chaîné sur un Diagnostic
- **THEN** `diagnostic.location` SHALL contenir le `Span` fourni

#### Scenario: Attachement d'une règle lint

- **WHEN** `.with_rule(rule_id)` est chaîné sur un Diagnostic
- **THEN** `diagnostic.rule` SHALL contenir l'identifiant fourni

---

### Requirement: Type Severity

Le système SHALL fournir une enum `Severity` publique dont les variants sont les codes FHIR R5 IssueSeverity utilisables par un diagnostic : `Fatal`, `Error`, `Warning`, `Information`, ordonnés du plus sévère au moins sévère. Le code `success` n'est pas un niveau de diagnostic et SHALL NOT avoir de variant ; aucun variant `Hint` ne SHALL exister. `Severity::ALL` SHALL contenir tous les variants, du plus sévère au moins sévère.

#### Scenario: Ordre des sévérités

- **WHEN** deux valeurs `Severity` sont comparées avec `<`
- **THEN** `Fatal < Error < Warning < Information` SHALL être vrai

#### Scenario: Liste des variants

- **WHEN** `Severity::ALL` est lu
- **THEN** il SHALL valoir `[Fatal, Error, Warning, Information]`

### Requirement: Validation structurelle — type de ressource inconnu (STRUCTURE_000)

Le système SHALL émettre exactement un `Diagnostic` avec code `STRUCTURE_000` et severity `Error` lorsque `resourceType` ne désigne pas une ressource concrète, c'est-à-dire lorsqu'aucune StructureDefinition du registry n'a ce type avec `kind = resource` et `abstract = false`, et SHALL arrêter toute validation supplémentaire. Les ressources abstraites (`Resource`, `DomainResource`) et les types de données (ex. `HumanName`) SHALL donc être rejetés.

#### Scenario: Type de ressource inconnu

- **WHEN** `validate` est appelé avec une ressource dont `resourceType` n'existe pas dans le registry
- **THEN** la fonction SHALL retourner exactement un Diagnostic avec code `STRUCTURE_000` et severity `Error`

#### Scenario: Type abstrait ou type de données

- **WHEN** `validate` est appelé avec une ressource dont `resourceType` est `Resource`, `DomainResource` ou `HumanName`
- **THEN** la fonction SHALL retourner exactement un Diagnostic avec code `STRUCTURE_000` et severity `Error`

---

### Requirement: Validation structurelle — élément inconnu (STRUCTURE_001)

Le système SHALL émettre un `Diagnostic` avec code `STRUCTURE_001` et severity `Error` pour chaque champ de la ressource qui ne correspond à aucun chemin de profondeur 1 dans le snapshot de la StructureDefinition. Le suffixe d'un champ polymorphique (`value[x]`) SHALL être valide uniquement s'il nomme l'un des types déclarés de l'élément (aucune liste de suffixes codée en dur). Un nom de champ contenant `.`, `[` ou `]` SHALL toujours être signalé, sans résolution, au premier niveau comme dans les `BackboneElement`. Les champs `extension` et `modifierExtension` SHALL être résolus dans le snapshot comme tout autre champ : absents du snapshot (ex. `Bundle.extension`, Bundle dérivant de `Resource`), ils SHALL produire `STRUCTURE_001`. Un champ `_nom` (extension de primitif) SHALL être accepté seulement si `nom` résout un élément (y compris une variante de choix, ex. `_valueString`) de type primitif (nom de type commençant par une minuscule) ; sinon `STRUCTURE_001` SHALL être émis au chemin de `_nom` (ex. `Patient._bogus`, ou `Patient._name` car `HumanName` est complexe).

#### Scenario: Élément inconnu détecté

- **WHEN** une ressource contient un champ `unknownField` absent du snapshot
- **THEN** un Diagnostic `STRUCTURE_001` SHALL être émis dont `path` contient le nom du champ

#### Scenario: Champ connu ne produit pas d'erreur

- **WHEN** une ressource contient uniquement des champs définis dans le snapshot
- **THEN** aucun Diagnostic `STRUCTURE_001` ne SHALL être émis

#### Scenario: Extensions déclarées et extensions primitives autorisées

- **WHEN** un Patient contient des champs `extension`, `modifierExtension` (déclarés dans son snapshot) ou `_birthDate` (extension d'un primitif déclaré)
- **THEN** aucun Diagnostic `STRUCTURE_001` ne SHALL être émis pour ces champs

#### Scenario: Extension sur une ressource qui n'en déclare pas

- **WHEN** un Bundle contient un champ `extension`
- **THEN** un Diagnostic `STRUCTURE_001` SHALL être émis avec `path` = `Bundle.extension`

#### Scenario: Champ polymorphique valide (value[x])

- **WHEN** une ressource contient `valueBoolean` et la StructureDefinition déclare `value[x]`
- **THEN** aucun Diagnostic `STRUCTURE_001` ne SHALL être émis

#### Scenario: Suffixe polymorphique hors des types déclarés

- **WHEN** une Observation contient `valueAddress` et `Observation.value[x]` ne déclare pas le type `Address`
- **THEN** un Diagnostic `STRUCTURE_001` SHALL être émis pour `valueAddress`

#### Scenario: Nom d'élément contenant '.', '[' ou ']'

- **WHEN** une ressource (ou un BackboneElement) contient un champ `component.code` ou le littéral `value[x]`
- **THEN** un Diagnostic `STRUCTURE_001` SHALL être émis pour ce champ

#### Scenario: Extension de primitif sans élément primitif correspondant

- **WHEN** un Patient contient `_bogus` (aucun élément `bogus`) ou `_name` (`name` est de type complexe `HumanName`)
- **THEN** un Diagnostic `STRUCTURE_001` SHALL être émis avec `path` = `Patient._bogus`, respectivement `Patient._name`

### Requirement: Validation structurelle — forme tableau JSON (STRUCTURE_002)

Pour une ressource issue du JSON (`Resource.from_xml` faux), le système SHALL émettre un `Diagnostic` avec code `STRUCTURE_002` et severity `Error` pour chaque champ résolu, à toute profondeur (premier niveau, `BackboneElement`, types complexes, `Extension`, variantes `[x]` comme `valueString`), dont la forme contredit l'élément de la définition de base : un élément de `max` différent de `"1"` SHALL être un tableau JSON, un élément de `max="1"` ne SHALL PAS être un tableau. Le `path` SHALL être le chemin d'instance (ex. `Patient.name`, `Patient.name[0].given`, `Patient.extension[0].valueString`). Une valeur `null` ne SHALL PAS être signalée. Les champs `_nom` (extensions de primitif) ne SHALL PAS être soumis à ce contrôle ; leur contenu, parcouru comme un `Element`, l'est. La forme SHALL être déterminée uniquement par la définition de base : les couches de cardinalité de profil n'appliquent pas ce contrôle. Une ressource issue de l'XML ne SHALL jamais être soumise à ce contrôle, le remodelage XML conservant le regroupement XML (un élément XML unique reste scalaire). `STRUCTURE_002` et `CARDINALITY_002` PEUVENT être émis pour le même champ (ex. `"gender": ["male", "female"]`). Pour le JSON uniquement, chaque élément de tableau qui est lui-même un tableau JSON SHALL produire `STRUCTURE_002` au chemin `<path>[i]` (location = span de l'élément, message `Element '<path>[i]' is a JSON array inside an array`) ; cet élément n'est pas parcouru plus loin (aucun contrôle de type, imbriqué ou de ressource embarquée), aucun élément FHIR ne portant un tableau comme valeur (R5 json.html). Le XML ne peut pas produire cette forme.

#### Scenario: Élément répétable sous forme d'objet

- **WHEN** un Patient JSON contient `"name": {"family": "Doe"}`
- **THEN** un Diagnostic `STRUCTURE_002` SHALL être émis avec `path` = `Patient.name`

#### Scenario: Élément max="1" sous forme de tableau

- **WHEN** un Patient JSON contient `"gender": ["male"]`
- **THEN** un Diagnostic `STRUCTURE_002` SHALL être émis avec `path` = `Patient.gender`

#### Scenario: Forme incorrecte en profondeur et dans une extension

- **WHEN** un Patient JSON contient `"name": [{"given": "A"}]` et `"extension": [{"url": "http://example.org/x", "valueString": ["a"]}]`
- **THEN** un Diagnostic `STRUCTURE_002` SHALL être émis avec `path` = `Patient.name[0].given`, et un autre avec `path` = `Patient.extension[0].valueString`

#### Scenario: Tableau dans un tableau

- **WHEN** un Patient JSON contient `"name": [[{"family": "x"}]]`
- **THEN** seul un Diagnostic `STRUCTURE_002` SHALL être émis, avec `path` = `Patient.name[0]`

#### Scenario: Tableau dans un tableau en profondeur

- **WHEN** un Patient JSON contient `"name": [{"given": [["a"]]}]`
- **THEN** seul un Diagnostic `STRUCTURE_002` SHALL être émis, avec `path` = `Patient.name[0].given[0]`

#### Scenario: Tableau dans contained

- **WHEN** un Patient JSON contient `"contained": [[{"resourceType": "Patient"}]]`
- **THEN** seul un Diagnostic `STRUCTURE_002` SHALL être émis, avec `path` = `Patient.contained[0]`

#### Scenario: Élément XML unique non signalé

- **WHEN** un Patient XML contient un unique élément `<name>`
- **THEN** aucun Diagnostic `STRUCTURE_002` ne SHALL être émis

### Requirement: Validation structurelle — tableau JSON vide (STRUCTURE_005)

Pour une ressource issue du JSON uniquement, à toute profondeur du parcours (premier niveau, `BackboneElement`, types complexes, `Extension`, variantes `[x]`, ressources contenues ou embarquées, éléments porteurs de ressources comme `contained: []`), un champ dont la valeur JSON est un tableau vide `[]` SHALL produire un `Diagnostic` `STRUCTURE_005` de severity `Error`, avec `path` = chemin d'instance, `location` = span du tableau, et message `Array cannot be empty at '<path>': omit the property if it has no values` (R5 resource-formats.html#comparison : « JSON objects and arrays are never empty »). Une extension de primitif `_nom: []` (où `nom` résout un type primitif) SHALL être signalée au chemin de la propriété `…nom` (et non `…_nom`), une seule fois lorsque `nom` vaut aussi `[]`, et aussi lorsque `nom` porte des valeurs (`"given": ["P"], "_given": []`). Un tableau de seuls `null` ne SHALL PAS recevoir `STRUCTURE_005` (json.html#primitive : SHOULD omettre ; le `CARDINALITY_003` Warning demeure). Un nom d'élément inconnu ne reçoit que `STRUCTURE_001`. Une ressource issue de l'XML ne SHALL jamais être soumise à ce contrôle. Coexistence : un `[]` reçoit aussi `CARDINALITY_003` (min=0) ou `CARDINALITY_001` (min>0), et `STRUCTURE_002` sur un élément de `max="1"`.

#### Scenario: Élément optionnel imbriqué en tableau vide

- **WHEN** un Bundle JSON contient `"entry": []` sous un `List` (`List.entry: []`)
- **THEN** un Diagnostic `STRUCTURE_005` de severity `Error` SHALL être émis avec `path` = `List.entry` et `location` = span du tableau

#### Scenario: Tableau vide sur given

- **WHEN** un Patient JSON contient `"name": [{"given": []}]`
- **THEN** un Diagnostic `STRUCTURE_005` SHALL être émis avec `path` = `Patient.name[0].given`

#### Scenario: Extension de primitif vide

- **WHEN** un Patient JSON contient `"name": [{"given": ["P"], "_given": []}]`
- **THEN** un Diagnostic `STRUCTURE_005` SHALL être émis avec `path` = `Patient.name[0].given` (et non `_given`)

#### Scenario: Valeur et extension toutes deux vides

- **WHEN** un Patient JSON contient `"name": [{"given": [], "_given": []}]`
- **THEN** un seul Diagnostic `STRUCTURE_005` SHALL être émis pour `Patient.name[0].given`

#### Scenario: Tableau de null

- **WHEN** un Patient JSON contient `"name": [{"given": [null]}]`
- **THEN** aucun Diagnostic `STRUCTURE_005` ne SHALL être émis (seul `CARDINALITY_003` s'applique)

### Requirement: Validation de cardinalité — élément requis absent (CARDINALITY_001)

Le système SHALL émettre un `Diagnostic` avec code `CARDINALITY_001` et severity `Error` pour chaque élément déclaré avec `min > 0` dans le snapshot et absent de la ressource. Une valeur JSON qui est un tableau vide (`[]`) ou un tableau ne contenant que des `null` SHALL compter comme absente, à toute profondeur (parcours unique, réutilisé par la couche de cardinalité de profil) : `"compose": {"include": []}` ou `"include": [null]` dans un ValueSet, ou `"author": []` dans une Composition, produit `CARDINALITY_001`. Un élément requis de type primitif (nom de type commençant par une minuscule) dont la valeur est absente (ou un tableau vide ou de seuls `null`) SHALL être considéré présent via `_nom` (extension de primitif) seulement si la valeur de `_nom` est un objet, ou un tableau d'objets et de `null` contenant au moins un objet ; de même qu'une valeur `[]` ou `[null]`, un `_nom` scalaire tel que `"_status": 5`, `[]` ou `[null]` ne compte pas et produit `CARDINALITY_001` ; ainsi `"status": [null], "_status": [{"id": "a"}]` reste présent. Pour un type complexe, `_nom` ne SHALL PAS compter comme présence. Lorsque la valeur est absente et que `_nom` est un tableau plus long que le maximum fini, un `CARDINALITY_002` SHALL être émis (path et position de `_nom`). Un nom d'élément contenant `.`, `[` ou `]` (y compris préfixé par `_`) SHALL produire `STRUCTURE_001`. Un choix `[x]` SHALL être vérifié selon le seul type désigné par son suffixe ; sa cardinalité porte sur l'ensemble de ses variantes (champs `base` + type autorisé capitalisé, `_variante` comptant seulement pour un type primitif et sous la même forme objet que `_nom`), à toute profondeur : aucune variante et `min > 0` produit `CARDINALITY_001` au chemin du choix (ex. `Immunization.occurrence[x]`), plus d'une variante produit `CARDINALITY_002` à la seconde variante. La règle du tableau vide ne s'applique pas aux variantes `[x]` (de `max="1"`, un tableau y produit déjà `STRUCTURE_002`). Un `integer64` SHALL être une chaîne conforme à `[0]|[-+]?[1-9][0-9]*` et représentable sur 64 bits signés (`TYPE_003` sinon, `TYPE_001` si non chaîne). Lorsque l'élément est présent, le système SHALL compter ses valeurs : les éléments non `null` (un scalaire compte pour un, un scalaire `null` pour zéro) ; un élément `null` compte lorsque l'objet `_nom` de la primitive au même index le remplace (json.html) ; pour une variante de choix, `_variante` ne compte que pour un type primitif. Si le nombre est inférieur à `min` (couche de base ou de profil, ex. profil de base `lipidprofile`, `DiagnosticReport.result` min=3 avec un seul résultat), un `CARDINALITY_001` Error SHALL être émis au chemin de l'élément, `location` = son span, message `Element '<path>' has <n> value(s) but minimum is <min>` (ElementDefinition.min : « minimum number of times this element SHALL appear »).

#### Scenario: Élément requis absent

- **WHEN** une ressource Observation est fournie sans le champ `status` (min=1)
- **THEN** un Diagnostic `CARDINALITY_001` SHALL être émis mentionnant `status`

#### Scenario: Élément optionnel absent ne produit pas d'erreur

- **WHEN** une ressource est fournie sans un champ optionnel (min=0)
- **THEN** aucun Diagnostic `CARDINALITY_001` ne SHALL être émis pour ce champ

#### Scenario: Min de profil sur un élément présent

- **WHEN** un DiagnosticReport conforme au profil `lipidprofile` (`DiagnosticReport.result` min=3) contient un seul `result`
- **THEN** un Diagnostic `CARDINALITY_001` SHALL être émis au chemin `DiagnosticReport.result` avec le message `Element 'DiagnosticReport.result' has 1 value(s) but minimum is 3`

#### Scenario: Scalaire null sur un élément requis

- **WHEN** une Observation contient `"status": null`
- **THEN** un Diagnostic `CARDINALITY_001` SHALL être émis (en plus de `TYPE_001`) ; avec `"status": null, "_status": {"id": "a"}`, aucun `CARDINALITY_001` ne SHALL être émis

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

### Requirement: Validation de cardinalité — dépassement du maximum (CARDINALITY_002)

Le système SHALL émettre un `Diagnostic` avec code `CARDINALITY_002` et severity `Error` lorsqu'un élément contient plus de valeurs que son `max` déclaré (lorsque `max` est un entier fini).

#### Scenario: Tableau dépassant le max

- **WHEN** un champ déclaré avec `max="1"` contient un tableau de 2 éléments
- **THEN** un Diagnostic `CARDINALITY_002` SHALL être émis indiquant le count et le max

#### Scenario: Max illimité (*) n'est jamais violé

- **WHEN** un champ est déclaré avec `max="*"`
- **THEN** aucun Diagnostic `CARDINALITY_002` ne SHALL être émis quel que soit le nombre de valeurs

---

### Requirement: Validation de type — erreur de type de base (TYPE_001)

Le système SHALL émettre un `Diagnostic` avec code `TYPE_001` et severity `Error` lorsqu'une valeur n'est pas du type JSON attendu par la StructureDefinition (ex. : nombre fourni pour un champ `string`, ou chaîne pour un champ `boolean`). Pour un champ polymorphique de premier niveau (`value[x]`), le type vérifié SHALL être celui sélectionné par le suffixe du champ. Une valeur scalaire (chaîne, nombre, booléen ou `null`) fournie pour un élément dont le type est un datatype complexe, `BackboneElement` ou `Element` SHALL produire `TYPE_001` avec le message `Expected <type> object at '<path>'` ; un tableau SHALL être vérifié élément par élément. Dans un tableau primitif répétitif, un `null` à l'index i SHALL NOT produire `TYPE_001` lorsque `_nom[i]` est un objet NON VIDE (json.html : les objets ne sont jamais vides ; id/extensions d'une répétition sans valeur), en JSON comme en XML ; sans objet non vide à `_nom[i]` (dont `{}`), `TYPE_001` SHALL être émis.

Les types `string`, `markdown`, `base64Binary`, `uri`, `url`, `canonical`, `oid`, `uuid` et `xhtml` SHALL recevoir une chaîne JSON ; toute autre valeur JSON SHALL produire `TYPE_001`.

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

- **WHEN** un champ de type `integer` contient le littéral JSON `-0` (parsé en `Value::Decimal` nul)
- **THEN** un Diagnostic `TYPE_001` SHALL être émis

#### Scenario: uri attendue, nombre reçu

- **WHEN** un champ de type `uri`, `markdown`, `oid` ou `uuid` contient une valeur JSON numérique
- **THEN** un Diagnostic `TYPE_001` SHALL être émis

#### Scenario: null dans un tableau primitif avec _nom objet

- **WHEN** un Patient contient `"given": ["Jim", null], "_given": [null, {"extension": [...]}]` (json.html 2.1.6.4.2, JSON et XML)
- **THEN** aucun `TYPE_001` SHALL être émis pour `Patient.name[0].given[1]`

#### Scenario: null dans un tableau primitif sans _nom objet

- **WHEN** un Patient contient `"given": ["Jim", null]` seul (ou avec `_given` absent, plus court, `null` ou `{}` à cet index)
- **THEN** un Diagnostic `TYPE_001` SHALL être émis à `Patient.name[0].given[1]`

### Requirement: Validation de type — contrainte de valeur entière (TYPE_002)

Le système SHALL émettre un `Diagnostic` avec code `TYPE_002` et severity `Error` lorsqu'une valeur entière viole une contrainte de valeur : valeur négative pour `unsignedInt`, valeur inférieure ou égale à zéro pour `positiveInt`, ou valeur d'un `integer`, `unsignedInt` ou `positiveInt` hors de la plage des entiers signés 32 bits (maximum `2147483647` ; minimum `-2147483648` pour `integer`). Une valeur de type chaîne SHALL aussi produire `TYPE_002` lorsque son nombre de caractères (et non d'octets) dépasse le `maxLength` de `<type>.value` dans le Registry (R5 : `string`, 1 048 576) ; `Element.id` étant vérifié comme `string`, la limite s'y applique aussi.

#### Scenario: unsignedInt négatif

- **WHEN** un champ de type `unsignedInt` contient la valeur JSON `-1`
- **THEN** un Diagnostic `TYPE_002` SHALL être émis

#### Scenario: positiveInt à zéro

- **WHEN** un champ de type `positiveInt` contient la valeur JSON `0`
- **THEN** un Diagnostic `TYPE_002` SHALL être émis

#### Scenario: integer hors de la plage 32 bits

- **WHEN** `valueInteger` contient la valeur JSON `2147483648`
- **THEN** un Diagnostic `TYPE_002` SHALL être émis

#### Scenario: string plus longue que maxLength

- **WHEN** un champ de type `string` contient 1 048 577 caractères
- **THEN** un Diagnostic `TYPE_002` SHALL être émis

#### Scenario: maxLength compté en caractères

- **WHEN** un champ de type `string` contient 1 048 576 caractères multi-octets (par exemple `é`)
- **THEN** aucun Diagnostic `TYPE_002` ne SHALL être émis

#### Scenario: Element.id plus long que maxLength

- **WHEN** un `Element.id` contient plus de 1 048 576 caractères
- **THEN** un Diagnostic `TYPE_002` SHALL être émis

---

### Requirement: Validation de type — format date/heure et expressions régulières R5 (TYPE_003)

Le système SHALL émettre un `Diagnostic` avec code `TYPE_003` et severity `Error` lorsqu'une valeur de type `string`, `markdown`, `base64Binary`, `uri`, `url`, `canonical`, `oid` ou `uuid` ne correspond pas à l'expression régulière R5 du type, lue depuis le Registry et ancrée en `\A(?:...)\z`. Une chaîne vide pour ces types SHALL produire `TYPE_003` (R5 json.html : les valeurs de propriété string ne peuvent jamais être vides ; les regex `uri` et `base64Binary` acceptent `""`). Une valeur non vide composée uniquement de whitespace qui satisfait la regex (en pratique `string`, `markdown` et `base64Binary`, dont le whitespace est ignoré) SHALL produire `TYPE_008` et non `TYPE_003` (voir TYPE_008). Le message `TYPE_003` SHALL être `Invalid <type> format at '<path>': '<value>'`, sauf pour `base64Binary` : `Invalid base64Binary format at '<path>'` (sans la charge utile). Pour `base64Binary`, le whitespace ASCII SHALL être ignoré avant l'application de la regex (R5 datatypes.html, RFC 4648). Une valeur `canonical` qui passe la regex et `maxLength` mais se termine par `|` (version vide après la barre, ex. `http://x.org/SD/x|`, `|`, `#x|`) SHALL produire `TYPE_003` avec le message `Canonical at '<path>' has an empty version after '|': '<value>'` (R5 datatypes.html : la version est ajoutée après `|`, et une version est une chaîne, jamais vide ; cas officiel patient-ig-ig) ; `uri` et `url` terminés par `|` ne sont pas concernés. `xhtml` n'a pas de regex R5 (chaîne uniquement). La famille des dates, les types entiers, `integer64`, `code` et `id` conservent des contrôles écrits à la main. `decimal` suit la regex R5 lue telle qu'écrite (le `{1,9}}` publié lu `{1,9}`) : voir les scénarios « Décimal XML selon la regex R5 » et « Décimal JSON selon la regex R5 ».

Pour les types date/heure, le format SHALL être conforme aux expressions régulières R5 :

- `date` : `YYYY(-MM(-DD)?)?`, année de `0001` à `9999`, jour valide pour le mois (années bissextiles comprises) ;
- `dateTime` : une `date`, ou une date complète suivie de `Thh:mm:ss(.f{1,9})?` et d'un fuseau obligatoire `Z` ou `(+|-)hh:mm` (décalage au plus `14:00`) ; conformément à la regex R5, un fuseau SHALL être accepté sur une valeur réduite à une année-mois ou à une date complète (`2020-01Z`, `2020-01-01+02:00`), mais SHALL être rejeté sur une année seule (`2020Z`) ; une heure sans fuseau SHALL être rejetée ;
- `instant` : toujours la forme complète avec fuseau ;
- `time` : `hh:mm:ss(.f{1,9})?` sans fuseau, heures `00`-`23`, minutes `00`-`59`, secondes `00`-`60`.

#### Scenario: Date invalide

- **WHEN** un champ de type `date` contient la chaîne `"not-a-date"`
- **THEN** un Diagnostic `TYPE_003` SHALL être émis

#### Scenario: Date valide ne produit pas d'erreur

- **WHEN** un champ de type `date` contient `"2024-01-15"`
- **THEN** aucun Diagnostic `TYPE_003` ne SHALL être émis

#### Scenario: Jour inexistant pour le mois

- **WHEN** un champ de type `date` contient `"2020-02-31"`
- **THEN** un Diagnostic `TYPE_003` SHALL être émis

#### Scenario: instant partiel

- **WHEN** un champ de type `instant` contient `"2020"`
- **THEN** un Diagnostic `TYPE_003` SHALL être émis

#### Scenario: dateTime suivi de caractères parasites

- **WHEN** un champ de type `dateTime` contient `"2020-01-01T10:00:00garbage"`
- **THEN** un Diagnostic `TYPE_003` SHALL être émis

#### Scenario: dateTime réduit à la date avec fuseau

- **WHEN** un champ de type `dateTime` contient `"2020-01Z"`, `"2020-01-01Z"` ou `"2020-01-01+02:00"`
- **THEN** aucun Diagnostic `TYPE_003` ne SHALL être émis

#### Scenario: dateTime réduit à l'année avec fuseau

- **WHEN** un champ de type `dateTime` contient `"2020Z"`
- **THEN** un Diagnostic `TYPE_003` SHALL être émis

#### Scenario: uri avec espace

- **WHEN** un champ de type `uri` contient `"http://a b"`
- **THEN** un Diagnostic `TYPE_003` SHALL être émis

#### Scenario: oid valide

- **WHEN** un champ de type `oid` contient `"urn:oid:1.2.3"`
- **THEN** aucun Diagnostic `TYPE_003` ne SHALL être émis

#### Scenario: oid invalide

- **WHEN** un champ de type `oid` contient `"1.2.3"`
- **THEN** un Diagnostic `TYPE_003` SHALL être émis

#### Scenario: string vide

- **WHEN** un champ de type `string` contient `""`
- **THEN** un Diagnostic `TYPE_003` SHALL être émis

#### Scenario: string whitespace seul

- **WHEN** un champ de type `string` contient `"   "`
- **THEN** un Diagnostic `TYPE_008` de severity `Warning` SHALL être émis et aucun `TYPE_003`

#### Scenario: Message base64Binary

- **WHEN** un champ de type `base64Binary` contient `"not base64!"`
- **THEN** le message `TYPE_003` SHALL être `Invalid base64Binary format at '<path>'`, sans la valeur

#### Scenario: base64Binary avec whitespace ignoré

- **WHEN** un champ de type `base64Binary` contient `"aGVs bG8="` ou une valeur base64 coupée par des sauts de ligne
- **THEN** aucun Diagnostic `TYPE_003` ne SHALL être émis

#### Scenario: base64Binary invalide

- **WHEN** un champ de type `base64Binary` contient `"not base64!"`
- **THEN** un Diagnostic `TYPE_003` SHALL être émis

#### Scenario: canonical avec version vide

- **WHEN** un champ de type `canonical` contient `"http://x.org/SD/x|"`
- **THEN** un Diagnostic `TYPE_003` SHALL être émis avec le message `Canonical at '<path>' has an empty version after '|': 'http://x.org/SD/x|'`
- **AND** un `uri` ou `url` de même valeur ne SHALL produire aucun Diagnostic `TYPE_003`

### Requirement: Validation de type — whitespace dans un code (TYPE_004)

Le système SHALL émettre un `Diagnostic` avec code `TYPE_004` et severity `Error` lorsqu'un champ de type `code` ne respecte pas la regex R5 `[^\s]+( [^\s]+)*` : des mots non vides séparés par un unique espace. Une chaîne vide, un espace en début ou en fin, deux espaces consécutifs, ou tout whitespace autre qu'un espace simple (tabulation, saut de ligne, espace insécable...) SHALL produire un `TYPE_004`.

#### Scenario: Code avec espaces simples entre mots

- **WHEN** un champ de type `code` contient `"text/plain; charset=utf-8"` ou `"a b c"`
- **THEN** aucun Diagnostic `TYPE_004` ne SHALL être émis

#### Scenario: Code avec espace mal placé

- **WHEN** un champ de type `code` contient `"a  b"` (deux espaces) ou `" a"` (espace initial)
- **THEN** un Diagnostic `TYPE_004` SHALL être émis

#### Scenario: Code sans whitespace

- **WHEN** un champ de type `code` contient `"final"`
- **THEN** aucun Diagnostic `TYPE_004` ne SHALL être émis

---

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

### Requirement: Validation des invariants FHIRPath (INVARIANT_<key>)

Le système SHALL évaluer les expressions FHIRPath de contrainte définies sur l'élément du snapshot de chaque nœud d'instance, ainsi que sur l'élément racine de son type, à toute profondeur (par ex. `qty-3` sur toute `Quantity`, `ext-1` sur toute `Extension`) ; chaque clé SHALL être évaluée une seule fois par nœud. Le `path` du Diagnostic SHALL être le chemin d'instance du nœud (ex. `Patient.contact[0]`) et sa localisation le span du nœud ; les contraintes de la ressource racine restent signalées au chemin du type de ressource, sans span. Les contraintes racine d’une ressource SHALL être celles de son élément racine et celles des éléments racine de ses ancêtres ressource (`DomainResource`, `Resource` ; `Base` exclu car son `ele-1` est une règle d’élément), chaque clé une seule fois ; le R5 publie 37 racines dérivées de `DomainResource` (ex. `ValueSet`, `CodeSystem`) sans `dom-2` à `dom-6`, qui SHALL quand même leur être appliquées par cet héritage. Un primitif SHALL être évalué avec son élément JSON `_nom`, appariés par indice ; un `_nom` sans valeur SHALL être évalué au chemin nu de l'élément (ex. `Patient.name[0].family`) ; le `_id` de la racine SHALL être apparié à l'`id` de la ressource, que le parseur extrait des champs ; un item JSON `null` n'est pas évalué. Chaque invariant (racine, imbriqué, profil de type) SHALL être évalué par `fhir_fhirpath::evaluate_parsed_with_environment` avec un `Environment` dont : `member_of` est construit depuis `Registry::value_set_contains` (`memberOf`) ; `is_subtype` est construit depuis `Registry::type_ancestors` (`is`, `as` et `ofType` répondent pour les sous-types FHIR : `canonical` et `url` sont des `uri`, `code` est un `string`) ; `%resource` est la ressource englobant le nœud à toute profondeur (une ressource contenue ou une entrée de Bundle est son propre `%resource`) ; `%rootResource` est le conteneur le plus externe pour une ressource contenue, sinon `%resource` (une entrée de Bundle est sa propre racine). Une valeur sans type FHIR (élément inconnu, STRUCTURE_001/TYPE_001) interrogée sur un sous-type FHIR de `string` SHALL donner `INVARIANT_SKIPPED`. `%terminologies` SHALL y répondre depuis le `Registry` en cours d'usage pour la validation (la surcouche `Registry::with_extra` quand des définitions sont fournies, sinon la base) : `expand`, `validateVS` et `translate` SHALL être évalués, et une réponse que le `Registry` ne peut décider SHALL être une erreur d'évaluation, donc `INVARIANT_SKIPPED`. Chaque expression SHALL être analysée une seule fois par contrainte (cache). La ressource SHALL être convertie par `fhir_fhirpath::resource_to_typed_value` (racine) ou `fhir_fhirpath::node_to_typed_value` (nœuds imbriqués, même typage) avec `Registry::child_element`, de sorte que les primitives `date`, `dateTime`, `instant` et `time` soient comparées comme des valeurs temporelles. Pour chaque contrainte dont l'expression ne retourne pas `[true]` (une seule valeur vraie, y compris un booléen FHIR portant des extensions), il SHALL émettre un `Diagnostic` avec code `INVARIANT_<key>` où `<key>` est la valeur du champ `key` de la contrainte. Le code `INVARIANT_SKIPPED` est réservé : il signale une contrainte non vérifiée et n'est jamais une violation. `fhir_validator::validate` SHALL toujours évaluer le FHIRPath avec la navigation stricte (N1 §3) : il construit un seul `fhir_fhirpath::Strict` par appel, à partir de `Registry::has_element`, partagé par toutes les évaluations de l'arbre de la ressource (racine et ressources imbriquées) : invariants racine du noyau, invariants d'éléments imbriqués, contraintes de profil (racine, élément, profils de type) et discriminateurs de découpage. Une expression nommant un élément que le type ne déclare pas échoue à l'évaluation et SHALL être signalée `INVARIANT_SKIPPED` (jamais une violation : la spécification FHIR ne rend pas l'instance invalide à cause d'une expression de profil invalide). Le message de `INVARIANT_SKIPPED` SHALL avoir la forme `constraint <key> not checked: <reason> (in '<expression>')`, nommant la clé, la raison (ex. `type error: familly is not an element of HumanName`) et l'expression. La severity R5 d'une contrainte est liée (binding required) à constraint-severity (`error | warning`) : seule la valeur exacte `warning` SHALL donner `Severity::Warning` ; toute autre valeur (`error`, mais aussi `information`, `Warning` ou un code inconnu) SHALL donner `Severity::Error`.

#### Scenario: Contraintes racine héritées

- **WHEN** une ressource dérive de `DomainResource` (ex. `Patient`)
- **THEN** les contraintes racine de `DomainResource` et de `Resource` SHALL être évaluées en plus des siennes, une seule fois par clé ; pour `ValueSet` (racine publiée sans `dom-2` à `dom-6`) ces clés SHALL être appliquées par héritage, mais jamais `ele-1` de `Base`

#### Scenario: Contrainte violée émet INVARIANT_<key>

- **WHEN** une contrainte avec `key="dom-2"` et `expression="..."` évalue à `[false]`
- **THEN** un Diagnostic avec code `INVARIANT_dom-2` SHALL être émis avec la severity correspondant à la severity de la contrainte

#### Scenario: Contrainte satisfaite n'émet pas de diagnostic

- **WHEN** une contrainte FHIRPath évalue à `[true]`
- **THEN** aucun Diagnostic `INVARIANT_` ne SHALL être émis pour cette contrainte

#### Scenario: Erreur d'évaluation FHIRPath signalée INVARIANT_SKIPPED

- **WHEN** l'évaluation FHIRPath d'une contrainte retourne une erreur
- **THEN** aucun Diagnostic `INVARIANT_<key>` ne SHALL être émis pour cette contrainte
- **AND** exactement un Diagnostic de severity `Information` avec code `INVARIANT_SKIPPED` SHALL être émis, au `path` du type de ressource pour une contrainte de la racine ou au chemin d'instance du nœud pour une contrainte imbriquée, avec un message `constraint <key> not checked: <reason> (in '<expression>')` nommant la `key` de la contrainte, la raison de l'erreur et l'expression

#### Scenario: Collection FHIRPath trop grande signalée INVARIANT_SKIPPED

- **WHEN** l'évaluation d'une contrainte retourne `EvalError::CollectionTooLarge` (collection intermédiaire de plus de 100 000 items)
- **THEN** exactement un Diagnostic `Information` `INVARIANT_SKIPPED` nommant la `key` de la contrainte et « collection larger than 100000 items » SHALL être émis

#### Scenario: Chaîne FHIRPath trop longue signalée INVARIANT_SKIPPED

- **WHEN** l'évaluation d'une contrainte retourne `EvalError::StringTooLong` (chaîne construite de plus de 1 048 576 caractères)
- **THEN** exactement un Diagnostic `Information` `INVARIANT_SKIPPED` nommant la `key` de la contrainte et « string longer than 1048576 characters » SHALL être émis

#### Scenario: Sous-type FHIR reconnu par is/ofType

- **WHEN** une `Patient` porte `extension` avec `valueCanonical` `#b` et `contained` une `Basic` d'`id` `b`
- **THEN** `dom-3` (`ofType(canonical)` / `ofType(uri)`) SHALL être évaluée et ne produire aucun Diagnostic

#### Scenario: dom-3 sur ressource contenue non référencée

- **WHEN** une `Patient` contient une ressource `Basic` que rien ne référence
- **THEN** un Diagnostic `INVARIANT_dom-3` de severity `Error` SHALL être émis au chemin `Patient`

#### Scenario: dom-3 satisfait par un lien du narratif (JSON)

- **WHEN** une `Patient` JSON contient une `Basic` d'`id` `b` et un `text.div` xhtml avec `<a href="#b">`
- **THEN** aucun Diagnostic `INVARIANT_dom-3` SHALL être émis

#### Scenario: dom-3 satisfait par une image du narratif (XML)

- **WHEN** une `Patient` XML contient une `Basic` d'`id` `b` et un narratif avec `<img src="#b"/>`
- **THEN** aucun Diagnostic `INVARIANT_dom-3` SHALL être émis

#### Scenario: dom-3 non satisfait par du texte ou une URL externe

- **WHEN** une `Basic` contenue n'est nommée que dans le texte du narratif, un autre attribut ou une URL externe
- **THEN** un Diagnostic `INVARIANT_dom-3` de severity `Error` SHALL être émis au chemin `Patient`

#### Scenario: ref-1 résolu contre le conteneur

- **WHEN** une `Observation` contenue référence `#` (le conteneur) et `#b` (une sœur contenue)
- **THEN** aucun Diagnostic `INVARIANT_ref-1` SHALL être émis

#### Scenario: ref-1 sur contained manquant

- **WHEN** une `Observation` contenue référence `#missing`
- **THEN** un Diagnostic `INVARIANT_ref-1` SHALL être émis à `Patient.contained[0].subject`

#### Scenario: Entrée de Bundle est sa propre racine

- **WHEN** une `Observation` d'entrée de Bundle référence `#c` (son propre `contained`) et `#`
- **THEN** `#c` SHALL être résolu, et `#` SHALL produire `INVARIANT_ref-1` à `Bundle.entry[0].resource.focus[0]`

#### Scenario: Ressource contenue dans une entrée de Bundle

- **WHEN** une `Patient` d'entrée de Bundle contient une `Observation` référençant `#`, `#b` et `#z`
- **THEN** `%rootResource` SHALL être la `Patient` de l'entrée (pas le Bundle) et seul `#z` SHALL produire `INVARIANT_ref-1` à `Bundle.entry[0].resource.contained[0].focus[1]`

#### Scenario: Expression non analysable signalée INVARIANT_SKIPPED

- **WHEN** l'expression FHIRPath d'une contrainte ne peut pas être analysée (par ex. une fonction non implémentée comme `timeOfDay`)
- **THEN** la contrainte SHALL être ignorée tant que cette fonction n'est pas implémentée
- **AND** exactement un Diagnostic `Information` `INVARIANT_SKIPPED` nommant la `key`, la raison et l'expression SHALL être émis, afin qu'un résultat sans erreur ne masque jamais une contrainte non vérifiée

#### Scenario: Invariants txt-1 et txt-2 évalués

- **WHEN** un `text.div` contient un balisage refusé par `htmlChecks()` (élément, attribut ou URL interdits, ou aucun contenu)
- **THEN** `txt-1` et `txt-2` (qui partagent `htmlChecks()`) SHALL produire chacun une erreur à `<chemin>.text.div`, à toute profondeur (ressources `contained` comprises)
- **AND** aucun `INVARIANT_SKIPPED` ne SHALL être émis pour ces deux contraintes

#### Scenario: Invariant dom-6 évalué (identifiant entre backticks)

- **WHEN** une DomainResource R5 n'a pas de `text.div` (contrainte `dom-6`, ``text.`div`.exists()``, severity `warning`)
- **THEN** un Diagnostic `Warning` avec code `INVARIANT_dom-6` SHALL être émis
- **AND** aucun Diagnostic `INVARIANT_SKIPPED` ne SHALL être émis pour `dom-6`

#### Scenario: Invariant dgr-1 ignoré (référence non suivable par resolve())

- **WHEN** une DiagnosticReport R5 avec `composition` est validée (contrainte `dgr-1`, dont l'expression appelle `resolve()` sur une référence serveur non suivable)
- **THEN** aucun Diagnostic `INVARIANT_dgr-1` ne SHALL être émis
- **AND** exactement un Diagnostic `Information` `INVARIANT_SKIPPED` nommant `dgr-1` SHALL être émis

#### Scenario: %terminologies décidé conforme

- **WHEN** une contrainte appelle `%terminologies.validateVS(...)` ou `expand(...)` avec un ValueSet que le `Registry` sait énumérer et que le code ou l'expansion le satisfait
- **THEN** aucun Diagnostic ne SHALL être émis pour cette contrainte

#### Scenario: %terminologies décidé non conforme

- **WHEN** la même contrainte est évaluée avec un code absent du ValueSet ou une expansion qui ne contient pas le code
- **THEN** un Diagnostic `INVARIANT_<key>` SHALL être émis
- **AND** aucun `INVARIANT_SKIPPED` ne SHALL être émis pour cette contrainte

#### Scenario: %terminologies indécidable ignoré

- **WHEN** une contrainte appelle `%terminologies` avec un ValueSet que le `Registry` ne peut pas énumérer
- **THEN** exactement un Diagnostic `Information` `INVARIANT_SKIPPED` nommant la `key` SHALL être émis
- **AND** aucun Diagnostic `INVARIANT_<key>` ne SHALL être émis pour cette contrainte

#### Scenario: Severity hors de constraint-severity mappée en Error

- **WHEN** une contrainte violée a `severity="information"` ou `severity="Warning"` (codes sensibles à la casse)
- **THEN** le Diagnostic `INVARIANT_<key>` SHALL avoir la severity `Error`

#### Scenario: Contrainte sans expression ignorée

- **WHEN** une contrainte n'a pas de champ `expression`
- **THEN** aucun Diagnostic ne SHALL être émis pour cette contrainte

#### Scenario: Invariants évalués avec navigation des éléments choix

- **WHEN** une contrainte est évaluée
- **THEN** elle SHALL l'être via `evaluate_parsed_with_choices` avec `Registry::is_choice_variant`, de sorte qu'un nom de base (`value`) trouve sa variante (`valueQuantity`)

#### Scenario: fhs-2 et mrp-2 sans faux positif

- **WHEN** une ressource valide renseigne l'élément choix visé par `fhs-2` ou `mrp-2` via une variante typée
- **THEN** aucun Diagnostic `INVARIANT_fhs-2` ni `INVARIANT_mrp-2` ne SHALL être émis

#### Scenario: obs-6 détecte dataAbsentReason avec valueQuantity

- **WHEN** une Observation contient à la fois `dataAbsentReason` et `valueQuantity`
- **THEN** un Diagnostic `INVARIANT_obs-6` SHALL être émis

#### Scenario: app-5 compare des instants, pas du texte

- **WHEN** une Appointment a `start` = `2020-01-01T10:00:00+02:00` et `end` = `2020-01-01T09:30:00Z` (`app-5`, `start <= end`)
- **THEN** aucun Diagnostic `INVARIANT_app-5` ne SHALL être émis (08:00Z précède 09:30Z, alors que le texte « 10 » suit « 09 »)

#### Scenario: Primitive sans valeur dans un invariant

- **WHEN** une Bundle `document` n'a que `_timestamp` (extension data-absent-reason) (`bdl-10`)
- **THEN** un Diagnostic `INVARIANT_bdl-10` SHALL être émis

#### Scenario: Invariant évalué sur un nœud imbriqué

- **WHEN** un `Patient.contact[0]` viole une contrainte de son élément de snapshot, ou une `Quantity` viole `qty-3`
- **THEN** un Diagnostic `INVARIANT_<key>` SHALL être émis avec pour `path` le chemin d'instance du nœud (ex. `Patient.contact[0]`) et pour localisation le span du nœud

#### Scenario: Clé évaluée une fois par nœud

- **WHEN** un nœud porte une contrainte à la fois sur son élément de snapshot et sur la racine de son type avec la même clé
- **THEN** cette clé SHALL être évaluée une seule fois pour ce nœud

#### Scenario: Contrainte racine sans span

- **WHEN** une contrainte de la ressource racine est violée
- **THEN** le Diagnostic SHALL avoir pour `path` le type de ressource et aucun span

#### Scenario: Primitive avec _nom sans valeur

- **WHEN** `Patient.name[0]` n'a que `_family` sans `family`
- **THEN** les contraintes de `family` SHALL être évaluées au chemin `Patient.name[0].family`, et un item JSON `null` ne SHALL pas être évalué

#### Scenario: INVARIANT_SKIPPED au chemin du nœud

- **WHEN** l'expression d'une contrainte d'un nœud imbriqué ne s'analyse pas ou échoue
- **THEN** exactement un Diagnostic `Information` `INVARIANT_SKIPPED` SHALL être émis par nœud, au chemin de ce nœud

#### Scenario: Élément inconnu dans une contrainte de profil

- **WHEN** une contrainte de profil a l'expression `name.familly.exists()` sur une `Patient`
- **THEN** exactement un Diagnostic `Information` `INVARIANT_SKIPPED` SHALL être émis, nommant `familly` et l'expression
- **AND** aucun Diagnostic `INVARIANT_<key>` ne SHALL être émis pour cette contrainte

### Requirement: Validation terminologique — binding required (TERMINOLOGY_001)

Le système SHALL émettre `TERMINOLOGY_001` lorsqu'un élément, à toute profondeur (y compris dans un datatype, ex. `HumanName.use`, et pour une variante de choix dont le binding s'applique au type résolu de la variante, ex. `versionAlgorithmCoding`), de type `code`, `Coding` ou `CodeableConcept`, possède un binding de force `required` (Diagnostic d'erreur) ou `extensible` (Diagnostic de sévérité warning) dont le ValueSet est connu et que le code est prouvé hors du ValueSet. Les éléments sous un `contentReference` et dans les ressources embarquées SHALL être vérifiés comme tout autre élément (parcours unique) ; les autres forces de binding (`preferred`, `example`) SHALL être ignorées. Un `Coding` sans `system` NE DOIT JAMAIS être considéré comme membre du ValueSet : il compte comme hors du ValueSet, même lorsque l'appartenance serait autrement indécidable. Hors de ce cas, une appartenance indécidable (`Registry::value_set_contains` retourne `None`) NE DOIT JAMAIS être signalée. Pour un `CodeableConcept`, un seul `coding` appartenant au ValueSet suffit ; le Diagnostic n'est émis que si tous les `coding` portant un code sont hors du ValueSet. Sous un binding `required`, un `CodeableConcept` sans aucun `coding` portant un code (texte seul) SHALL être signalé avec un message commençant par `CodeableConcept with no coded Coding at` (terminologies.html 4.1.4.2.1) ; sous un binding `extensible`, le texte seul SHALL être accepté (4.1.4.2.2). Sous un binding `required`, un élément de type `Coding` sans `code` SHALL être signalé avec un message commençant par `Coding with no code at`, comme un `CodeableConcept` sans `coding` codé ; sous un binding `extensible`, il NE DOIT PAS être signalé. Un `coding` unique non encapsulé dans un tableau (un seul `<coding>` en XML) SHALL être lu comme un tableau d'un élément.

#### Scenario: Code hors du ValueSet requis

- **WHEN** un `code` lié en `required` à un ValueSet connu contient une valeur absente de ce ValueSet
- **THEN** un Diagnostic d'erreur `TERMINOLOGY_001` SHALL être émis avec le `path` de l'élément (index inclus pour les tableaux)

#### Scenario: Code hors du ValueSet extensible

- **WHEN** un `CodeableConcept` lié en `extensible` à un ValueSet connu ne contient que des `coding` prouvés hors de ce ValueSet
- **THEN** un Diagnostic `TERMINOLOGY_001` de sévérité warning SHALL être émis

#### Scenario: Coding sans code sous binding required

- **WHEN** `Appointment.recurrenceTemplate.monthlyTemplate.dayOfWeek` (`Coding` lié en `required`) ne porte qu'un `system`
- **THEN** un Diagnostic d'erreur `TERMINOLOGY_001` au message commençant par `Coding with no code at` SHALL être émis

#### Scenario: Binding preferred ou example ignoré

- **WHEN** l'élément a un binding de force `preferred` ou `example`
- **THEN** aucun Diagnostic terminologique SHALL être émis

#### Scenario: Code appartenant au ValueSet

- **WHEN** le code appartient au ValueSet
- **THEN** aucun Diagnostic terminologique SHALL être émis

#### Scenario: Appartenance indécidable

- **WHEN** le ValueSet est inconnu, utilise des filtres, des exclusions ou un CodeSystem fragmentaire/inconnu, et chaque `coding` porte un `system`
- **THEN** aucun Diagnostic `TERMINOLOGY_001` SHALL être émis

#### Scenario: Coding sans system

- **WHEN** un `Coding` lié en `required` porte un `code` mais pas de `system`, même si l'appartenance au ValueSet est indécidable
- **THEN** un Diagnostic d'erreur `TERMINOLOGY_001` SHALL être émis

#### Scenario: CodeableConcept avec un coding valide

- **WHEN** un `CodeableConcept` contient au moins un `coding` appartenant au ValueSet
- **THEN** aucun Diagnostic `TERMINOLOGY_001` SHALL être émis

#### Scenario: CodeableConcept texte seul sous binding required

- **WHEN** un `CodeableConcept` lié en `required` n'a que `text` et aucun `coding` portant un code
- **THEN** un Diagnostic d'erreur `TERMINOLOGY_001` SHALL être émis avec un message commençant par `CodeableConcept with no coded Coding at`

#### Scenario: CodeableConcept texte seul sous binding extensible

- **WHEN** un `CodeableConcept` lié en `extensible` n'a que `text`
- **THEN** aucun Diagnostic SHALL être émis

#### Scenario: Coding XML unique

- **WHEN** un `CodeableConcept` XML lié en `required` contient un seul `<coding>` hors du ValueSet (objet et non tableau)
- **THEN** un Diagnostic `TERMINOLOGY_001` SHALL être émis comme pour un tableau d'un élément

#### Scenario: Élément imbriqué dans un datatype

- **WHEN** `Patient.name[0].use` contient un code hors du ValueSet requis
- **THEN** un Diagnostic `TERMINOLOGY_001` SHALL être émis au `path` `Patient.name[0].use`

#### Scenario: Variante de choix liée

- **WHEN** une variante de choix typée `Coding` (ex. `versionAlgorithmCoding`) est hors du ValueSet lié
- **THEN** un Diagnostic `TERMINOLOGY_001` SHALL être émis, le binding s'appliquant au type résolu de la variante

### Requirement: Validation des références — type inconnu (REFERENCE_001)

Le système SHALL émettre `REFERENCE_001` (erreur) lorsque `Reference.reference` de toute valeur de type `Reference`, à toute profondeur (y compris variantes de choix comme `valueReference`), est une référence relative (sans schéma URI au sens de RFC 3986 ; un `:` hors schéma de tête, comme dans `Foo/a:b`, ne rend pas la référence absolue) dont le type n'est pas un type de ressource concret connu du Registry (type inconnu ou abstrait). Les références conditionnelles (`Patient?identifier=...`) SHALL être acceptées si le type est valide. Les références absolues et `urn:` SHALL être ignorées par cette règle (non résolues) ; les références absolues ne sont vérifiées que par `REFERENCE_003`.

#### Scenario: Type de ressource inconnu

- **WHEN** `reference` vaut `Foo/123` ou `Foo/a:b`
- **THEN** un Diagnostic `REFERENCE_001` SHALL être émis avec `path` se terminant par `.reference`

#### Scenario: Type abstrait

- **WHEN** `reference` vaut `DomainResource/1`
- **THEN** un Diagnostic `REFERENCE_001` SHALL être émis

#### Scenario: Référence conditionnelle, absolue ou urn

- **WHEN** `reference` vaut `Patient?identifier=x`, `http://example.org/Patient/1` ou `urn:uuid:...`
- **THEN** aucun Diagnostic `REFERENCE_001` SHALL être émis

#### Scenario: Référence imbriquée ou variante de choix

- **WHEN** une `Reference` imbriquée ou une variante de choix (ex. `valueReference`) a une référence invalide
- **THEN** les Diagnostics `REFERENCE_*` SHALL être émis au chemin d'instance de cette valeur

### Requirement: Validation des références — ressource contenue absente (REFERENCE_002)

Le système SHALL émettre `REFERENCE_002` (erreur) lorsqu'une référence `#id` ne correspond à aucun `id` dans `contained`, ou lorsque la référence est `#` seule hors d'une ressource contenue (ref-1 n'autorise `#` que si `%rootResource != %resource`). Une ressource contenue SHALL résoudre `#id` contre les `id` contenus de son conteneur, et `#` seul y SHALL être accepté. Une entrée de Bundle ou une ressource de Parameters est sa propre racine : `#id` SHALL être résolu contre son propre `contained` et `#` seul SHALL produire `REFERENCE_002`. `#id` SHALL être résolu quelle que soit la profondeur de la référence. En XML, un `<contained>` unique SHALL être accepté non encapsulé ; l'encapsulation `<contained><X>` étant dépliée au remodelage, aucune lecture XML spécifique des `id` contenus n'existe. ref-1 et dom-3 SHALL être évaluées (voir l'évaluation des invariants avec environnement). L'invariant racine dom-3 SHALL aussi compter les références du narratif : une ressource contenue dont l'`id` est nommé par `#id` dans un `a/@href` ou `img/@src` d'un narratif xhtml de la ressource (éléments xhtml préfixés inclus, entités décodées) SHALL satisfaire dom-3 ; le texte, les autres attributs et les URL externes ne comptent pas, et un balisage mal formé interrompt l'analyse. Pour dom-3 seulement, le contexte d'évaluation SHALL écarter ces entrées contenues alors que `%resource` reste la ressource entière, de sorte que les références faites par ces entrées comptent encore (miroir du remplacement manuel de dom-3 du validateur Java ; R5 references.html §2.1.3.0.10, narrative.html §2.1.4.0.3).

#### Scenario: Contained introuvable

- **WHEN** `reference` vaut `#p1` et `contained` ne contient aucune ressource d'`id` `p1`
- **THEN** un Diagnostic `REFERENCE_002` SHALL être émis

#### Scenario: Contained présent

- **WHEN** `contained` contient une ressource d'`id` `p1`
- **THEN** aucun Diagnostic SHALL être émis

#### Scenario: Dièse seul à la racine

- **WHEN** `reference` vaut `#` dans la ressource racine
- **THEN** un Diagnostic `REFERENCE_002` SHALL être émis

#### Scenario: Dièse seul dans une ressource contenue

- **WHEN** `reference` vaut `#` dans `Patient.contained[0]`
- **THEN** aucun Diagnostic `REFERENCE_002` SHALL être émis

#### Scenario: Dièse seul dans une entrée de Bundle

- **WHEN** `reference` vaut `#` dans `Bundle.entry[0].resource`
- **THEN** un Diagnostic `REFERENCE_002` SHALL être émis

#### Scenario: Contained résolu depuis un nœud imbriqué

- **WHEN** une `Reference` imbriquée vaut `#p1` et la racine contient `p1`
- **THEN** aucun Diagnostic `REFERENCE_002` SHALL être émis

#### Scenario: Contained XML unique

- **WHEN** un seul élément `contained` XML contenant une ressource d'`id` `p1` est présent
- **THEN** `#p1` SHALL être résolu

### Requirement: Validation de profil — profil inconnu (PROFILE_001)

Le système SHALL émettre un Diagnostic de sévérité warning `PROFILE_001` pour chaque URL de `meta.profile` inconnue du Registry (suffixe `|version` séparé de l'URL pour la résolution ; une référence avec `#fragment` — `url|version#id` désigne la ressource contenue `id` de cette version, que le Registry n'indexe pas — SHALL NOT être résolue et produit `PROFILE_001`, « not known »), avec le `path` `<Type>.meta.profile[i]`. Une URL nommant une StructureDefinition fournie sans snapshot (voir `PROFILE_010`) SHALL produire `PROFILE_010` à la place de `PROFILE_001`.

#### Scenario: Profil inconnu

- **WHEN** `meta.profile` contient une URL absente du Registry
- **THEN** un Diagnostic `PROFILE_001` de sévérité warning SHALL être émis

#### Scenario: Référence avec fragment

- **WHEN** `meta.profile` contient `http://hl7.org/fhir/StructureDefinition/vitalsigns|5.0.0#vs`
- **THEN** un `PROFILE_001` de sévérité warning SHALL être émis, et aucun `PROFILE_003`

### Requirement: Validation de profil — profil sans snapshot (PROFILE_010)

Une StructureDefinition fournie via `Registry::with_extra` sans snapshot SHALL être tenue hors du Registry et consignée dans `Registry::needs_snapshot`. Là où `PROFILE_001` serait émis pour l'URL d'un tel profil, le système SHALL émettre un Diagnostic de sévérité error `PROFILE_010` (message `Profile '<url>' has no snapshot; snapshot required`) : pour `meta.profile` au `path` `<Type>.meta.profile[i]` (canonique sans `|version`), pour `type.profile` au nœud, et pour un discriminateur `profile` d'un découpage, une fois par URL au chemin de l'élément découpé, après l'affectation. Une référence versionnée (`url|version`) à un tel profil SHALL donner `PROFILE_010` sur tous ces chemins.

#### Scenario: meta.profile sans snapshot

- **WHEN** `meta.profile[0]` nomme un profil fourni sans snapshot
- **THEN** un Diagnostic `PROFILE_010` de sévérité error SHALL être émis à `<Type>.meta.profile[0]` et aucun `PROFILE_001`

#### Scenario: URL toujours inconnue

- **WHEN** `meta.profile` contient une URL ni dans le Registry ni dans `needs_snapshot`
- **THEN** un `PROFILE_001` SHALL être émis, et aucun `PROFILE_010`

#### Scenario: Profil de slice sans snapshot

- **WHEN** un discriminateur `profile` référence un profil de slice nécessitant un snapshot
- **THEN** un seul `PROFILE_010` par URL SHALL être émis au chemin de l'élément découpé, et un `SLICING_SKIPPED` (information) SHALL l'être aussi car le découpage ne peut être évalué

#### Scenario: Référence versionnée sans snapshot

- **WHEN** `meta.profile`, `type.profile` ou un profil de slice nomme `url|version` d'un profil fourni sans snapshot
- **THEN** un `PROFILE_010` SHALL être émis au chemin correspondant

### Requirement: Validation de profil — version différente (PROFILE_003)

Une référence canonique `url|version` SHALL être séparée en l'URL (clé du Registry) et la version. Le système SHALL émettre un Diagnostic de sévérité warning `PROFILE_003` lorsqu'un profil connu est référencé avec une `version` qui diffère de celle de la StructureDefinition chargée (ou que celle-ci n'en a pas), pour : `meta.profile` au `path` `<Type>.meta.profile[i]` ; `ElementDefinition.type.profile`, au chemin d'instance de la valeur ; les profils de slice d'un discriminateur `profile`, une fois par découpage et par référence distincte, au chemin de l'élément découpé. Le message conserve la référence telle qu'écrite. La validation SHALL ensuite se poursuivre contre la version chargée (décision utilisateur V10 ; la spécification FHIR est silencieuse). Une référence avec `#fragment` n'est pas résolue (R5 references.html, « Canonical URLs ») et ne produit pas `PROFILE_003`. Aucun `PROFILE_003` SHALL être émis sans `|version` ou lorsque la version correspond.

#### Scenario: Version de profil différente

- **WHEN** une Observation a `meta.profile` = `http://hl7.org/fhir/StructureDefinition/vitalsigns|1.0.0` et que la version chargée diffère
- **THEN** un Diagnostic `PROFILE_003` de sévérité warning SHALL être émis, et les contrôles de cardinalité de vitalsigns SHALL s'appliquer (ex. `CARDINALITY_001` à `Observation.subject`)

#### Scenario: Version différente sur type.profile

- **WHEN** `type.profile` d'un élément nomme `url|version` avec une version différente de celle chargée
- **THEN** un `PROFILE_003` SHALL être émis au chemin d'instance de la valeur et le profil chargé SHALL s'appliquer

#### Scenario: Version différente sur un profil de slice

- **WHEN** un discriminateur `profile` référence plusieurs fois le même `url|version` de version différente
- **THEN** un seul `PROFILE_003` SHALL être émis, au chemin de l'élément découpé

#### Scenario: Sans version ou version identique

- **WHEN** le profil n'a pas de `|version`, ou sa version égale celle chargée
- **THEN** aucun `PROFILE_003` SHALL être émis

### Requirement: Validation de profil — type incompatible (PROFILE_002) et cardinalité du profil

Le système SHALL émettre `PROFILE_002` (erreur) lorsqu'un profil connu de `meta.profile` s'applique à un autre type de ressource que celui de la ressource, sauf si le type racine du profil (chemin du premier élément du snapshot) est le type de la ressource ou l'un de ses ancêtres (`Registry::type_ancestors`, via `baseDefinition` : ex. `DomainResource` et `Resource` pour `Patient`), que le profil soit `abstract` ou non. Un profil sur un type ancêtre est parcouru depuis son propre chemin racine (ex. `DomainResource.text` `min` 1 donne `CARDINALITY_001` à `Patient.text`). Pour un profil connu compatible et distinct de la définition de base, les règles de cardinalité du profil SHALL être appliquées ; les constats déjà rapportés par la validation de base (même code et même `path`) SHALL être dédupliqués ; la déduplication ne compare qu'aux diagnostics de la même ressource (son parcours de base), jamais à ceux des autres entrées d'un Bundle. Pour un élément de choix `[x]` d'un profil de `meta.profile` (ou `type.profile`), à tous les niveaux, une variante présente dont le type est permis par la base mais absent des types de l'élément du profil SHALL produire `STRUCTURE_001` (erreur) au chemin d'instance de la variante (ex. `Observation.valueString`), message `Element '<path>' is not allowed by profile '<url>': type '<type>' is not one of the types of '<element path>'` ; rien n'est émis si l'élément du profil ne déclare aucun type. Les variantes retirées restent des items de slicing. Les profils connus de `meta.profile` SHALL être appliqués à tous les niveaux (cardinalité des enfants, `fixed[x]`, `pattern[x]`, slices par valeur) et pas seulement à la racine ; un `<profile>` XML unique (scalaire après la remise en forme XML) SHALL être accepté comme un tableau d'un élément. Les éléments du snapshot du profil portant un `sliceName` SHALL être ignorés par la vérification de cardinalité ; l'élément répétitif de base reste vérifié, et les enfants d'une slice ne comptent jamais comme enfants de l'élément découpé.

Les contraintes (invariants FHIRPath) des profils de `meta.profile` SHALL aussi être évaluées, à tous les niveaux (racine, ressources `contained`, entrées de Bundle) : celles de la racine du profil sur la ressource, et celles de chaque élément du profil sur chaque item qu'il désigne (variantes de choix `[x]` conservées par le profil, occurrences de tableau, enfants de slice, éléments `contentReference` traités comme backbone). Une contrainte (même clé et même expression) déjà portée par l'élément de base du noyau au même chemin ou par la racine du type de l'item SHALL être ignorée (le parcours de base l'évalue) et un constat de même code et même `path` déjà rapporté SHALL être écarté, de sorte qu'une contrainte héritée ne soit rapportée qu'une fois. Les diagnostics sont `INVARIANT_<key>` (erreur, ou warning si la sévérité de la contrainte est warning) au chemin du nœud, ou `INVARIANT_SKIPPED` (information) si la contrainte n'est pas évaluable ; l'évaluation utilise l'environnement FHIRPath de la ressource (`%resource`, `%rootResource`, `is_subtype`).

#### Scenario: Invariants des contraintes d'un profil de meta.profile

- **WHEN** une Observation déclare `meta.profile` = `http://hl7.org/fhir/StructureDefinition/vitalsigns` et viole les contraintes du profil
- **THEN** `INVARIANT_vs-2` SHALL être émis à `Observation`, `INVARIANT_vs-1` à `Observation.effectiveDateTime` et `INVARIANT_vs-3` à `Observation.component[0]`, chacun une seule fois

#### Scenario: Profil d'une ressource contenue ou d'une entrée de Bundle

- **WHEN** une ressource `contained` ou une entrée de Bundle déclare un profil dont une contrainte est violée
- **THEN** l'`INVARIANT_<key>` SHALL être émis au chemin d'instance de cette ressource embarquée

#### Scenario: Profil d'un autre type

- **WHEN** une ressource `Patient` déclare un profil dont le type est `Observation`
- **THEN** un Diagnostic `PROFILE_002` SHALL être émis

#### Scenario: Profil sur un type ancêtre

- **WHEN** le type racine du profil est un ancêtre du type de la ressource (ex. `DomainResource` pour un `Patient`)
- **THEN** aucun `PROFILE_002` SHALL être émis

#### Scenario: Profil abstract d'un autre type

- **WHEN** un `Patient` déclare un profil `abstract: true` de type `Observation`
- **THEN** `PROFILE_002` SHALL être émis et aucune règle d'`Observation` SHALL être appliquée

#### Scenario: Profil de contraintes non abstract sur DomainResource

- **WHEN** un profil non `abstract` sur `DomainResource` est appliqué à un `Patient`
- **THEN** aucun `PROFILE_002` SHALL être émis et ses règles d'élément SHALL s'appliquer aux chemins du `Patient` (`DomainResource.text` `min` 1 donne `CARDINALITY_001` à `Patient.text`)

#### Scenario: Profil DomainResource sur un Bundle

- **WHEN** un profil sur `DomainResource` est déclaré par un `Bundle` (qui dérive de `Resource`)
- **THEN** `PROFILE_002` SHALL être émis

#### Scenario: Variante de choix retirée par le profil

- **WHEN** un profil limite `Observation.value[x]` à `Quantity` et l'instance porte `valueString`
- **THEN** `STRUCTURE_001` SHALL être émis à `Observation.valueString`, et aucun constat pour `valueQuantity`

#### Scenario: Cardinalité du profil dédupliquée

- **WHEN** un élément requis par le profil est aussi requis par la base et absent
- **THEN** un seul Diagnostic `CARDINALITY_001` SHALL être émis pour ce `path`

#### Scenario: Slice du profil ignorée

- **WHEN** le snapshot du profil contient un élément avec `sliceName` (ex. `min` 1)
- **THEN** aucune cardinalité n'SHALL être vérifiée pour cette slice, l'élément de base restant vérifié

---

### Requirement: Validation de profil — valeur fixe (PROFILE_004) et motif (PROFILE_005)

Le système SHALL émettre `PROFILE_004` (erreur) pour chaque partie d'une occurrence d'un élément dont l'élément de profil porte `fixed[x]` qui n'est pas exactement la valeur fixe (ElementDefinition.fixed[x] : « all values must be an exact match (case and accent sensitive). Missing elements/attributes must also be missing. »). Les objets sont comparés propriété par propriété, récursivement : une propriété de valeur différente ou manquante SHALL produire une erreur à `<chemin de l'occurrence>.<propriété>` avec le message « Value at '<chemin>' must be exactly <JSON attendu>: the <clé fixedX> <JSON fixe> required by '<chemin de l'élément>' » ; une propriété présente dans l'instance mais absente de la valeur fixe SHALL produire une erreur à son chemin (« must be absent »). Les tableaux de même longueur sont comparés élément par élément à `<chemin>[i]` ; un nombre de répétitions différent, ou tout autre écart (valeur primitive, primitive avec `id` ou `extension`), SHALL produire une seule erreur au chemin courant. Les nombres sont comparés comme décimaux exacts tels qu'écrits, chiffres et échelle : `3` et `3.00` ne correspondent pas à `3.0` (R5 elementdefinition.html : la valeur fixe « SHALL be exactly the value », datatypes.html : « 0.010 is regarded as different to 0.01 » ; le texte du nombre de la définition est conservé par `arbitrary_precision` de serde_json). Un élément absent SHALL NOT produire de constat. Le système SHALL émettre `PROFILE_005` (erreur) lorsqu'une occurrence ne suit pas `pattern[x]` (nombres comparés comme pour `fixed[x]` ; chaque propriété du motif présente et conforme récursivement ; chaque élément de tableau du motif correspond à un élément de l'instance ; propriétés et éléments supplémentaires autorisés). Le `path` SHALL être le chemin d'instance de l'occurrence. En XML, un enfant répétable unique (resté scalaire) SHALL être comparé comme un tableau d'un élément.

#### Scenario: Valeur fixe violée

- **WHEN** un Bundle déclare un profil dont `Bundle.type` est fixé et la valeur diffère
- **THEN** un Diagnostic `PROFILE_004` SHALL être émis à `Bundle.type`

#### Scenario: Valeur fixe sur occurrence imbriquée

- **WHEN** `Observation.referenceRange[0].high` diffère de la valeur fixée par le profil sur `value` et `unit`
- **THEN** un `PROFILE_004` SHALL être émis à `Observation.referenceRange[0].high.value` et un autre à `Observation.referenceRange[0].high.unit`, aucun à `Observation.referenceRange[0].high`

#### Scenario: Propriété en trop ou répétitions différentes

- **WHEN** l'instance porte une propriété absente de la valeur fixe, ou un nombre de répétitions différent de celui de la valeur fixe
- **THEN** un `PROFILE_004` SHALL être émis à la propriété en trop (« must be absent »), ou une seule erreur au chemin du tableau

#### Scenario: Nombres égaux par valeur

- **WHEN** la valeur fixe est `3` et l'instance porte `3.0` ou `3.00`
- **THEN** aucun `PROFILE_004` SHALL être émis

#### Scenario: Élément absent

- **WHEN** l'élément portant `fixed[x]` est absent
- **THEN** aucun `PROFILE_004` SHALL être émis

#### Scenario: Motif non suivi

- **WHEN** une occurrence ne contient pas une propriété ou un élément de tableau requis par `pattern[x]`
- **THEN** un Diagnostic `PROFILE_005` SHALL être émis au chemin de l'occurrence

#### Scenario: Motif avec extras

- **WHEN** l'occurrence contient en plus des propriétés ou éléments absents du motif
- **THEN** aucun `PROFILE_005` SHALL être émis

#### Scenario: Enfant répétable XML unique

- **WHEN** en XML un enfant répétable unique est comparé à un `fixed[x]`/`pattern[x]` tableau d'un élément
- **THEN** il SHALL être traité comme un tableau d'un élément

---

### Requirement: Validation de profil — découpage (PROFILE_006 à PROFILE_009, SLICING_SKIPPED)

Pour chaque élément découpé, le système SHALL affecter chaque répétition à la première slice dont elle satisfait tous les discriminateurs, les chemins de discriminateur étant évalués avec l'évaluateur FHIRPath sur la valeur typée de l'élément. Les types de discriminateur sont : `value`/`pattern` (un `fixed` exige l'égalité exacte, un `pattern` un sous-ensemble ; sans valeur fixe, toute valeur au chemin convient), `exists` (la cible avec `min>0` doit exister, avec `max=0` doit être absente), `type` (`$this` sur un choix `[x]` utilise le type de la clé ; sinon le type FHIR de la valeur ou son `resourceType` ; correspondance exacte, un type de slice abstrait accepte ses spécialisations) et `profile` (`$this` uniquement ; conforme = aucune erreur issue des règles de ce profil, y compris les contraintes de sa racine et de ses éléments, hors celles du type du noyau que le parcours de base évalue). Les slices d'extension SHALL être appariées par l'URL de leur unique `type.profile`, sur `url`. Pour un élément à choix `[x]`, chaque variante autorisée par la définition de base SHALL constituer un item. La comparaison `fixed`/`pattern` SHALL porter sur les valeurs FHIRPath (`_name` JSON fusionné ; une correspondance exacte rejette une primitive portant `id`/extensions).

Le système SHALL émettre, au chemin d'instance concerné :
- `PROFILE_006` (erreur) : le nombre d'items d'une slice est hors de son `min`/`max` (au chemin de l'élément découpé) ;
- `PROFILE_007` (erreur) : découpage fermé et un item ne correspond à aucune slice (au chemin de l'item) ;
- `PROFILE_008` (erreur) : découpage ordonné et un item correspond à une slice déclarée avant la slice d'un item précédent ;
- `PROFILE_009` (erreur) : slice `openAtEnd` et un item apparaît après un item non apparié ;
- `SLICING_SKIPPED` (information) : un discriminateur ne peut être évalué (chemin fonction tel que `resolve()`, `ofType()`, `extension('url')` ; chemin non analysable ; type non supporté tel que `position` ; slice ne donnant aucune valeur au chemin ; profil inconnu ou nécessitant un snapshot (ce dernier émet en plus un `PROFILE_010` par URL au chemin de l'élément découpé, après l'affectation) ; erreur d'évaluation, y compris un chemin nommant un élément que le type ne déclare pas, la navigation étant stricte ; la raison est alors `cannot evaluate discriminator '<path>': <error>`). Il SHALL être émis seulement s'il existe des items ou une slice avec `min>0`, et le découpage n'est alors pas évalué.

Exception : dans un découpage fermé, une slice `max=0` ne donnant aucune valeur de discriminateur (ex. `patch` du Bundle d'historique R5) SHALL ne correspondre à rien.

Les items appariés SHALL recevoir les `fixed`/`pattern` propres à la slice, les invariants de la slice dont la clé est absente de l'élément découpé et les enfants de la slice ; les constats déjà faits au même code et même chemin SHALL NOT être répétés.

Limites connues : les re-slices comptent comme des slices ; les chemins de discriminateur doivent désigner un élément.

#### Scenario: Slice requise absente

- **WHEN** une Observation vitalsigns n'a aucune `category` correspondant à la catégorie `vital-signs`
- **THEN** un Diagnostic `PROFILE_006` SHALL être émis à `Observation.category`

#### Scenario: Slice au-dessus du maximum

- **WHEN** la catégorie `vital-signs` apparaît deux fois
- **THEN** un Diagnostic `PROFILE_006` SHALL être émis à `Observation.category`

#### Scenario: Item hors slices d'un découpage fermé

- **WHEN** un item ne correspond à aucune slice d'un découpage `closed`
- **THEN** un Diagnostic `PROFILE_007` SHALL être émis au chemin de l'item

#### Scenario: Ordre non respecté

- **WHEN** dans un découpage `ordered`, un item correspond à une slice déclarée avant celle d'un item précédent
- **THEN** un Diagnostic `PROFILE_008` SHALL être émis

#### Scenario: openAtEnd

- **WHEN** un item d'une slice d'un découpage `openAtEnd` suit un item non apparié
- **THEN** un Diagnostic `PROFILE_009` SHALL être émis

#### Scenario: Discriminateur non évaluable

- **WHEN** un discriminateur utilise `resolve()`, un type `position`, un profil inconnu, ou une slice ne donne aucune valeur au chemin, et qu'il existe des items ou une slice avec `min>0`
- **THEN** un Diagnostic `SLICING_SKIPPED` (information) SHALL être émis et aucun `PROFILE_006` à `PROFILE_009` SHALL être émis pour ce découpage

#### Scenario: Slice max=0 sans valeur dans un découpage fermé

- **WHEN** une slice `max=0` ne donne aucun discriminateur dans un découpage fermé
- **THEN** elle SHALL ne correspondre à aucun item

---

#### Scenario: Chemin de discriminateur inconnu

- **WHEN** un discriminateur a pour chemin `systen` sur un `Identifier`
- **THEN** un Diagnostic `SLICING_SKIPPED` (information) SHALL être émis, nommant `systen` (`cannot evaluate discriminator 'systen': ...`), et le découpage SHALL ne pas être évalué

### Requirement: Validation de profil — profils de type (type.profile)

Lorsque `type.profile` d'un élément nomme un profil pour le type du nœud (ex. `Claim.item.quantity` → `SimpleQuantity`), le système SHALL appliquer à ce nœud les invariants racine du profil non déjà portés par le type ou l'élément, ainsi que ses cardinalités, `fixed[x]` et `pattern[x]`. Avec plusieurs profils, la conformité à un seul SHALL suffire ; une référence `url|version` SHALL être séparée en URL et version (`PROFILE_003` si la version diffère) ; un profil inconnu SHALL produire un `PROFILE_001` (warning) au nœud, et un profil fourni sans snapshot un `PROFILE_010` (error) au nœud. Les constats déjà faits par la validation de base (même code et même `path`) SHALL NOT être répétés.

#### Scenario: Invariant du profil de type

- **WHEN** `Claim.item[0].quantity` viole `sqty-1` de `SimpleQuantity`
- **THEN** un Diagnostic `INVARIANT_sqty-1` SHALL être émis

#### Scenario: Cardinalité du profil de type

- **WHEN** `Claim.item[0].quantity` porte un `comparator` interdit par `SimpleQuantity`
- **THEN** un Diagnostic `CARDINALITY_002` SHALL être émis à `Claim.item[0].quantity.comparator`

#### Scenario: Plusieurs profils de type

- **WHEN** le nœud est conforme à l'un des profils listés
- **THEN** aucun constat de profil de type SHALL être émis

#### Scenario: Profil de type inconnu

- **WHEN** un des profils de `type.profile` est inconnu du Registry
- **THEN** un `PROFILE_001` de sévérité warning SHALL être émis au nœud

#### Scenario: Profil de type sans snapshot

- **WHEN** un des profils de `type.profile` est fourni sans snapshot
- **THEN** un `PROFILE_010` de sévérité error SHALL être émis au nœud, à la place de `PROFILE_001`

#### Scenario: Profil de type versionné

- **WHEN** `type.profile` nomme `url|version` d'un profil chargé de version différente
- **THEN** un `PROFILE_003` SHALL être émis et le profil chargé SHALL être appliqué

### Requirement: Exécution des manifestes de test officiels

Le crate `fhir-validator` SHALL fournir un test d'intégration exécutant le manifeste `validator/manifest.json` de `fhir-test-cases` pour les cas R5 autonomes (sans `supporting`, `profile`, `profiles`, `packages`, `logical`) dans les taux principaux, JSON (`parse_json`) et XML (`parse_xml`). Le résultat attendu SHALL provenir de l'entrée `java` : le fichier OperationOutcome qu'elle nomme sous `outcomes/`, un `outcome` en ligne ou `errorCount` ; le nombre d'erreurs attendues est le nombre d'issues `error` + `fatal`. Les cas que le manifeste déclare hors périmètre — clé `supporting5` (définitions supplémentaires non chargées) ou `for-publication` valant `true` ou `"true"` (règles de publication HL7) — SHALL être ignorés, comptés par format dans un compteur distinct « skipped (out of scope) » affiché dans le rapport, et exclus du dénominateur du taux de réussite. Les cas sans résultat attendu (fichier d'outcome nommé absent des fixtures) sont comptés comme ignorés ; un fichier d'outcome nommé présent mais illisible ou qui n'est pas un OperationOutcome SHALL faire échouer le test. Chaque cas est jugé sur « présence d'erreurs » contre « erreurs attendues » ; un échec d'analyse (y compris un fichier non UTF-8) compte comme « présence d'erreurs ». Les taux JSON et XML SHALL être affichés séparément (réussites, fausses erreurs, erreurs manquées, échecs d'analyse) et SHALL être au moins 0,87 pour JSON et 0,88 pour XML (planchers de régression). Le test SHALL échouer si, par format, les décomptes (cas jugés, ignorés sans résultat attendu, hors périmètre) diffèrent exactement de JSON (108, 1, 3) et XML (107, 1, 1) sur les fixtures épinglées (dérive des fixtures ou des filtres). En plus du jugement « présence d'erreurs » (inchangé), le test SHALL comparer, pour chaque cas jugé, le nombre de diagnostics de sévérité Error (un échec d'analyse compte pour 1) au nombre d'erreurs attendues (`errorCount` + `fatalCount`, ou issues `error` + `fatal` de l'`outcome` en ligne ou du fichier `outcomes/`), et afficher par format une table des écarts (cas, format, attendu, obtenu, delta) triée par |delta| décroissant puis par nom de fichier, avec un résumé matched/over/under. Le décompte ne SHALL jamais modifier le verdict réussite/échec d'un cas. Le test SHALL échouer si le nombre de cas concordants descend sous le plancher exact `MIN_ERROR_COUNT_MATCHES` = (78, 79) (JSON, XML), à relever à mesure que les décomptes convergent. Les cas R5 portant l'une des clés `supporting`, `profile`, `profiles`, `packages`, `logical` SHALL aller dans un compteur `supporting` distinct (les deux formats ensemble), jamais dans les taux principaux. Un cas portant `profile` (exécution avec profil explicite, `-profile` de Java) est différé et compté « skipped (explicit profile, deferred) ». Sont comptés « skipped (needs snapshot/package/xml) » : les cas avec `packages` ou `logical`, un fichier `supporting`/`profiles` qui n'est pas `.json` ou est absent, un ensemble fourni rejeté par la surcouche (`Error::Unsupported`, ex. une spécialisation ; `Error::Conflict`, ex. deux versions d'un même canonique) et une StructureDefinition fournie sans snapshot (`needs_snapshot`). Les autres SHALL être validés avec `Registry::with_extra(base, fichiers)` et jugés comme les cas principaux, avec leur propre ligne de rapport et table des écarts de décompte. Le test SHALL échouer si les décomptes (jugés, ignorés sans résultat attendu, hors périmètre, needs snapshot/package/xml, profil explicite différé) diffèrent exactement de (42, 0, 0, 68, 69), ou si le nombre de cas réussis ou de concordances de décompte `supporting` descend sous 38 (planchers sans marge). Le répertoire de fixtures SHALL pouvoir être surchargé par `FHIR_FIXTURES_DIR` ; si le manifeste est absent, le test SHALL être ignoré.

#### Scenario: Taux de réussite sous le plancher

- **WHEN** le taux de réussite JSON est inférieur à 0,87 ou le taux XML inférieur à 0,88
- **THEN** le test SHALL échouer

#### Scenario: Cas hors périmètre

- **WHEN** un cas du manifeste porte une clé `supporting5` ou `for-publication` valant `true` ou `"true"`
- **THEN** il SHALL être compté dans « skipped (out of scope) » et exclu du dénominateur du taux de réussite

#### Scenario: Cas avec définitions supplémentaires

- **WHEN** un cas R5 porte `supporting` ou `profiles` (fichiers `.json` chargeables, sans snapshot manquant)
- **THEN** il SHALL être validé avec `Registry::with_extra`, compté dans le tally `supporting` et non dans les taux JSON/XML principaux ; un cas avec `profile` SHALL être compté « skipped (explicit profile, deferred) » et un cas avec `packages`, `logical`, fichier non `.json`, surcouche rejetée ou snapshot manquant « skipped (needs snapshot/package/xml) »

#### Scenario: Sous-module absent

- **WHEN** le manifeste est introuvable
- **THEN** le test SHALL se terminer sans échec

#### Scenario: Écarts de décompte d'erreurs

- **WHEN** le nombre d'erreurs obtenu d'un cas diffère de l'attendu
- **THEN** le cas SHALL figurer dans la table des écarts du format, sans changer son verdict réussite/échec

#### Scenario: Plancher de concordance des décomptes

- **WHEN** le nombre de cas concordants JSON ou XML est inférieur à son plancher (78 pour JSON, 79 pour XML)
- **THEN** le test SHALL échouer

---

### Requirement: Validation récursive dans les tableaux

Le système SHALL appliquer les vérifications de type à chaque élément d'un tableau JSON, en incluant l'index dans le `path` du Diagnostic.

#### Scenario: Élément de tableau invalide

- **WHEN** un champ de type `boolean[]` contient `[true, "not-a-bool"]`
- **THEN** un Diagnostic TYPE_001 SHALL être émis avec `path` contenant `[1]` (index de l'élément invalide)

---

### Requirement: Validation des éléments imbriqués

Un seul parcours récursif SHALL appliquer ces vérifications, en partant du premier niveau de la ressource ; il s'exécute avant les invariants de la ressource racine et la couche profil. À toute profondeur, un champ `_nom` SHALL être accepté seulement si `nom` résout un élément (ou une variante de choix) de type primitif, sinon `STRUCTURE_001` au chemin de `_nom` (ex. `Patient.name[0]._bogus`). Un `_nom` accepté SHALL être parcouru via la StructureDefinition du type complexe `Element` (contrôle de `id` et `extension`) ; un tableau SHALL être parcouru élément par élément (index dans le `path`, éléments `null` ignorés) ; une valeur non objet, ou un élément de tableau ni objet ni `null`, SHALL produire `TYPE_001` (ex. `"_birthDate": "x"` → `Patient._birthDate`, `"_given": [1]` → `Patient.name[0]._given[0]`). Le champ `_nom` lui-même n'est pas soumis à STRUCTURE_002 ; son contenu l'est pour une entrée JSON. Les champs `extension` et `modifierExtension` SHALL être traités comme tout autre élément de type `Extension`, à toute profondeur : résolus dans le snapshot (STRUCTURE_001 s'ils n'y figurent pas, ex. `Patient.name[0].modifierExtension`), puis parcourus via la StructureDefinition `Extension` (cardinalité de `url`, type de `value[x]`, extensions imbriquées). Un élément `[x]` SHALL être vérifié comme un tout, au premier niveau comme en profondeur : requis sans variante → `CARDINALITY_001` (au chemin `…[x]`), deux variantes → `CARDINALITY_002`, une variante présente soumise à `CARDINALITY_002` selon son `max`. Le système SHALL descendre dans les `BackboneElement` (via le même snapshot) et dans les types complexes (via leur StructureDefinition `complex-type`) et y appliquer les vérifications structurelles (STRUCTURE_001, et STRUCTURE_002 pour les entrées JSON), de cardinalité (CARDINALITY_001/002/003) et de type (TYPE_*), et applique `STRUCTURE_005` (tableau JSON vide) à toute profondeur pour une entrée JSON, avec un `path` d'instance incluant les index (ex. `Patient.name[0].bogus`). Les éléments `[x]` SHALL être résolus par leur suffixe. Les couches terminologie, invariants et références s'exécutent à toute profondeur sur ce même parcours ; seule la couche profil reste limitée au premier niveau . Un élément à `contentReference` (local `#Questionnaire.item` ou absolu `http://hl7.org/fhir/StructureDefinition/Observation#Observation.referenceRange`) SHALL être validé dans le même parcours contre les enfants et règles de l'élément cible (STRUCTURE_001, cardinalité, types, terminologie, références, invariants de la cible, ex. que-6 à `Questionnaire.item[0].item[0].item[0]`), et les valeurs FHIRPath en dessous SHALL être typées via la cible ; la récursion suit les données de l'instance (bornée par `MAX_DEPTH` des parseurs).

Tout élément de type `Resource` (`contained`, `Bundle.entry.resource`, `Bundle.entry.response.outcome`, `Bundle.issues`, `Parameters.parameter.resource`) contient une ressource validée selon son propre type après son conteneur : parcours, invariants racine (ex. dom-6 à `Bundle.entry[0].resource`, org-1 à `Patient.contained[0]`) et couche `meta.profile`, avec des chemins d'instance tels que `Patient.contained[0].bogus` ou `Bundle.entry[0].resource.active`. Son `id` SHALL être soumis à TYPE_005 à `<chemin>.id`. Un élément non objet SHALL produire `TYPE_001` « Expected Resource object » ; un élément `null` est ignoré ; un `resourceType` absent, non chaîne, inconnu ou abstrait SHALL produire `STRUCTURE_000` au chemin de l'élément (seule cette ressource embarquée s'arrête). Ordre : diagnostics du conteneur, puis ressources embarquées dans l'ordre du document. La règle `_nom` s'applique à toute profondeur : un enfant requis présent uniquement sous la forme `_nom` SHALL être considéré présent seulement si son type est primitif et si `_nom` est un objet ou un tableau d'objets et de `null` contenant au moins un objet, et un `_nom` en tableau au-delà du maximum fini SHALL produire `CARDINALITY_002`.

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

#### Scenario: Contenu d'une extension de primitif parcouru

- **WHEN** un Patient contient `"_birthDate": {"bogus": 1}` ou `"_birthDate": {"extension": [{"valueString": "x"}]}`
- **THEN** un Diagnostic STRUCTURE_001 SHALL être émis avec `path` = `Patient._birthDate.bogus`, respectivement CARDINALITY_001 avec `path` = `Patient._birthDate.extension[0].url`

#### Scenario: Extension de primitif de forme invalide

- **WHEN** un Patient contient `"_birthDate": "x"` ou `Patient.name[0]` contient `"_given": [1]`
- **THEN** un Diagnostic TYPE_001 SHALL être émis avec `path` = `Patient._birthDate`, respectivement `Patient.name[0]._given[0]`

#### Scenario: Couches sémantiques à toute profondeur

- **WHEN** un nœud imbriqué viole un invariant, un binding ou une référence
- **THEN** les Diagnostics correspondants SHALL être émis

#### Scenario: contentReference local parcouru

- **WHEN** `Questionnaire.item[0].item[0]` contient un champ `bogus` ou un `item` violant que-6
- **THEN** `STRUCTURE_001` SHALL être émis à `Questionnaire.item[0].item[0].bogus`, respectivement l'invariant que-6 à `Questionnaire.item[0].item[0].item[0]`

#### Scenario: Champ invalide dans une ressource contenue

- **WHEN** `Patient.contained[0]` contient un champ `bogus`
- **THEN** `STRUCTURE_001` SHALL être émis avec `path` = `Patient.contained[0].bogus`

#### Scenario: Entrée de Bundle non objet

- **WHEN** `Bundle.entry[0].resource` est une chaîne
- **THEN** `TYPE_001` « Expected Resource object » SHALL être émis au chemin `Bundle.entry[0].resource`

### Requirement: Nom textuel de Severity

Le système SHALL fournir `Severity::as_str()` retournant le code FHIR R5 IssueSeverity de la sévérité : `"fatal"`, `"error"`, `"warning"`, `"information"`. C'est la représentation exposée par les bindings. `Severity` SHALL implémenter `FromStr` comme inverse exact de `as_str` : toute autre chaîne (`success`, `hint`, casse différente comme `Error`, chaîne vide) SHALL être rejetée.

#### Scenario: Conversion en chaîne

- **WHEN** `as_str()` est appelé sur `Severity::Fatal`, `Error`, `Warning` et `Information`
- **THEN** il SHALL retourner respectivement `"fatal"`, `"error"`, `"warning"` et `"information"`

#### Scenario: Aller-retour FromStr

- **WHEN** `s.as_str().parse::<Severity>()` est appelé pour chaque `s` de `Severity::ALL`
- **THEN** il SHALL retourner `Ok(s)`

#### Scenario: Codes rejetés par FromStr

- **WHEN** `"success"`, `"hint"`, `"Error"` ou `""` est analysé en `Severity`
- **THEN** l'analyse SHALL échouer

### Requirement: Remodelage des ressources XML

Quand `Resource.from_xml` est vrai (positionné par `parse_xml`), `validate` SHALL remodeler la ressource une seule fois en forme JSON avant toute validation : les primitifs `boolean`, `integer`, `unsignedInt`, `positiveInt` et `decimal` sont typés d'après les types d'éléments du Registry, et `id`/`extension` d'un primitif sont déplacés vers `_nom`. La partie `_nom` SHALL elle-même être remodelée comme un `Element`, de sorte que les valeurs primitives de ses extensions soient typées. Un primitif XML sans valeur ni `id`/`extension` (par exemple `<active/>`) SHALL être conservé tel quel afin que `TYPE_001` se déclenche. Un texte mal formé SHALL rester une chaîne afin que `TYPE_001` se déclenche. Le remodelage SHALL suivre les `contentReference` jusqu'à l'élément cible. Un élément de type `Resource` ne contenant qu'un enfant `<X>` (X type de ressource concret) SHALL être déplié en objet de `resourceType` « X » puis remodelé selon la définition de X (primitifs typés, `_nom`) ; un encapsuleur inconnu SHALL être conservé tel quel (donnant `STRUCTURE_000`). Les tableaux SHALL conserver le regroupement XML (frères répétés uniquement).

Le remodelage SHALL vérifier la forme XML de chaque élément d'après `ElementDefinition.xml_attr` : un élément dont la `representation` contient `xmlAttr` (valeur d'un primitif, `Element.id`, `Extension.url`) donné comme élément enfant, et tout autre élément donné comme attribut, SHALL produire un Diagnostic `STRUCTURE_001` au chemin d'instance de cet élément.

Une valeur d'attribut épurée par le parseur (`Resource::trimmed_attributes`) sur un élément connu SHALL produire un Diagnostic `TYPE_007` de sévérité `Warning` au chemin d'instance de l'élément, au lieu d'une erreur de type.

#### Scenario: Primitif XML typé

- **WHEN** `validate` reçoit une ressource XML avec `<active value="true"/>`
- **THEN** `active` SHALL être validé comme booléen sans `TYPE_001`

#### Scenario: Décimal XML selon la regex R5

- **WHEN** le texte d’un `decimal` XML est comparé à la regex R5 du Registry `-?(0|[1-9][0-9]{0,17})(\.[0-9]{1,17})?([eE][+-]?[0-9]{1,9})?` (le `{1,9}}` publié est une coquille, lu `{1,9}`)
- **THEN** `1e09` SHALL être valide ; un texte non conforme SHALL rester une chaîne et donner `TYPE_001`

#### Scenario: Décimal JSON selon la regex R5

- **WHEN** un nombre JSON d'un élément `decimal` est comparé à la même regex, sur son texte source tel qu'écrit (`Resource::number_texts` : texte source de chaque nombre JSON dont la valeur ne se réimprime pas telle qu'écrite, indexé par offset d'octet du nœud), à toute profondeur, ressources contenues comprises
- **THEN** `1E-22` SHALL être valide ; `1000000000000000000` et `1.000000000000000000E-24` SHALL produire `TYPE_003` (erreur) avec le message `Invalid decimal format at '<path>': '<texte écrit>'` (cas officiel `obs-decimal.json`, désormais conforme) ; le XML reste inchangé

#### Scenario: Texte mal formé

- **WHEN** une ressource XML contient `<multipleBirthInteger value="abc"/>`
- **THEN** `validate` SHALL émettre `TYPE_001`

#### Scenario: Extension de primitif XML

- **WHEN** un primitif XML porte un enfant `extension`
- **THEN** l'extension SHALL être validée comme `_nom`

#### Scenario: Primitif typé dans une extension de primitif XML

- **WHEN** `validate` reçoit `<active value="true"><extension url="..."><valueBoolean value="true"/></extension></active>`
- **THEN** aucune erreur SHALL être émise

#### Scenario: Primitif mal formé dans une extension de primitif XML

- **WHEN** `validate` reçoit `<active value="true"><extension url="..."><valueBoolean value="maybe"/></extension></active>`
- **THEN** `validate` SHALL émettre `TYPE_001` au chemin `Patient._active.extension.valueBoolean`

#### Scenario: Primitif XML vide

- **WHEN** `validate` reçoit une ressource XML `Patient` avec `<active/>`
- **THEN** `validate` SHALL émettre `TYPE_001` au chemin `Patient.active`

#### Scenario: Attribut XML donné comme élément

- **WHEN** `validate` reçoit `<active><value value="true"/></active>`
- **THEN** `validate` SHALL émettre `STRUCTURE_001` au chemin `Patient.active.value`
- **WHEN** `validate` reçoit `<gender value="male"><id value="g1"/></gender>`
- **THEN** `validate` SHALL émettre `STRUCTURE_001` au chemin `Patient._gender.id`

#### Scenario: Élément XML donné comme attribut

- **WHEN** `validate` reçoit `<name family="x"/>`
- **THEN** `validate` SHALL émettre `STRUCTURE_001` au chemin `Patient.name.family`

#### Scenario: Valeur d'attribut épurée

- **WHEN** `validate` reçoit `<active value=" true"/>`, un `id` épuré sur `<gender>` ou un `url` épuré sur une extension de `<birthDate>`
- **THEN** `validate` SHALL émettre un `TYPE_007` de sévérité `Warning` au chemin respectif `Patient.active`, `Patient._gender.id` ou `Patient._birthDate.extension.url`, sans `TYPE_001`

#### Scenario: Ressource contenue XML dépliée

- **WHEN** `validate` reçoit un `Patient` XML avec `<contained><Patient><active value="maybe"/></Patient></contained>`
- **THEN** `validate` SHALL émettre `TYPE_001` au chemin `Patient.contained.active` sans `STRUCTURE_000`

#### Scenario: Bundle message XML sans faux bdl-12

- **WHEN** `validate` reçoit un Bundle XML `message` dont la première entrée est un `MessageHeader`
- **THEN** ni `bdl-12` ni `bdl-11` ne SHALL être émis à tort

### Requirement: Validation terminologique — Coding sans system (TERMINOLOGY_002)

Le système SHALL émettre un Diagnostic de sévérité warning `TERMINOLOGY_002` pour chaque `Coding` portant un `code` mais pas de `system`, lorsqu'il appartient à un élément, à toute profondeur (y compris variantes de choix), de type `Coding` ou `CodeableConcept`, lié en `required` ou `extensible` à un ValueSet déclaré. Le `path` SHALL être celui du Coding (ex. `Observation.code.coding[0]`, ou le chemin de l'élément pour un type `Coding`). Un Coding sans `code` SHALL être ignoré.

#### Scenario: Coding lié sans system

- **WHEN** `Observation.code` (lié en `required` ou `extensible`) contient un `coding[0]` avec un `code` et sans `system`
- **THEN** un Diagnostic `TERMINOLOGY_002` de sévérité warning SHALL être émis avec le `path` `Observation.code.coding[0]`

#### Scenario: Coding avec system

- **WHEN** chaque `coding` lié porte un `system`
- **THEN** aucun Diagnostic `TERMINOLOGY_002` SHALL être émis

#### Scenario: Coding sans system imbriqué

- **WHEN** un `Coding` lié sans `system` se trouve dans un élément imbriqué
- **THEN** un Diagnostic `TERMINOLOGY_002` SHALL être émis au chemin d'instance de ce Coding

### Requirement: Validation des références — type déclaré incohérent (REFERENCE_003)

Le système SHALL émettre `REFERENCE_003` (erreur), au `path` `<élément>.reference`, lorsque, à toute profondeur, `Reference.type` (nom de type nu ou `http://hl7.org/fhir/StructureDefinition/<Type>`) diffère du type d'une référence URL FHIR (references.html 2.1.3.0.1). Une URL FHIR est une référence qui correspond entièrement à l'expression régulière de référence littérale R5 `((http|https)://([A-Za-z0-9\-\\\.\:\%\$]*\/)+)?(Type)\/[A-Za-z0-9\-\.]{1,64}(\/_history\/[A-Za-z0-9\-\.]{1,64})?`, où `Type` est un type de ressource concret connu du Registry. Une référence `#id` vers une ressource contenue (racine, ressource contenue au sein de la liste d'ids de son conteneur, entrée de Bundle ; JSON et XML) prend le `resourceType` de cette ressource contenue comme type cible, lorsque c'est un type de ressource concret connu du Registry ; sans `resourceType` ou avec un `resourceType` inconnu, aucun contrôle de type n'est fait. Dans un Bundle, une référence absolue (ex. `urn:uuid:`) égale exactement au `fullUrl` d'une entrée du Bundle le plus interne (un Bundle imbriqué est sa propre portée ; les ressources contenues d'une entrée utilisent le Bundle de l'entrée) prend le `resourceType` de cette entrée comme type cible (bundle.html 2.4.5.1), y compris lorsque ce `fullUrl` est une URL RESTful (`http(s)://.../Type/id`) : le type de l'entrée fait alors autorité sur le type que nomme l'URL ; entrées de même `fullUrl` dont les types diffèrent ou dont l'une n'a ni `resource` ni `resourceType` (fullUrl ambigu), et références sans entrée correspondante, ne sont pas résolues et ne sont jamais signalées. Les références relatives ne sont pas résolues contre les bases de `fullUrl`. Les références conditionnelles, `#` seul, `urn:` sans entrée de Bundle correspondante et les URL absolues non conformes ne sont pas des URL FHIR et SHALL être ignorées par cette règle.

#### Scenario: Type déclaré différent

- **WHEN** `reference` vaut `Patient/1` (ou `http://example.org/fhir/Patient/1`) et `type` vaut `Practitioner`
- **THEN** un Diagnostic `REFERENCE_003` SHALL être émis avec `path` se terminant par `.reference`

#### Scenario: Type déclaré sous forme d'URL canonique

- **WHEN** `reference` vaut `Patient/1` et `type` vaut `http://hl7.org/fhir/StructureDefinition/Patient`
- **THEN** aucun Diagnostic `REFERENCE_003` SHALL être émis

#### Scenario: Référence qui n'est pas une URL FHIR

- **WHEN** `reference` vaut `urn:uuid:...`, `#` ou `Patient?identifier=x` et `type` vaut `Practitioner`
- **THEN** aucun Diagnostic `REFERENCE_003` SHALL être émis

#### Scenario: Référence contenue de type différent

- **WHEN** `reference` vaut `#p1`, `contained` contient une `Patient` d'`id` `p1` et `type` vaut `Practitioner`
- **THEN** un Diagnostic `REFERENCE_003` SHALL être émis avec `path` se terminant par `.reference`

#### Scenario: Référence urn:uuid vers une entrée de Bundle de type différent

- **WHEN** dans un Bundle, `reference` vaut `urn:uuid:x`, `type` vaut `Practitioner` et une entrée a `fullUrl` `urn:uuid:x` et une `Patient`
- **THEN** un Diagnostic `REFERENCE_003` SHALL être émis avec `path` se terminant par `.reference`

#### Scenario: Le type de l'entrée prime sur le type d'une URL RESTful

- **WHEN** dans un Bundle, `reference` vaut `http://acme.com/Patient/p1`, `type` vaut `Basic` et une entrée a ce `fullUrl` et une `Basic`
- **THEN** aucun Diagnostic `REFERENCE_003` SHALL être émis

#### Scenario: fullUrl partagé par une entrée sans resource

- **WHEN** dans un Bundle, `reference` vaut `urn:uuid:x`, `type` vaut `Practitioner` et deux entrées ont `fullUrl` `urn:uuid:x`, l'une avec une `Patient`, l'autre sans `resource` (ou sans `resourceType`)
- **THEN** aucun Diagnostic `REFERENCE_003` SHALL être émis

#### Scenario: Référence urn:uuid sans entrée correspondante

- **WHEN** dans un Bundle, `reference` vaut `urn:uuid:y`, `type` vaut `Practitioner` et aucune entrée n'a ce `fullUrl` (ou deux entrées de types différents le partagent)
- **THEN** aucun Diagnostic `REFERENCE_003` SHALL être émis

### Requirement: Validation des références — référence relative mal formée (REFERENCE_004)

Le système SHALL émettre un Diagnostic de sévérité warning `REFERENCE_004` lorsqu'une référence relative, à toute profondeur, (sans schéma URI (RFC 3986), sans `?`, ne commençant pas par `#`) dont le premier segment est un type de ressource concret connu ne correspond pas à `Type/id` ou `Type/id/_history/version`, avec id et version de la forme `[A-Za-z0-9\-\.]{1,64}`. L'expression régulière n'étant pas énoncée comme un SHALL par la spécification FHIR, la sévérité est warning. Une référence relative à un type inconnu ou abstrait SHALL relever uniquement de `REFERENCE_001`.

#### Scenario: Référence relative mal formée

- **WHEN** `reference` vaut `Organization`, `Practitioner/a b`, `Practitioner/a:b` ou `Practitioner/1/_history`
- **THEN** un Diagnostic `REFERENCE_004` de sévérité warning SHALL être émis avec `path` se terminant par `.reference`

#### Scenario: Référence relative bien formée

- **WHEN** `reference` vaut `Practitioner/1` ou `Practitioner/1/_history/2`
- **THEN** aucun Diagnostic `REFERENCE_004` SHALL être émis

#### Scenario: Type inconnu

- **WHEN** `reference` vaut `Foo`
- **THEN** seul `REFERENCE_001` SHALL être émis, sans `REFERENCE_004`

### Requirement: Validation des références — type cible non autorisé (REFERENCE_005)

Le système SHALL émettre `REFERENCE_005` (erreur), au `path` `<élément>.reference`, lorsque, à toute profondeur, une référence URL FHIR (même définition par expression régulière de référence littérale R5 que `REFERENCE_003` : `Type/id` relatif ou absolu http(s), `_history` optionnel) désigne un type de ressource non autorisé par la liste `targetProfile` du `Reference` de l'élément. Chaque `targetProfile` compte pour le type nommé par le chemin racine de son snapshot (un profil comme `vitalsigns` compte pour `Observation`). Un `targetProfile` autorise son type racine ET tout type qui en dérive (chaîne `baseDefinition`, via `Registry::type_ancestors`) : `Resource` autorise toute ressource, `DomainResource` toute ressource de domaine. Aucun `targetProfile`, ou un `targetProfile` que le Registry ne résout pas (cible inconnue ; tout `targetProfile` du noyau R5 se résout), SHALL autoriser tout type. Une référence `#id` désignant une ressource contenue est contrôlée avec le `resourceType` de celle-ci comme type cible (même portée que `REFERENCE_003`) ; sans `resourceType` ou avec un `resourceType` inconnu, aucun contrôle. Dans un Bundle, une référence absolue (ex. `urn:uuid:`) égale exactement au `fullUrl` d'une entrée du Bundle le plus interne (un Bundle imbriqué est sa propre portée ; les ressources contenues d'une entrée utilisent le Bundle de l'entrée) prend le `resourceType` de cette entrée comme type cible (bundle.html 2.4.5.1), y compris lorsque ce `fullUrl` est une URL RESTful (`http(s)://.../Type/id`) : le type de l'entrée fait alors autorité sur le type que nomme l'URL ; entrées de même `fullUrl` dont les types diffèrent ou dont l'une n'a ni `resource` ni `resourceType` (fullUrl ambigu), et références sans entrée correspondante, ne sont pas résolues et ne sont jamais signalées. Les références relatives ne sont pas résolues contre les bases de `fullUrl`. `#` seul, `urn:` sans entrée correspondante, les URL absolues non FHIR, conditionnelles et logiques (`identifier` seul) NE SONT PAS contrôlées en type.

#### Scenario: Type cible interdit

- **WHEN** `Patient.managingOrganization.reference` vaut `Practitioner/2`
- **THEN** un Diagnostic d'erreur `REFERENCE_005` SHALL être émis

#### Scenario: Type cible autorisé

- **WHEN** `Patient.managingOrganization.reference` vaut `Organization/2`
- **THEN** aucun Diagnostic `REFERENCE_005` SHALL être émis

#### Scenario: Référence imbriquée

- **WHEN** `Patient.contact[0].organization.reference` vaut `Patient/1`
- **THEN** un Diagnostic `REFERENCE_005` SHALL être émis avec `path` = `Patient.contact[0].organization.reference`

#### Scenario: Référence dans un datatype

- **WHEN** `Identifier.assigner` référence un type autre que `Organization`
- **THEN** un Diagnostic `REFERENCE_005` SHALL être émis

#### Scenario: Cible DomainResource

- **WHEN** un `Reference(DomainResource)` référence `Patient/1`
- **THEN** aucun Diagnostic `REFERENCE_005` SHALL être émis

#### Scenario: Référence contenue de type interdit

- **WHEN** `Patient.managingOrganization.reference` vaut `#p1` et `contained` contient une `Practitioner` d'`id` `p1`
- **THEN** un Diagnostic d'erreur `REFERENCE_005` SHALL être émis

#### Scenario: Cible Resource

- **WHEN** `Provenance.target` (`Reference(Resource)`) référence un type quelconque
- **THEN** aucun Diagnostic `REFERENCE_005` SHALL être émis

#### Scenario: Référence urn:uuid vers une entrée de Bundle de type interdit

- **WHEN** dans un Bundle, `Patient.managingOrganization.reference` vaut `urn:uuid:x` et une entrée a `fullUrl` `urn:uuid:x` et une `Practitioner`
- **THEN** un Diagnostic d'erreur `REFERENCE_005` SHALL être émis

#### Scenario: Entrée de Bundle à fullUrl RESTful de type autorisé

- **WHEN** dans un Bundle, `Patient.managingOrganization.reference` vaut `http://acme.com/Practitioner/o1` et une entrée a ce `fullUrl` et une `Organization`
- **THEN** aucun Diagnostic `REFERENCE_005` SHALL être émis

#### Scenario: fullUrl partagé par une entrée sans resource (REFERENCE_005)

- **WHEN** dans un Bundle, `Patient.managingOrganization.reference` vaut `urn:uuid:x` et deux entrées ont ce `fullUrl`, l'une avec une `Practitioner`, l'autre sans `resource` (ou sans `resourceType`)
- **THEN** aucun Diagnostic `REFERENCE_005` SHALL être émis

### Requirement: Validation des références — id dupliqué entre ressources contenues (REFERENCE_006)

Le système SHALL émettre `REFERENCE_006` (erreur), au `path` `<ressource>.contained[i]`, pour chaque ressource contenue dont l'`id` est déjà utilisé par une ressource contenue précédente du même conteneur (deuxième occurrence et suivantes), avec le message `Duplicate id for contained resource: '<id>'`. Les `id` sont comparés quel que soit le `resourceType`. Une ressource contenue sans `id` de type chaîne est ignorée. Le contrôle s'applique une fois par ressource conteneur (ressource de premier niveau, chaque entrée de Bundle et chaque ressource embarquée) ; une ressource contenue partage l'espace d'ids de son conteneur et n'est pas recontrôlée. Base normative R5 (narrative.html 2.1.4.0.4 : « The id attribute SHALL have a unique value within the resource ... Contained resources are included in the id uniqueness scope of the resource that contains them » ; references.html#contained). Une référence `#id` vers un id dupliqué SHALL continuer de se résoudre vers la première ressource correspondante. Hors périmètre : l'unicité de tous les attributs `id` d'éléments, et l'`id` contenu face à l'id logique du conteneur.

#### Scenario: Deux ressources contenues de même id

- **WHEN** `contained` contient deux ressources d'`id` `a`
- **THEN** un Diagnostic d'erreur `REFERENCE_006` SHALL être émis avec `path` = `<ressource>.contained[1]`

#### Scenario: Même id, resourceType différent

- **WHEN** `contained` contient une `Organization` et une `Practitioner` d'`id` `a`
- **THEN** un Diagnostic `REFERENCE_006` SHALL être émis pour la seconde

#### Scenario: Ids distincts ou absents

- **WHEN** les ressources contenues ont des `id` distincts, ou n'ont pas d'`id`
- **THEN** aucun Diagnostic `REFERENCE_006` SHALL être émis

#### Scenario: Entrée de Bundle

- **WHEN** une ressource d'une entrée de Bundle contient deux ressources contenues de même `id`
- **THEN** un Diagnostic `REFERENCE_006` SHALL être émis, une seule fois par doublon

### Requirement: Validation de cardinalité — tableau vide d'un élément optionnel (CARDINALITY_003)

Pour un élément déclaré avec `min = 0` dont la valeur JSON est un tableau vide (`[]`) ou un tableau ne contenant que des `null`, le système SHALL émettre un `Diagnostic` avec code `CARDINALITY_003` et severity `Warning`, à toute profondeur, avec `path` = chemin d'instance de l'élément, `location` = span du tableau, et message `Element '<path>' is an array with no values and should be omitted` (R5 json.html : les objets ne sont jamais vides, un tableau de seuls `null` SHOULD être omis). Lorsque le `_nom` d'un élément primitif est un objet ou un tableau contenant au moins un objet, l'élément SHALL être considéré présent et aucun `CARDINALITY_003` ne SHALL être émis. Les variantes `[x]` ne SHALL PAS être soumises à cette règle. Coexistence voulue : un élément de `max="1"` fourni sous la forme `[]` reçoit aussi `STRUCTURE_002` en plus de `CARDINALITY_001` ou `CARDINALITY_003` ; lorsqu'un profil rend requis (`min >= 1`) un élément optionnel de la base, un `[]` reçoit le `CARDINALITY_003` de la base et le `CARDINALITY_001` du profil (la déduplication code + `path` ne fusionne pas des codes différents). Un `[]` JSON reçoit en outre `STRUCTURE_005` (Error), les tableaux de seuls `null` n'en recevant pas.

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

### Requirement: Validation de type — valeur d'attribut XML épurée (TYPE_007)

Pour une ressource XML, chaque valeur d'attribut épurée par le parseur (`Resource::trimmed_attributes`) sur un élément connu SHALL produire un `Diagnostic` avec code `TYPE_007` et severity `Warning`. Le `path` SHALL être le chemin d'instance de l'élément (ex. `Patient.active`, `Patient._gender.id`, `Patient._birthDate.extension.url`). Les blancs retirés SHALL NOT produire d'erreur de type (`TYPE_001`). Ce code est distinct de `TYPE_006`, réservé aux caractères de contrôle. L'`id` de la ressource racine (`<id value=" p1 "/>`, remonté par le parseur dans `Resource::id`) est aussi couvert : le `Diagnostic` a le `path` `<Type>.id` et la localisation de l'élément `<id>`.

#### Scenario: Valeur primitive épurée

- **WHEN** `validate` reçoit une ressource XML `Patient` avec `<active value=" true"/>`
- **THEN** un Diagnostic `TYPE_007` de severity `Warning` SHALL être émis au chemin `Patient.active`, sans `TYPE_001` ni `TYPE_006`

#### Scenario: id et url épurés

- **WHEN** `validate` reçoit un `id` épuré sur `<gender>` ou un `url` épuré sur une extension de `<birthDate>`
- **THEN** un `TYPE_007` de severity `Warning` SHALL être émis au chemin respectif `Patient._gender.id` ou `Patient._birthDate.extension.url`

#### Scenario: id de la ressource racine épuré

- **WHEN** `validate` reçoit un `Patient` XML avec `<id value=" p1 "/>`
- **THEN** un `TYPE_007` de severity `Warning` SHALL être émis au chemin `Patient.id` avec la localisation de l'élément `<id>`

### Requirement: Validation de type — whitespace seul dans une chaîne (TYPE_008)

Le système SHALL émettre un `Diagnostic` avec code `TYPE_008` et severity `Warning` lorsqu'une valeur non vide de type `string`, `markdown` ou `base64Binary` est composée uniquement de whitespace et satisfait la regex R5 de son type (décision utilisateur PLAN 11.7-A4 révisée ; R5 datatypes.html : « SHOULD always contain non-whitespace content »). Le message SHALL être `<type> at '<path>' should contain non-whitespace content` et la localisation SHALL être celle de la valeur. `xhtml` et les types complexes ou inconnus SHALL NOT produire `TYPE_008`. `maxLength` (`TYPE_002`) SHALL être vérifié en premier : une chaîne whitespace seul trop longue produit `TYPE_002` uniquement. Les cas officiels `primitive-good-ws.json`, `primitive-good-ws.xml` et `params-ws.json` SHALL être sans erreur.

#### Scenario: string whitespace seul

- **WHEN** un champ `string` contient `"  "`
- **THEN** un `TYPE_008` de severity `Warning` SHALL être émis avec le message `string at '<path>' should contain non-whitespace content` et la localisation de la valeur

#### Scenario: Chaîne vide

- **WHEN** un champ `string` contient `""`
- **THEN** un `TYPE_003` SHALL être émis et aucun `TYPE_008`

#### Scenario: xhtml et types complexes

- **WHEN** une valeur `xhtml` ou d'un type complexe ne contient que du whitespace
- **THEN** aucun `TYPE_008` ne SHALL être émis

#### Scenario: maxLength dépassé

- **WHEN** une chaîne whitespace seul dépasse `maxLength`
- **THEN** seul un `TYPE_002` SHALL être émis

#### Scenario: Fixtures officielles

- **WHEN** `primitive-good-ws.json`, `primitive-good-ws.xml` et `params-ws.json` sont validés
- **THEN** aucune erreur ne SHALL être émise

### Requirement: Validation structurelle — namespace schema instance XML (STRUCTURE_003)

Pour une ressource XML, le système SHALL émettre, pour chaque élément enregistré dans `Resource::schema_instance`, un `Diagnostic` avec code `STRUCTURE_003`, severity `Error`, `path` = type de la ressource, localisation = span de l'élément, et message `The XML schema instance namespace is not allowed in FHIR resources` (xml.html 2.1.6.3.4). Une ressource JSON SHALL NOT produire ce diagnostic.

#### Scenario: xsi:schemaLocation sur la racine

- **WHEN** `validate` reçoit un `Patient` XML dont la racine déclare `xmlns:xsi` et porte `xsi:schemaLocation`
- **THEN** exactement un Diagnostic `STRUCTURE_003` de severity `Error` SHALL être émis avec `path` = `Patient`, la localisation de la racine et le message `The XML schema instance namespace is not allowed in FHIR resources`

#### Scenario: Plusieurs éléments concernés

- **WHEN** deux éléments distincts de la ressource déclarent le namespace XMLSchema-instance
- **THEN** deux Diagnostics `STRUCTURE_003` SHALL être émis, chacun à la localisation de son élément

### Requirement: Validation structurelle — ordre des éléments XML (STRUCTURE_004)

Pour une ressource XML uniquement, le système SHALL émettre un `Diagnostic` avec code `STRUCTURE_004` et severity `Error` pour chaque élément enfant écrit après un frère que le snapshot de la StructureDefinition de base liste plus tard (xml.html : « Elements must always appear in the order documented »). Le `path` SHALL être le chemin d'instance de l'élément fautif et la localisation sa position dans la source. Les frères SHALL être comparés par offset du span source (le parseur regroupe les frères répétés en tableaux). Une variante de choix `[x]` prend la position de son élément `[x]`. Tout frère situé après un frère listé plus tard SHALL être signalé (`gender, active, name` : `active` et `name`). La règle s'applique à toute profondeur (éléments backbone, types complexes, enfants d'extension de primitif, ressources contenues ou embarquées). Les attributs n'ont pas d'ordre : un élément `xmlAttr` écrit comme élément enfant reçoit seulement `STRUCTURE_001`, sans `STRUCTURE_004`. L'`<id>` simple de la racine, remonté par le parseur dans `Resource.id`, SHALL faire partie du contrôle d'ordre racine avec le span de son élément. Les positions du snapshot SHALL venir de l'index de chemins de la StructureDefinition (`StructureDefinition::position`). Une ressource JSON SHALL NOT produire ce diagnostic.

#### Scenario: Éléments dans le mauvais ordre

- **WHEN** `validate` reçoit un `Patient` XML avec `<gender/>` puis `<active/>`
- **THEN** un Diagnostic `STRUCTURE_004` SHALL être émis au chemin `Patient.active` avec sa localisation

#### Scenario: Frère répété intercalé

- **WHEN** un `Patient` XML contient `<name>` avec `<given/><family/><given/>`
- **THEN** un seul `STRUCTURE_004` SHALL être émis, au chemin `Patient.name.family`

#### Scenario: Id racine hors ordre

- **WHEN** `validate` reçoit `<Patient><active value="true"/><id value="p"/></Patient>`
- **THEN** un Diagnostic `STRUCTURE_004` SHALL être émis au chemin `Patient.id`, localisé sur l'élément `<id>`

#### Scenario: Ordre correct

- **WHEN** un `Patient` XML a ses éléments dans l'ordre du snapshot
- **THEN** aucun `STRUCTURE_004` ne SHALL être émis

#### Scenario: Attribut écrit comme élément

- **WHEN** un élément `xmlAttr` est écrit comme élément enfant
- **THEN** seul un `STRUCTURE_001` SHALL être émis, sans `STRUCTURE_004`

### Requirement: Validation — objets et éléments jamais vides

Un élément complexe vide (`<name/>` en XML, `"name": [{}]` en JSON) SHALL être signalé par l'invariant `ele-1`, à toute profondeur ; une valeur d'attribut vide (`<family value=""/>`) SHALL être signalée par les contrôles de type (`TYPE_003`) (xml.html « Attributes cannot be empty », « FHIR elements are never empty » ; json.html « Objects are never empty »). Chaque nœud SHALL produire ce diagnostic une seule fois, sans contrôle séparé.

#### Scenario: Élément complexe vide

- **WHEN** un `Patient` contient `<name/>` (XML) ou `"name": [{}]` (JSON)
- **THEN** un diagnostic `ele-1` SHALL être émis une seule fois pour ce nœud

#### Scenario: Attribut vide

- **WHEN** un `Patient` XML contient `<family value=""/>` dans un `name`
- **THEN** un `TYPE_003` SHALL être émis une seule fois pour ce nœud

### Requirement: Validation terminologique — system désignant un ValueSet (TERMINOLOGY_003)

Le système SHALL émettre un Diagnostic de sévérité error `TERMINOLOGY_003`, au `path` du `system`, lorsque `Coding.system` (lié ou non, à toute profondeur) ou `ValueSet.compose.include.system` / `ValueSet.compose.exclude.system` est l'URL canonique d'un ValueSet chargé dans le Registry. Une URI inconnue du Registry SHALL NOT être signalée.

#### Scenario: Coding.system est un ValueSet

- **WHEN** un `Coding` (lié ou non, à toute profondeur) a pour `system` l'URL canonique d'un ValueSet du Registry
- **THEN** un Diagnostic d'erreur `TERMINOLOGY_003` SHALL être émis au `path` du `system`

#### Scenario: compose include/exclude system est un ValueSet

- **WHEN** `ValueSet.compose.include.system` ou `exclude.system` est l'URL canonique d'un ValueSet du Registry
- **THEN** un Diagnostic d'erreur `TERMINOLOGY_003` SHALL être émis au `path` de ce `system`

#### Scenario: system inconnu

- **WHEN** le `system` est une URI absente du Registry
- **THEN** aucun Diagnostic `TERMINOLOGY_003` SHALL être émis

### Requirement: Validation des références — fullUrl d'entrée en désaccord avec l'id (REFERENCE_007)

Le système SHALL émettre `REFERENCE_007` (erreur), au `path` `Bundle.entry[i]`, lorsque le `fullUrl` absolu d'une entrée correspond entièrement à l'expression régulière d'URL RESTful `[base]/Type/id[/_history/v]` et que sa partie `id` diffère de l'`id` de la ressource de l'entrée (Bundle.entry.fullUrl : « The fullUrl SHALL NOT disagree with the id in the resource »). Aucun diagnostic SHALL être émis si la ressource n'a pas d'`id` ou si le `fullUrl` n'est pas RESTful (ex. `urn:uuid:`). La partie type n'est pas comparée (R5 ne nomme que l'id).

#### Scenario: id en désaccord

- **WHEN** une entrée a `fullUrl` `http://example.org/fhir/Patient/1` et une ressource d'`id` `2`
- **THEN** un Diagnostic d'erreur `REFERENCE_007` SHALL être émis avec `path` = `Bundle.entry[0]`

#### Scenario: id concordant, absent ou fullUrl non RESTful

- **WHEN** l'`id` concorde, la ressource n'a pas d'`id`, ou le `fullUrl` vaut `urn:uuid:...`
- **THEN** aucun Diagnostic `REFERENCE_007` SHALL être émis

### Requirement: Validation des pièces jointes — size et hash (ATTACHMENT_001, ATTACHMENT_002)

Pour tout objet Attachment, à toute profondeur, lorsque `data` est présent et se décode en base64 (espaces ASCII ignorés, selon base64Binary), le système SHALL le comparer à `size` et `hash`. Il SHALL émettre `ATTACHMENT_001` (erreur, au `path` de l'Attachment) lorsque `size` (chaîne integer64 ou nombre JSON) diffère du nombre d'octets décodés. Il SHALL émettre `ATTACHMENT_002` (erreur, au `path` de l'Attachment) lorsque `hash`, décodé, diffère du SHA-1 des octets décodés ; le message SHALL donner les hashs déclaré et réel en base64. Aucun diagnostic SHALL être émis si `data` est absent (le contenu peut être à `url`), si `data` ou `hash` ne se décode pas, ou si `size` n'est pas un entier : les contrôles de format les signalent.

#### Scenario: size en désaccord

- **WHEN** un Attachment a un `data` de 5 octets décodés et `size` = `10`
- **THEN** un Diagnostic d'erreur `ATTACHMENT_001` SHALL être émis au `path` de l'Attachment

#### Scenario: hash en désaccord

- **WHEN** le `hash` décodé d'un Attachment diffère du SHA-1 des octets de `data`
- **THEN** un Diagnostic d'erreur `ATTACHMENT_002` SHALL être émis, son message donnant les hashs déclaré et réel

#### Scenario: rien à comparer

- **WHEN** `data` est absent, ou `data`/`hash` ne se décode pas, ou `size` n'est pas un entier
- **THEN** aucun Diagnostic `ATTACHMENT_001` ni `ATTACHMENT_002` SHALL être émis

### Requirement: Validation des profils — meaningWhenMissing interdit (DEFINITION_001)

Pour toute StructureDefinition (ressource racine ou intégrée) dont `derivation` vaut `constraint`, le système SHALL émettre une erreur `DEFINITION_001` pour chaque `differential.element[i]` qui renseigne `meaningWhenMissing`, au `path` `<ressource>.differential.element[<i>].meaningWhenMissing`; le message SHALL nommer le chemin de l'élément. Selon ElementDefinition.meaningWhenMissing, une signification implicite ne peut être définie que sur une ressource, un type de données ou une extension, jamais dans un profil. Aucun diagnostic SHALL être émis si `derivation` vaut `specialization` ou est absente, si la définition est une extension (`type` Extension et `baseDefinition`, sans `|version`, égal à l'url de la StructureDefinition Extension du Registry), ni si la contrainte n'a pas de `type` (contrôle des champs obligatoires). Le snapshot SHALL NOT être vérifié. Un profil d'une autre extension SHALL être signalé.

#### Scenario: profil avec meaningWhenMissing

- **WHEN** une StructureDefinition `constraint` a un `differential.element[1]` avec `meaningWhenMissing`
- **THEN** une erreur `DEFINITION_001` SHALL être émise à `<ressource>.differential.element[1].meaningWhenMissing`, son message nommant le chemin de l'élément

#### Scenario: définitions exemptées

- **WHEN** la définition est une `specialization`, sans `derivation`, une extension de base, ou une contrainte sans `type`
- **THEN** aucun Diagnostic `DEFINITION_001` SHALL être émis

#### Scenario: profil d'une autre extension

- **WHEN** une StructureDefinition de type Extension a pour `baseDefinition` une autre extension et `derivation` `constraint`, avec `meaningWhenMissing` dans le différentiel
- **THEN** une erreur `DEFINITION_001` SHALL être émise

### Requirement: Validation des profils — contentReference modifié (DEFINITION_006)

Pour toute StructureDefinition dont `derivation` vaut `constraint` et dont la `baseDefinition` (sans `|version`) figure dans le Registry, le système SHALL émettre une erreur `DEFINITION_006` pour chaque `differential.element[i]` qui renseigne `contentReference` lorsque l'élément de même chemin existe dans le snapshot de base et que le chemin référencé (après le dernier `#`) diffère de celui de l'élément de base (ou que ce dernier n'en a pas), au `path` `<ressource>.differential.element[<i>].contentReference`. Selon ElementDefinition.contentReference : « ContentReferences can only be defined in specializations, not constrained types, and they cannot be changed ». Aucun diagnostic SHALL être émis pour une spécialisation, pour une même cible (forme `#chemin` ou canonique `<url>#chemin`), ni si l'élément de base est introuvable.

#### Scenario: contentReference changé par un profil

- **WHEN** un profil `constraint` renseigne `contentReference` vers un chemin différent de celui de l'élément de base
- **THEN** une erreur `DEFINITION_006` SHALL être émise à `<ressource>.differential.element[<i>].contentReference`

#### Scenario: contentReference inchangé ou hors périmètre

- **WHEN** la cible est la même (nue ou canonique), la définition est une spécialisation, ou l'élément de base est introuvable
- **THEN** aucun Diagnostic `DEFINITION_006` SHALL être émis

### Requirement: Validation des profils — orderMeaning ajouté (DEFINITION_007)

Pour toute StructureDefinition dont `derivation` vaut `constraint` et dont la `baseDefinition` est dans le Registry, le système SHALL émettre une erreur `DEFINITION_007` à `<ressource>.differential.element[<i>].orderMeaning` lorsque l'élément renseigne `orderMeaning` alors que l'élément de base de même chemin n'en a pas (ElementDefinition.orderMeaning : « if absent in the base type, a profile cannot assert meaning »). Aucun diagnostic SHALL être émis si la base porte `orderMeaning` (raffinement, ex. Bundle.entry) ni pour une spécialisation.

#### Scenario: orderMeaning ajouté

- **WHEN** un profil `constraint` renseigne `orderMeaning` sur un élément dont la base n'en a pas
- **THEN** une erreur `DEFINITION_007` SHALL être émise à `<ressource>.differential.element[<i>].orderMeaning`

#### Scenario: orderMeaning raffiné ou spécialisation

- **WHEN** la base porte déjà `orderMeaning`, ou la définition est une spécialisation
- **THEN** aucun Diagnostic `DEFINITION_007` SHALL être émis

### Requirement: Validation des profils — isModifier changé (DEFINITION_008)

Pour tout profil (même périmètre et même exemption que DEFINITION_001 : une nouvelle définition d'extension est exemptée) dont la `baseDefinition` est dans le Registry, le système SHALL émettre une erreur `DEFINITION_008` à `<ressource>.differential.element[<i>].isModifier` lorsque l'élément énonce `isModifier` (booléen JSON, ou « true »/« false » en XML) différent de celui de l'élément de base de même chemin (ElementDefinition.isModifier : « Once set, it cannot be changed in derived profiles (except in the special case of the defining a new extension) »). Aucun diagnostic SHALL être émis si la valeur est égale ni pour une définition d'extension.

#### Scenario: isModifier changé

- **WHEN** un profil énonce `isModifier` à une valeur différente de celle de l'élément de base
- **THEN** une erreur `DEFINITION_008` SHALL être émise à `<ressource>.differential.element[<i>].isModifier`

#### Scenario: isModifier égal ou extension

- **WHEN** la valeur est égale à celle de la base, ou la définition est une nouvelle extension
- **THEN** aucun Diagnostic `DEFINITION_008` SHALL être émis

### Requirement: Validation des profils — cardinalité des tranches (DEFINITION_002)

Pour toute StructureDefinition (profil ou spécialisation) ayant un `differential`, le système SHALL, pour chaque `differential.element[i]` portant `slicing`, déterminer le maximum n de l'élément découpé : son `max` énoncé, sinon le `max` de l'élément de même chemin dans le snapshot de la `baseDefinition` (Registry). Si n est `*` ou mal formé, aucun contrôle SHALL être effectué. Selon R5 (profiling, Slice Cardinality), le système SHALL émettre une erreur `DEFINITION_002` : (a) à `<ressource>.differential.element[<i>].max` pour chaque tranche (id `<id du découpé>:<nom>`, les re-tranches `a/b` exclues) dont le `max` énoncé est `*` ou supérieur à n; (b) à `<ressource>.differential.element[<index du découpé>]` si la somme des `min` énoncés des tranches (absent = 0, addition saturante) dépasse n. Les éléments sans `id` SHALL être ignorés. DEFINITION_001 reste limité aux profils.

#### Scenario: minimums des tranches supérieurs au maximum

- **WHEN** les `min` des tranches de `Observation.component` totalisent 2 et le découpé a `max` 1 (sd-card-slice)
- **THEN** une erreur `DEFINITION_002` SHALL être émise à `<ressource>.differential.element[0]`

#### Scenario: maximum d'une tranche supérieur à celui du découpé

- **WHEN** le découpé a `max` 2 et une tranche a `max` `*`
- **THEN** une erreur `DEFINITION_002` SHALL être émise à `<ressource>.differential.element[1].max`

#### Scenario: maximum hérité de la définition de base

- **WHEN** le découpé `Observation.value[x]` n'énonce pas de `max` (0..1 dans la base) et deux tranches ont `min` 1
- **THEN** une erreur `DEFINITION_002` SHALL être émise à `<ressource>.differential.element[0]`

#### Scenario: cas sans diagnostic

- **WHEN** les tranches respectent le maximum, ou le maximum de base est `*`, ou les éléments n'ont pas d'`id`
- **THEN** aucun Diagnostic `DEFINITION_002` SHALL être émis

#### Scenario: minimums énormes

- **WHEN** les `min` des tranches valent `i64::MAX` et le découpé a `max` 1
- **THEN** une seule erreur `DEFINITION_002` SHALL être émise, sans débordement

### Requirement: Validation des profils — min d'un enfant de tranche (DEFINITION_003)

Pour toute StructureDefinition ayant un `differential`, le système SHALL émettre une erreur `DEFINITION_003` (eld-2, Min <= Max) à `<ressource>.differential.element[<i>].min` lorsqu'un enfant de tranche (id dont un segment antérieur au dernier porte un nom de tranche, le dernier non) énonce un `min` supérieur au `max` énoncé de son homologue non découpé dans le même différentiel (id sans noms de tranche); le message SHALL commencer par « No instance can meet the cardinality rules for <id> ». Les éléments sans `id` SHALL être ignorés.

#### Scenario: min d'un enfant de tranche supérieur au max de l'homologue

- **WHEN** `Observation.component:a.value[x]` a `min` 1 et `Observation.component.value[x]` a `max` 0 (sd-card-child)
- **THEN** une erreur `DEFINITION_003` SHALL être émise à `<ressource>.differential.element[4].min`

#### Scenario: min compatible

- **WHEN** le `min` de l'enfant de tranche ne dépasse pas le `max` de l'homologue
- **THEN** aucun Diagnostic `DEFINITION_003` SHALL être émis

### Requirement: Validation des profils — cible d'un canonical (DEFINITION_004)

Pour toute StructureDefinition dont `derivation` vaut `constraint`, le système SHALL émettre une erreur `DEFINITION_004` à `<ressource>.differential.element[<i>].type[<j>].targetProfile[<k>]` pour chaque `targetProfile` (suffixe de version retiré) d'un type de code `canonical` d'un élément du différentiel qui désigne une définition du Registry de `kind` = `resource`, non abstraite, et qui n'atteint pas `CanonicalResource` par ses extensions `structuredefinition-implements` (`StructureDefinition.implements`) ni par sa chaîne `baseDefinition` (R5 references.html, « Canonical URLs » : seules les ressources canoniques peuvent être la cible d'une référence canonical). Les cibles abstraites (`Resource` = canonical(Any), `CanonicalResource`, `MetadataResource`) SHALL passer. Une définition absente du Registry, sur la cible ou durant la marche, laisse le cas indécis : aucun Diagnostic. Les spécialisations (`derivation` = `specialization`) SHALL NOT être contrôlées, la spécialisation R5 NutritionIntake listant `ClinicalUseDefinition`, non canonique, comme cible d'`instantiatesCanonical`.

#### Scenario: Cible canonical non canonique

- **WHEN** un profil (`derivation` = `constraint`) déclare un type `canonical` dont le `targetProfile` est une ressource non canonique du Registry (ex. `Patient`)
- **THEN** une erreur `DEFINITION_004` SHALL être émise à `<ressource>.differential.element[<i>].type[<j>].targetProfile[<k>]`

#### Scenario: Cible canonique ou abstraite

- **WHEN** le `targetProfile` désigne une ressource canonique (ex. `ValueSet`) ou une définition abstraite (ex. `Resource`)
- **THEN** aucun Diagnostic `DEFINITION_004` SHALL être émis

#### Scenario: Cible inconnue ou spécialisation

- **WHEN** le `targetProfile` n'est pas dans le Registry, ou la StructureDefinition est une spécialisation
- **THEN** aucun Diagnostic `DEFINITION_004` SHALL être émis

### Requirement: Validation des profils — contexte d’extension (DEFINITION_005)

Pour une StructureDefinition d’extension, le système SHALL émettre une erreur `DEFINITION_005` à `StructureDefinition.context[<n>].expression` lorsqu’un contexte de type `element` sans `#` ne désigne aucun élément de la spécification de base (ExtensionContextType `element` : « If there is no #, the Element id is one defined in the base specification »). Les contextes `url#id` ne sont pas encore contrôlés.

#### Scenario: Contexte element inconnu

- **WHEN** un contexte `element` sans `#` a pour expression un id absent de la spécification de base
- **THEN** une erreur `DEFINITION_005` SHALL être émise à `StructureDefinition.context[<n>].expression`

#### Scenario: Contexte element connu ou avec dièse

- **WHEN** l’expression désigne un élément de base (ex. `Patient`) ou contient `#`
- **THEN** aucun Diagnostic `DEFINITION_005` SHALL être émis

### Requirement: Validation structurelle — Element.id dupliqué (STRUCTURE_006)

Le système SHALL émettre une erreur `STRUCTURE_006` pour tout `Element.id` dupliqué dans une ressource, ressources contenues comprises (Element.id : « Unique id for the element within a resource » ; narrative.html 2.1.4.0.4). Les `id` de ressource, les ressources embarquées ailleurs (entrées de Bundle, Parameters) et les sous-arbres `ElementDefinition` (portée par snapshot/différentiel selon sdf-16/sdf-17) SHALL être exclus.

#### Scenario: Element.id dupliqué

- **WHEN** deux éléments d’une même ressource portent le même `id`
- **THEN** une erreur `STRUCTURE_006` SHALL être émise

#### Scenario: Exclusions

- **WHEN** l’`id` est répété entre une entrée de Bundle et son Bundle, ou dans deux listes d’`ElementDefinition` distinctes
- **THEN** aucun Diagnostic `STRUCTURE_006` SHALL être émis

### Requirement: Validation structurelle — meta.tag et meta.security dupliqués (STRUCTURE_007)

Le système SHALL émettre une erreur `STRUCTURE_007` pour un couple system+code dupliqué dans `meta.tag` ou `meta.security`, par ressource contenues comprises (Meta : « The list of tags is a set. Uniqueness is based the system/code, and version and display are ignored »). Les codings sans `code` SHALL être ignorés.

#### Scenario: Tag dupliqué

- **WHEN** deux `meta.tag` ont même system et code (version ou display différents)
- **THEN** une erreur `STRUCTURE_007` SHALL être émise

#### Scenario: Coding sans code

- **WHEN** deux codings de `meta.tag` n’ont pas de `code`
- **THEN** aucun Diagnostic `STRUCTURE_007` SHALL être émis

### Requirement: Validation des profils — contentReferenceProfile (DEFINITION_009)

Sur une StructureDefinition, pour chaque élément (différentiel et snapshot) portant l'extension `http://hl7.org/fhir/StructureDefinition/contentReferenceProfile` avec un `valueUri`, le système SHALL émettre une erreur `DEFINITION_009` à `StructureDefinition.<differential|snapshot>.element[i].extension[j]` lorsque la valeur n'est pas de la forme `<canonique>#<id d'élément>` (deux côtés non vides, « not a valid element reference » ; un `|version` vide, p. ex. `url|#id`, est également invalide), ou lorsque le profil visé est introuvable (« could not be found ») ou n'a pas l'élément visé (« does not exist »). Base : « The value refers to an element definition in a profile, not to the profile itself, using the same format as the contentReference (a canonical URL for the profile, followed by '#' and the element id). » Le canonique (`|version` retiré) égal à l'`url` de la définition elle-même SHALL se résoudre contre elle (ids du snapshot et du différentiel, puis éléments de sa base par chemin) ; sinon contre une StructureDefinition du Registry ou de la surcouche, dont l'id SHALL être l'un des éléments. Une valeur du snapshot égale à celle de l'élément du différentiel de même id SHALL n'être signalée qu'une fois (sur le différentiel). Aucun Diagnostic SHALL être émis pour un id de tranche (contenant `:`), une cible chargée sans snapshot (`needs_snapshot`), ni une auto-référence dont la `baseDefinition` est déclarée mais absente du Registry (si la définition ne déclare aucune `baseDefinition`, un id absent de son propre snapshot et différentiel SHALL être signalé).

#### Scenario: Référence mal formée

- **WHEN** la valeur n'a pas de `#`, un côté du `#` est vide, ou le `|version` est vide (`url|#id`)
- **THEN** une erreur `DEFINITION_009` SHALL être émise (« not a valid element reference »)

#### Scenario: Profil ou élément introuvable

- **WHEN** la valeur est bien formée mais le canonique ne nomme ni la définition elle-même ni une StructureDefinition connue, ou l'id n'y existe pas
- **THEN** une erreur `DEFINITION_009` SHALL être émise (« could not be found », resp. « does not exist »)

#### Scenario: Référence résolue ou exemptée

- **WHEN** l'id existe dans la cible, est un id de tranche, ou la cible n'a pas de snapshot
- **THEN** aucun Diagnostic `DEFINITION_009` SHALL être émis

#### Scenario: Doublon snapshot et différentiel

- **WHEN** l'élément du snapshot porte la même valeur que l'élément du différentiel de même id
- **THEN** un seul Diagnostic SHALL être émis, sur le différentiel

