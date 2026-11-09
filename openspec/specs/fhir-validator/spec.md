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

Le système SHALL fournir une enum `Severity` publique avec les variants `Error`, `Warning`, `Information`, `Hint`, ordonnés du plus sévère au moins sévère.

#### Scenario: Ordre des sévérités

- **WHEN** deux valeurs `Severity` sont comparées avec `<`
- **THEN** `Error < Warning < Information < Hint` SHALL être vrai

---

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

### Requirement: Validation de cardinalité — élément requis absent (CARDINALITY_001)

Le système SHALL émettre un `Diagnostic` avec code `CARDINALITY_001` et severity `Error` pour chaque élément déclaré avec `min > 0` dans le snapshot et absent de la ressource. Un élément requis de type primitif (nom de type commençant par une minuscule) présent uniquement sous la forme `_nom` (extension de primitif) SHALL être considéré présent seulement si la valeur de `_nom` est un objet, ou un tableau d'objets et de `null` contenant au moins un objet (un scalaire tel que `"_status": 5`, `[]` ou `[null]` ne compte pas et produit `CARDINALITY_001`) ; pour un type complexe, `_nom` ne SHALL PAS compter comme présence. Lorsque la valeur est absente et que `_nom` est un tableau plus long que le maximum fini, un `CARDINALITY_002` SHALL être émis (path et position de `_nom`). Un nom d'élément contenant `.`, `[` ou `]` (y compris préfixé par `_`) SHALL produire `STRUCTURE_001`. Un choix `[x]` SHALL être vérifié selon le seul type désigné par son suffixe ; sa cardinalité porte sur l'ensemble de ses variantes (champs `base` + type autorisé capitalisé, `_variante` comptant seulement pour un type primitif et sous la même forme objet que `_nom`), à toute profondeur : aucune variante et `min > 0` produit `CARDINALITY_001` au chemin du choix (ex. `Immunization.occurrence[x]`), plus d'une variante produit `CARDINALITY_002` à la seconde variante. Un `integer64` SHALL être une chaîne conforme à `[0]|[-+]?[1-9][0-9]*` et représentable sur 64 bits signés (`TYPE_003` sinon, `TYPE_001` si non chaîne).

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

### Requirement: Validation de type — contrainte de valeur entière (TYPE_002)

Le système SHALL émettre un `Diagnostic` avec code `TYPE_002` et severity `Error` lorsqu'une valeur entière viole une contrainte de valeur : valeur négative pour `unsignedInt`, valeur inférieure ou égale à zéro pour `positiveInt`, ou valeur d'un `integer`, `unsignedInt` ou `positiveInt` hors de la plage des entiers signés 32 bits (maximum `2147483647` ; minimum `-2147483648` pour `integer`).

#### Scenario: unsignedInt négatif

- **WHEN** un champ de type `unsignedInt` contient la valeur JSON `-1`
- **THEN** un Diagnostic `TYPE_002` SHALL être émis

#### Scenario: positiveInt à zéro

- **WHEN** un champ de type `positiveInt` contient la valeur JSON `0`
- **THEN** un Diagnostic `TYPE_002` SHALL être émis

#### Scenario: integer hors de la plage 32 bits

- **WHEN** `valueInteger` contient la valeur JSON `2147483648`
- **THEN** un Diagnostic `TYPE_002` SHALL être émis

---

### Requirement: Validation de type — format date/heure (TYPE_003)

Le système SHALL émettre un `Diagnostic` avec code `TYPE_003` et severity `Error` lorsqu'une valeur de type `date`, `dateTime`, `instant` ou `time` ne respecte pas le format attendu, conforme aux expressions régulières R5 :

- `date` : `YYYY(-MM(-DD)?)?`, année de `0001` à `9999`, jour valide pour le mois (années bissextiles comprises) ;
- `dateTime` : une `date`, ou une date complète suivie de `Thh:mm:ss(.f{1,9})?` et d'un fuseau obligatoire `Z` ou `(+|-)hh:mm` (décalage au plus `14:00`) ; un fuseau sur une valeur réduite à la date SHALL être rejeté ;
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

