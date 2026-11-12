## MODIFIED Requirements

### Requirement: Validation des invariants FHIRPath (INVARIANT_<key>)

Le système SHALL évaluer les expressions FHIRPath de contrainte définies sur l'élément du snapshot de chaque nœud d'instance, ainsi que sur l'élément racine de son type, à toute profondeur (par ex. `qty-3` sur toute `Quantity`, `ext-1` sur toute `Extension`) ; chaque clé SHALL être évaluée une seule fois par nœud. Le `path` du Diagnostic SHALL être le chemin d'instance du nœud (ex. `Patient.contact[0]`) et sa localisation le span du nœud ; les contraintes de la ressource racine restent signalées au chemin du type de ressource, sans span. Un primitif SHALL être évalué avec son élément JSON `_nom`, appariés par indice ; un `_nom` sans valeur SHALL être évalué au chemin nu de l'élément (ex. `Patient.name[0].family`) ; un item JSON `null` n'est pas évalué. Chaque expression SHALL être analysée une seule fois par contrainte (cache). La ressource SHALL être convertie par `fhir_fhirpath::resource_to_typed_value` (racine) ou `fhir_fhirpath::node_to_typed_value` (nœuds imbriqués, même typage) avec `Registry::child_element`, de sorte que les primitives `date`, `dateTime`, `instant` et `time` soient comparées comme des valeurs temporelles. Pour chaque contrainte dont l'expression ne retourne pas `[true]` (une seule valeur vraie, y compris un booléen FHIR portant des extensions), il SHALL émettre un `Diagnostic` avec code `INVARIANT_<key>` où `<key>` est la valeur du champ `key` de la contrainte. Le code `INVARIANT_SKIPPED` est réservé : il signale une contrainte non vérifiée et n'est jamais une violation. La severity R5 d'une contrainte est liée (binding required) à constraint-severity (`error | warning`) : seule la valeur exacte `warning` SHALL donner `Severity::Warning` ; toute autre valeur (`error`, mais aussi `information`, `Warning` ou un code inconnu) SHALL donner `Severity::Error`.

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

#### Scenario: Invariant dgr-1 ignoré (resolve() non implémenté)

