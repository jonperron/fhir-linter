## MODIFIED Requirements

### Requirement: Validation des invariants FHIRPath (INVARIANT_<key>)

Le système SHALL évaluer les expressions FHIRPath de contrainte définies sur l'élément du snapshot de chaque nœud d'instance, ainsi que sur l'élément racine de son type, à toute profondeur (par ex. `qty-3` sur toute `Quantity`, `ext-1` sur toute `Extension`) ; chaque clé SHALL être évaluée une seule fois par nœud. Le `path` du Diagnostic SHALL être le chemin d'instance du nœud (ex. `Patient.contact[0]`) et sa localisation le span du nœud ; les contraintes de la ressource racine restent signalées au chemin du type de ressource, sans span. Les contraintes racine d’une ressource SHALL être celles de son élément racine et celles des éléments racine de ses ancêtres ressource (`DomainResource`, `Resource` ; `Base` exclu car son `ele-1` est une règle d’élément), chaque clé une seule fois ; le R5 publie 37 racines dérivées de `DomainResource` (ex. `ValueSet`, `CodeSystem`) sans `dom-2` à `dom-6`, qui SHALL quand même leur être appliquées par cet héritage. Un primitif SHALL être évalué avec son élément JSON `_nom`, appariés par indice ; un `_nom` sans valeur SHALL être évalué au chemin nu de l'élément (ex. `Patient.name[0].family`) ; le `_id` de la racine SHALL être apparié à l'`id` de la ressource, que le parseur extrait des champs ; un item JSON `null` n'est pas évalué. Chaque invariant (racine, imbriqué, profil de type) SHALL être évalué par `fhir_fhirpath::evaluate_parsed_with_environment` avec un `Environment` dont : `member_of` est construit depuis `Registry::value_set_contains` (`memberOf`) ; `is_subtype` est construit depuis `Registry::type_ancestors` (`is`, `as` et `ofType` répondent pour les sous-types FHIR : `canonical` et `url` sont des `uri`, `code` est un `string`) ; `%resource` est la ressource englobant le nœud à toute profondeur (une ressource contenue ou une entrée de Bundle est son propre `%resource`) ; `%rootResource` est le conteneur le plus externe pour une ressource contenue, sinon `%resource` (une entrée de Bundle est sa propre racine). Une valeur sans type FHIR (élément inconnu, STRUCTURE_001/TYPE_001) interrogée sur un sous-type FHIR de `string` SHALL donner `INVARIANT_SKIPPED`. Chaque expression SHALL être analysée une seule fois par contrainte (cache). La ressource SHALL être convertie par `fhir_fhirpath::resource_to_typed_value` (racine) ou `fhir_fhirpath::node_to_typed_value` (nœuds imbriqués, même typage) avec `Registry::child_element`, de sorte que les primitives `date`, `dateTime`, `instant` et `time` soient comparées comme des valeurs temporelles. Pour chaque contrainte dont l'expression ne retourne pas `[true]` (une seule valeur vraie, y compris un booléen FHIR portant des extensions), il SHALL émettre un `Diagnostic` avec code `INVARIANT_<key>` où `<key>` est la valeur du champ `key` de la contrainte. Le code `INVARIANT_SKIPPED` est réservé : il signale une contrainte non vérifiée et n'est jamais une violation. `fhir_validator::validate` SHALL toujours évaluer le FHIRPath avec la navigation stricte (N1 §3) : il construit un seul `fhir_fhirpath::Strict` par appel, à partir de `Registry::has_element`, partagé par toutes les évaluations de l'arbre de la ressource (racine et ressources imbriquées) : invariants racine du noyau, invariants d'éléments imbriqués, contraintes de profil (racine, élément, profils de type) et discriminateurs de découpage. Une expression nommant un élément que le type ne déclare pas échoue à l'évaluation et SHALL être signalée `INVARIANT_SKIPPED` (jamais une violation : la spécification FHIR ne rend pas l'instance invalide à cause d'une expression de profil invalide). Le message de `INVARIANT_SKIPPED` SHALL avoir la forme `constraint <key> not checked: <reason> (in '<expression>')`, nommant la clé, la raison (ex. `type error: familly is not an element of HumanName`) et l'expression. La severity R5 d'une contrainte est liée (binding required) à constraint-severity (`error | warning`) : seule la valeur exacte `warning` SHALL donner `Severity::Warning` ; toute autre valeur (`error`, mais aussi `information`, `Warning` ou un code inconnu) SHALL donner `Severity::Error`.

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

### Requirement: Validation de profil — découpage (PROFILE_006 à PROFILE_009, SLICING_SKIPPED)

Pour chaque élément découpé, le système SHALL affecter chaque répétition à la première slice dont elle satisfait tous les discriminateurs, les chemins de discriminateur étant évalués avec l'évaluateur FHIRPath sur la valeur typée de l'élément. Les types de discriminateur sont : `value`/`pattern` (un `fixed` exige l'égalité exacte, un `pattern` un sous-ensemble ; sans valeur fixe, toute valeur au chemin convient), `exists` (la cible avec `min>0` doit exister, avec `max=0` doit être absente), `type` (`$this` sur un choix `[x]` utilise le type de la clé ; sinon le type FHIR de la valeur ou son `resourceType` ; correspondance exacte, un type de slice abstrait accepte ses spécialisations) et `profile` (`$this` uniquement ; conforme = aucune erreur issue des règles de ce profil, y compris les contraintes de sa racine et de ses éléments, hors celles du type du noyau que le parcours de base évalue). Les slices d'extension SHALL être appariées par l'URL de leur unique `type.profile`, sur `url`. Pour un élément à choix `[x]`, chaque variante autorisée par la définition de base SHALL constituer un item. La comparaison `fixed`/`pattern` SHALL porter sur les valeurs FHIRPath (`_name` JSON fusionné ; une correspondance exacte rejette une primitive portant `id`/extensions).

Le système SHALL émettre, au chemin d'instance concerné :
- `PROFILE_006` (erreur) : le nombre d'items d'une slice est hors de son `min`/`max` (au chemin de l'élément découpé) ;
- `PROFILE_007` (erreur) : découpage fermé et un item ne correspond à aucune slice (au chemin de l'item) ;
- `PROFILE_008` (erreur) : découpage ordonné et un item correspond à une slice déclarée avant la slice d'un item précédent ;
- `PROFILE_009` (erreur) : slice `openAtEnd` et un item apparaît après un item non apparié ;
- `SLICING_SKIPPED` (information) : un discriminateur ne peut être évalué (chemin fonction tel que `resolve()`, `ofType()`, `extension('url')` ; chemin non analysable ; type non supporté tel que `position` ; slice ne donnant aucune valeur au chemin ; profil inconnu ; erreur d'évaluation, y compris un chemin nommant un élément que le type ne déclare pas, la navigation étant stricte ; la raison est alors `cannot evaluate discriminator '<path>': <error>`). Il SHALL être émis seulement s'il existe des items ou une slice avec `min>0`, et le découpage n'est alors pas évalué.

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