---

### Requirement: Validation de type — whitespace dans un code (TYPE_004)

Le système SHALL émettre un `Diagnostic` avec code `TYPE_004` et severity `Error` lorsqu'un champ de type `code` contient des espaces ou caractères whitespace.

#### Scenario: Code avec espace

- **WHEN** un champ de type `code` contient la chaîne `"invalid code"`
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

Le système SHALL évaluer les expressions FHIRPath de contrainte définies sur l'élément racine du snapshot. Pour chaque contrainte dont l'expression ne retourne pas `[true]`, il SHALL émettre un `Diagnostic` avec code `INVARIANT_<key>` où `<key>` est la valeur du champ `key` de la contrainte. Le code `INVARIANT_SKIPPED` est réservé : il signale une contrainte non vérifiée et n'est jamais une violation.

#### Scenario: Contrainte violée émet INVARIANT_<key>

- **WHEN** une contrainte avec `key="dom-2"` et `expression="..."` évalue à `[false]`
- **THEN** un Diagnostic avec code `INVARIANT_dom-2` SHALL être émis avec la severity correspondant à la severity de la contrainte

#### Scenario: Contrainte satisfaite n'émet pas de diagnostic

- **WHEN** une contrainte FHIRPath évalue à `[true]`
- **THEN** aucun Diagnostic `INVARIANT_` ne SHALL être émis pour cette contrainte

#### Scenario: Erreur d'évaluation FHIRPath signalée INVARIANT_SKIPPED

- **WHEN** l'évaluation FHIRPath d'une contrainte retourne une erreur
- **THEN** aucun Diagnostic `INVARIANT_<key>` ne SHALL être émis pour cette contrainte
- **AND** exactement un Diagnostic de severity `Information` avec code `INVARIANT_SKIPPED` SHALL être émis, au `path` du type de ressource, avec un message nommant la `key` de la contrainte et la raison de l'erreur

#### Scenario: Expression non analysable signalée INVARIANT_SKIPPED