- **WHEN** une DiagnosticReport R5 avec `composition` est validée (contrainte `dgr-1`, dont l'expression appelle `resolve()`)
- **THEN** aucun Diagnostic `INVARIANT_dgr-1` ne SHALL être émis
- **AND** exactement un Diagnostic `Information` `INVARIANT_SKIPPED` nommant `dgr-1` SHALL être émis

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

### Requirement: Validation terminologique — binding required (TERMINOLOGY_001)

Le système SHALL émettre `TERMINOLOGY_001` lorsqu'un élément, à toute profondeur (y compris dans un datatype, ex. `HumanName.use`, et pour une variante de choix dont le binding s'applique au type résolu de la variante, ex. `versionAlgorithmCoding`), de type `code`, `Coding` ou `CodeableConcept`, possède un binding de force `required` (Diagnostic d'erreur) ou `extensible` (Diagnostic de sévérité warning) dont le ValueSet est connu et que le code est prouvé hors du ValueSet. Les contentReference, les ressources contenues et les autres forces de binding (`preferred`, `example`) SHALL être ignorés. Un `Coding` sans `system` NE DOIT JAMAIS être considéré comme membre du ValueSet : il compte comme hors du ValueSet, même lorsque l'appartenance serait autrement indécidable. Hors de ce cas, une appartenance indécidable (`Registry::value_set_contains` retourne `None`) NE DOIT JAMAIS être signalée. Pour un `CodeableConcept`, un seul `coding` appartenant au ValueSet suffit ; le Diagnostic n'est émis que si tous les `coding` portant un code sont hors du ValueSet. Sous un binding `required`, un `CodeableConcept` sans aucun `coding` portant un code (texte seul) SHALL être signalé avec un message commençant par `CodeableConcept with no coded Coding at` (terminologies.html 4.1.4.2.1) ; sous un binding `extensible`, le texte seul SHALL être accepté (4.1.4.2.2). Un `coding` unique non encapsulé dans un tableau (un seul `<coding>` en XML) SHALL être lu comme un tableau d'un élément.

#### Scenario: Code hors du ValueSet requis

- **WHEN** un `code` lié en `required` à un ValueSet connu contient une valeur absente de ce ValueSet
- **THEN** un Diagnostic d'erreur `TERMINOLOGY_001` SHALL être émis avec le `path` de l'élément (index inclus pour les tableaux)

#### Scenario: Code hors du ValueSet extensible

- **WHEN** un `CodeableConcept` lié en `extensible` à un ValueSet connu ne contient que des `coding` prouvés hors de ce ValueSet
- **THEN** un Diagnostic `TERMINOLOGY_001` de sévérité warning SHALL être émis

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

Le système SHALL émettre `REFERENCE_002` (erreur) lorsqu'une référence `#id` ne correspond à aucun `id` dans `contained`, ou lorsque la référence est `#` seule : ref-1 n'autorise `#` que si `%rootResource != %resource`, et le validateur ne valide que la ressource racine. `#id` SHALL être résolu contre les `id` contenus de la ressource racine, quelle que soit la profondeur de la référence. En XML, un `<contained>` unique SHALL être accepté non encapsulé et l'`id` de chaque ressource contenue SHALL être lu à l'intérieur de son élément de type de ressource.

#### Scenario: Contained introuvable

- **WHEN** `reference` vaut `#p1` et `contained` ne contient aucune ressource d'`id` `p1`
- **THEN** un Diagnostic `REFERENCE_002` SHALL être émis

#### Scenario: Contained présent

- **WHEN** `contained` contient une ressource d'`id` `p1`
- **THEN** aucun Diagnostic SHALL être émis

#### Scenario: Dièse seul à la racine

- **WHEN** `reference` vaut `#` dans la ressource racine
- **THEN** un Diagnostic `REFERENCE_002` SHALL être émis

#### Scenario: Contained résolu depuis un nœud imbriqué

- **WHEN** une `Reference` imbriquée vaut `#p1` et la racine contient `p1`
- **THEN** aucun Diagnostic `REFERENCE_002` SHALL être émis

#### Scenario: Contained XML unique

- **WHEN** un seul élément `contained` XML contenant une ressource d'`id` `p1` est présent
- **THEN** `#p1` SHALL être résolu

### Requirement: Validation des références — type déclaré incohérent (REFERENCE_003)

Le système SHALL émettre `REFERENCE_003` (erreur), au `path` `<élément>.reference`, lorsque, à toute profondeur, `Reference.type` (nom de type nu ou `http://hl7.org/fhir/StructureDefinition/<Type>`) diffère du type d'une référence URL FHIR (references.html 2.1.3.0.1). Une URL FHIR est une référence qui correspond entièrement à l'expression régulière de référence littérale R5 `((http|https)://([A-Za-z0-9\-\\\.\:\%\$]*\/)+)?(Type)\/[A-Za-z0-9\-\.]{1,64}(\/_history\/[A-Za-z0-9\-\.]{1,64})?`, où `Type` est un type de ressource concret connu du Registry. Les références conditionnelles, `urn:`, `#id` et les URL absolues non conformes ne sont pas des URL FHIR et SHALL être ignorées par cette règle.

#### Scenario: Type déclaré différent

- **WHEN** `reference` vaut `Patient/1` (ou `http://example.org/fhir/Patient/1`) et `type` vaut `Practitioner`
- **THEN** un Diagnostic `REFERENCE_003` SHALL être émis avec `path` se terminant par `.reference`

#### Scenario: Type déclaré sous forme d'URL canonique

- **WHEN** `reference` vaut `Patient/1` et `type` vaut `http://hl7.org/fhir/StructureDefinition/Patient`
- **THEN** aucun Diagnostic `REFERENCE_003` SHALL être émis

#### Scenario: Référence qui n'est pas une URL FHIR

- **WHEN** `reference` vaut `urn:uuid:...`, `#p1` ou `Patient?identifier=x` et `type` vaut `Practitioner`
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

### Requirement: Validation des éléments imbriqués

Un seul parcours récursif SHALL appliquer ces vérifications, en partant du premier niveau de la ressource ; il s'exécute avant les couches terminologie, invariants, références et profil. À toute profondeur, un champ `_nom` SHALL être accepté seulement si `nom` résout un élément (ou une variante de choix) de type primitif, sinon `STRUCTURE_001` au chemin de `_nom` (ex. `Patient.name[0]._bogus`). Un `_nom` accepté SHALL être parcouru via la StructureDefinition du type complexe `Element` (contrôle de `id` et `extension`) ; un tableau SHALL être parcouru élément par élément (index dans le `path`, éléments `null` ignorés) ; une valeur non objet, ou un élément de tableau ni objet ni `null`, SHALL produire `TYPE_001` (ex. `"_birthDate": "x"` → `Patient._birthDate`, `"_given": [1]` → `Patient.name[0]._given[0]`). Le champ `_nom` lui-même n'est pas soumis à STRUCTURE_002 ; son contenu l'est pour une entrée JSON. Les champs `extension` et `modifierExtension` SHALL être traités comme tout autre élément de type `Extension`, à toute profondeur : résolus dans le snapshot (STRUCTURE_001 s'ils n'y figurent pas, ex. `Patient.name[0].modifierExtension`), puis parcourus via la StructureDefinition `Extension` (cardinalité de `url`, type de `value[x]`, extensions imbriquées). Un élément `[x]` SHALL être vérifié comme un tout, au premier niveau comme en profondeur : requis sans variante → `CARDINALITY_001` (au chemin `…[x]`), deux variantes → `CARDINALITY_002`, une variante présente soumise à `CARDINALITY_002` selon son `max`. Le système SHALL descendre dans les `BackboneElement` (via le même snapshot) et dans les types complexes (via leur StructureDefinition `complex-type`) et y appliquer les vérifications structurelles (STRUCTURE_001, et STRUCTURE_002 pour les entrées JSON), de cardinalité (CARDINALITY_001/002/003) et de type (TYPE_*), avec un `path` d'instance incluant les index (ex. `Patient.name[0].bogus`). Les éléments `[x]` SHALL être résolus par leur suffixe. Les couches terminologie, invariants et références s'exécutent à toute profondeur sur ce même parcours ; seule la couche profil reste limitée au premier niveau ; les `contentReference` (ignorés sans erreur, jamais signalés STRUCTURE_001) et les ressources contenues ne sont pas parcourus. La règle `_nom` s'applique à toute profondeur : un enfant requis présent uniquement sous la forme `_nom` SHALL être considéré présent seulement si son type est primitif et si `_nom` est un objet ou un tableau d'objets et de `null` contenant au moins un objet, et un `_nom` en tableau au-delà du maximum fini SHALL produire `CARDINALITY_002`.

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
- **THEN** les Diagnostics correspondants SHALL être émis ; les ressources contenues et les entrées de Bundle ne SHALL pas être parcourues