- **WHEN** l'expression FHIRPath d'une contrainte ne peut pas être analysée (par ex. les variables d'environnement `%` telles que `%resource`, non encore supportées par le parseur, si bien que `dom-3` est toujours ignorée)
- **THEN** la contrainte SHALL être ignorée tant que cette lacune du parseur n'est pas comblée
- **AND** exactement un Diagnostic `Information` `INVARIANT_SKIPPED` nommant la `key` et la raison SHALL être émis, afin qu'un résultat sans erreur ne masque jamais une contrainte non vérifiée

#### Scenario: Invariant dom-6 évalué (identifiant entre backticks)

- **WHEN** une DomainResource R5 n'a pas de `text.div` (contrainte `dom-6`, ``text.`div`.exists()``, severity `warning`)
- **THEN** un Diagnostic `Warning` avec code `INVARIANT_dom-6` SHALL être émis
- **AND** aucun Diagnostic `INVARIANT_SKIPPED` ne SHALL être émis pour `dom-6`

#### Scenario: Severity `information` mappée en Warning

- **WHEN** une contrainte violée a `severity="information"`
- **THEN** le Diagnostic `INVARIANT_<key>` SHALL avoir la severity `Warning`

#### Scenario: Contrainte sans expression ignorée

- **WHEN** une contrainte n'a pas de champ `expression`
- **THEN** aucun Diagnostic ne SHALL être émis pour cette contrainte

---

### Requirement: Validation terminologique — binding required (TERMINOLOGY_001)

Le système SHALL émettre un Diagnostic d'erreur `TERMINOLOGY_001` lorsqu'un élément de premier niveau, à type unique `code`, `Coding` ou `CodeableConcept`, possède un binding de force `required` dont le ValueSet est connu et que le code est prouvé hors du ValueSet. Les éléments imbriqués, les éléments à types multiples et les autres forces de binding SHALL être ignorés. Une appartenance indécidable (`Registry::value_set_contains` retourne `None`) NE DOIT JAMAIS être signalée. Pour un `CodeableConcept`, un seul `coding` appartenant au ValueSet suffit ; l'erreur n'est émise que si tous les `coding` sont prouvés hors du ValueSet.

#### Scenario: Code hors du ValueSet requis

- **WHEN** un `code` lié en `required` à un ValueSet connu contient une valeur absente de ce ValueSet
- **THEN** un Diagnostic `TERMINOLOGY_001` SHALL être émis avec le `path` de l'élément (index inclus pour les tableaux)

#### Scenario: Code appartenant au ValueSet

- **WHEN** le code appartient au ValueSet
- **THEN** aucun Diagnostic terminologique SHALL être émis

#### Scenario: Appartenance indécidable

- **WHEN** le ValueSet est inconnu, utilise des filtres, des exclusions ou un CodeSystem fragmentaire/inconnu
- **THEN** aucun Diagnostic SHALL être émis

#### Scenario: CodeableConcept avec un coding valide

- **WHEN** un `CodeableConcept` contient au moins un `coding` appartenant au ValueSet
- **THEN** aucun Diagnostic SHALL être émis

---

### Requirement: Validation des références — type inconnu (REFERENCE_001)

Le système SHALL émettre `REFERENCE_001` (erreur) lorsque `Reference.reference` d'un élément de premier niveau de type `Reference` est une référence relative dont le type n'est pas un type de ressource concret connu du Registry (type inconnu ou abstrait). Les références conditionnelles (`Patient?identifier=...`) SHALL être acceptées si le type est valide. Les références absolues et `urn:` SHALL être ignorées (non résolues).

#### Scenario: Type de ressource inconnu

- **WHEN** `reference` vaut `Foo/123`
- **THEN** un Diagnostic `REFERENCE_001` SHALL être émis avec `path` se terminant par `.reference`

#### Scenario: Type abstrait

- **WHEN** `reference` vaut `DomainResource/1`
- **THEN** un Diagnostic `REFERENCE_001` SHALL être émis

#### Scenario: Référence conditionnelle, absolue ou urn

- **WHEN** `reference` vaut `Patient?identifier=x`, `http://example.org/Patient/1` ou `urn:uuid:...`
- **THEN** aucun Diagnostic SHALL être émis

---

### Requirement: Validation des références — ressource contenue absente (REFERENCE_002)

Le système SHALL émettre `REFERENCE_002` (erreur) lorsqu'une référence `#id` ne correspond à aucun `id` dans `contained`. La référence `#` seule (la ressource elle-même) SHALL être acceptée.

#### Scenario: Contained introuvable

- **WHEN** `reference` vaut `#p1` et `contained` ne contient aucune ressource d'`id` `p1`
- **THEN** un Diagnostic `REFERENCE_002` SHALL être émis

#### Scenario: Contained présent

- **WHEN** `contained` contient une ressource d'`id` `p1`
- **THEN** aucun Diagnostic SHALL être émis

---

### Requirement: Validation de profil — profil inconnu (PROFILE_001)

Le système SHALL émettre un Diagnostic de sévérité warning `PROFILE_001` pour chaque URL de `meta.profile` inconnue du Registry (suffixe `|version` ignoré pour la résolution), avec le `path` `<Type>.meta.profile[i]`.

#### Scenario: Profil inconnu

- **WHEN** `meta.profile` contient une URL absente du Registry
- **THEN** un Diagnostic `PROFILE_001` de sévérité warning SHALL être émis

---

### Requirement: Validation de profil — type incompatible (PROFILE_002) et cardinalité du profil

Le système SHALL émettre `PROFILE_002` (erreur) lorsqu'un profil connu de `meta.profile` s'applique à un autre type de ressource que celui de la ressource, sauf si le profil porte sur un parent abstrait (ex. `DomainResource`). Pour un profil connu compatible et distinct de la définition de base, les règles de cardinalité du profil SHALL être appliquées ; les constats déjà rapportés par la validation de base (même code et même `path`) SHALL être dédupliqués. Les éléments du snapshot du profil portant un `sliceName` SHALL être ignorés par la vérification de cardinalité (les slices ne sont pas vérifiées) ; l'élément répétitif de base reste vérifié.

#### Scenario: Profil d'un autre type

- **WHEN** une ressource `Patient` déclare un profil dont le type est `Observation`
- **THEN** un Diagnostic `PROFILE_002` SHALL être émis

#### Scenario: Profil sur parent abstrait

- **WHEN** le profil est défini sur un type abstrait (ex. `DomainResource`)
- **THEN** aucun `PROFILE_002` SHALL être émis

#### Scenario: Cardinalité du profil dédupliquée

- **WHEN** un élément requis par le profil est aussi requis par la base et absent
- **THEN** un seul Diagnostic `CARDINALITY_001` SHALL être émis pour ce `path`

#### Scenario: Slice du profil ignorée

- **WHEN** le snapshot du profil contient un élément avec `sliceName` (ex. `min` 1)
- **THEN** aucune cardinalité n'SHALL être vérifiée pour cette slice, l'élément de base restant vérifié

---

### Requirement: Exécution des manifestes de test officiels

Le crate `fhir-validator` SHALL fournir un test d'intégration exécutant le manifeste `validator/manifest.json` de `fhir-test-cases` pour les cas R5 autonomes (sans `supporting`, `profile`, `profiles`, `packages`, `logical`), JSON (`parse_json`) et XML (`parse_xml`). Le résultat attendu SHALL provenir de l'entrée `java` : le fichier OperationOutcome qu'elle nomme sous `outcomes/`, un `outcome` en ligne ou `errorCount` ; le nombre d'erreurs attendues est le nombre d'issues `error` + `fatal`. Les cas sans résultat attendu, ou dont l’attente du manifeste est périmée pour R5 (liste explicite, ex. `list-contained.json`), sont comptés comme ignorés. Chaque cas est jugé sur « présence d'erreurs » contre « erreurs attendues » ; un échec d'analyse (y compris un fichier non UTF-8) compte comme « présence d'erreurs ». Les taux JSON et XML SHALL être affichés séparément (réussites, fausses erreurs, erreurs manquées, échecs d'analyse) et SHALL être au moins 0,66 pour JSON et 0,58 pour XML (planchers de régression). Le répertoire de fixtures SHALL pouvoir être surchargé par `FHIR_FIXTURES_DIR` ; si le manifeste est absent, le test SHALL être ignoré.

#### Scenario: Taux de réussite sous le plancher

- **WHEN** le taux de réussite JSON est inférieur à 0,66 ou le taux XML inférieur à 0,58
- **THEN** le test SHALL échouer

#### Scenario: Sous-module absent

- **WHEN** le manifeste est introuvable
- **THEN** le test SHALL se terminer sans échec

---

### Requirement: Validation récursive dans les tableaux

Le système SHALL appliquer les vérifications de type à chaque élément d'un tableau JSON, en incluant l'index dans le `path` du Diagnostic.

#### Scenario: Élément de tableau invalide

- **WHEN** un champ de type `boolean[]` contient `[true, "not-a-bool"]`
- **THEN** un Diagnostic TYPE_001 SHALL être émis avec `path` contenant `[1]` (index de l'élément invalide)

---

### Requirement: Validation des éléments imbriqués

Un seul parcours récursif SHALL appliquer ces vérifications, en partant du premier niveau de la ressource ; il s'exécute avant les couches terminologie, invariants, références et profil. À toute profondeur, un champ `_nom` SHALL être accepté seulement si `nom` résout un élément (ou une variante de choix) de type primitif, sinon `STRUCTURE_001` au chemin de `_nom` (ex. `Patient.name[0]._bogus`). Un `_nom` accepté SHALL être parcouru via la StructureDefinition du type complexe `Element` (contrôle de `id` et `extension`) ; un tableau SHALL être parcouru élément par élément (index dans le `path`, éléments `null` ignorés) ; une valeur non objet, ou un élément de tableau ni objet ni `null`, SHALL produire `TYPE_001` (ex. `"_birthDate": "x"` → `Patient._birthDate`, `"_given": [1]` → `Patient.name[0]._given[0]`). Les champs `extension` et `modifierExtension` SHALL être traités comme tout autre élément de type `Extension`, à toute profondeur : résolus dans le snapshot (STRUCTURE_001 s'ils n'y figurent pas, ex. `Patient.name[0].modifierExtension`), puis parcourus via la StructureDefinition `Extension` (cardinalité de `url`, type de `value[x]`, extensions imbriquées). Un élément `[x]` SHALL être vérifié comme un tout, au premier niveau comme en profondeur : requis sans variante → `CARDINALITY_001` (au chemin `…[x]`), deux variantes → `CARDINALITY_002`, une variante présente soumise à `CARDINALITY_002` selon son `max`. Le système SHALL descendre dans les `BackboneElement` (via le même snapshot) et dans les types complexes (via leur StructureDefinition `complex-type`) et y appliquer les vérifications structurelles (STRUCTURE_001), de cardinalité (CARDINALITY_001/002) et de type (TYPE_*), avec un `path` d'instance incluant les index (ex. `Patient.name[0].bogus`). Les éléments `[x]` SHALL être résolus par leur suffixe. Les couches terminologie, invariants, références et profil restent limitées au premier niveau ; les `contentReference` (ignorés sans erreur, jamais signalés STRUCTURE_001) et les ressources contenues ne sont pas parcourus. La règle `_nom` s'applique à toute profondeur : un enfant requis présent uniquement sous la forme `_nom` SHALL être considéré présent seulement si son type est primitif et si `_nom` est un objet ou un tableau d'objets et de `null` contenant au moins un objet, et un `_nom` en tableau au-delà du maximum fini SHALL produire `CARDINALITY_002`.

#### Scenario: Choix [x] en tableau au-delà du maximum

- **WHEN** une Observation contient `"valueString": ["a", "b"]` (`value[x]` a `max="1"`)
- **THEN** un Diagnostic CARDINALITY_002 SHALL être émis avec `path` = `Observation.valueString`

#### Scenario: Élément inconnu dans un type complexe

- **WHEN** `Patient.name[0]` contient un champ `bogus`
- **THEN** un Diagnostic STRUCTURE_001 SHALL être émis avec `path` = `Patient.name[0].bogus`

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

### Requirement: Nom textuel de Severity

Le système SHALL fournir `Severity::as_str()` retournant le nom en minuscules de la sévérité : `"error"`, `"warning"`, `"information"`, `"hint"`. C'est la représentation exposée par les bindings.

#### Scenario: Conversion en chaîne

- **WHEN** `as_str()` est appelé sur `Severity::Error`, `Warning`, `Information` et `Hint`
- **THEN** il SHALL retourner respectivement `"error"`, `"warning"`, `"information"` et `"hint"`

### Requirement: Remodelage des ressources XML

Quand `Resource.from_xml` est vrai (positionné par `parse_xml`), `validate` SHALL remodeler la ressource une seule fois en forme JSON avant toute validation : les primitifs `boolean`, `integer`, `unsignedInt`, `positiveInt` et `decimal` sont typés d'après les types d'éléments du Registry, et `id`/`extension` d'un primitif sont déplacés vers `_nom`. La partie `_nom` SHALL elle-même être remodelée comme un `Element`, de sorte que les valeurs primitives de ses extensions soient typées. Un primitif XML sans valeur ni `id`/`extension` (par exemple `<active/>`) SHALL être conservé tel quel afin que `TYPE_001` se déclenche. Un texte mal formé SHALL rester une chaîne afin que `TYPE_001` se déclenche. Les éléments `contentReference` et les ressources `contained` SHALL NOT être remodelés. Les tableaux SHALL conserver le regroupement XML (frères répétés uniquement).

#### Scenario: Primitif XML typé

- **WHEN** `validate` reçoit une ressource XML avec `<active value="true"/>`
- **THEN** `active` SHALL être validé comme booléen sans `TYPE_001`

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

