## MODIFIED Requirements

### Requirement: Validation des références — type déclaré incohérent (REFERENCE_003)

Le système SHALL émettre `REFERENCE_003` (erreur), au `path` `<élément>.reference`, lorsque, à toute profondeur, `Reference.type` (nom de type nu ou `http://hl7.org/fhir/StructureDefinition/<Type>`) diffère du type d'une référence URL FHIR (references.html 2.1.3.0.1). Une URL FHIR est une référence qui correspond entièrement à l'expression régulière de référence littérale R5 `((http|https)://([A-Za-z0-9\-\\\.\:\%\$]*\/)+)?(Type)\/[A-Za-z0-9\-\.]{1,64}(\/_history\/[A-Za-z0-9\-\.]{1,64})?`, où `Type` est un type de ressource concret connu du Registry. Une référence `#id` vers une ressource contenue (racine, ressource contenue au sein de la liste d'ids de son conteneur, entrée de Bundle ; JSON et XML) prend le `resourceType` de cette ressource contenue comme type cible, lorsque c'est un type de ressource concret connu du Registry ; sans `resourceType` ou avec un `resourceType` inconnu, aucun contrôle de type n'est fait. Dans un Bundle, une référence absolue (ex. `urn:uuid:`) égale exactement au `fullUrl` d'une entrée du Bundle le plus interne (un Bundle imbriqué est sa propre portée ; les ressources contenues d'une entrée utilisent le Bundle de l'entrée) prend le `resourceType` de cette entrée comme type cible (bundle.html 2.4.5.1) ; entrées de même `fullUrl` mais de types différents, et références sans entrée correspondante, ne sont pas résolues et ne sont jamais signalées. Les références relatives ne sont pas résolues contre les bases de `fullUrl`. Les références conditionnelles, `#` seul, `urn:` sans entrée de Bundle correspondante et les URL absolues non conformes ne sont pas des URL FHIR et SHALL être ignorées par cette règle.

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

#### Scenario: Référence urn:uuid sans entrée correspondante

- **WHEN** dans un Bundle, `reference` vaut `urn:uuid:y`, `type` vaut `Practitioner` et aucune entrée n'a ce `fullUrl` (ou deux entrées de types différents le partagent)
- **THEN** aucun Diagnostic `REFERENCE_003` SHALL être émis

### Requirement: Validation des références — type cible non autorisé (REFERENCE_005)

Le système SHALL émettre `REFERENCE_005` (erreur), au `path` `<élément>.reference`, lorsque, à toute profondeur, une référence URL FHIR (même définition par expression régulière de référence littérale R5 que `REFERENCE_003` : `Type/id` relatif ou absolu http(s), `_history` optionnel) désigne un type de ressource non autorisé par la liste `targetProfile` du `Reference` de l'élément. Chaque `targetProfile` compte pour le type nommé par le chemin racine de son snapshot (un profil comme `vitalsigns` compte pour `Observation`). Un `targetProfile` autorise son type racine ET tout type qui en dérive (chaîne `baseDefinition`, via `Registry::type_ancestors`) : `Resource` autorise toute ressource, `DomainResource` toute ressource de domaine. Aucun `targetProfile`, ou un `targetProfile` que le Registry ne résout pas (cible inconnue ; tout `targetProfile` du noyau R5 se résout), SHALL autoriser tout type. Une référence `#id` désignant une ressource contenue est contrôlée avec le `resourceType` de celle-ci comme type cible (même portée que `REFERENCE_003`) ; sans `resourceType` ou avec un `resourceType` inconnu, aucun contrôle. Dans un Bundle, une référence absolue (ex. `urn:uuid:`) égale exactement au `fullUrl` d'une entrée du Bundle le plus interne (un Bundle imbriqué est sa propre portée ; les ressources contenues d'une entrée utilisent le Bundle de l'entrée) prend le `resourceType` de cette entrée comme type cible (bundle.html 2.4.5.1) ; entrées de même `fullUrl` mais de types différents, et références sans entrée correspondante, ne sont pas résolues et ne sont jamais signalées. Les références relatives ne sont pas résolues contre les bases de `fullUrl`. `#` seul, `urn:` sans entrée correspondante, les URL absolues non FHIR, conditionnelles et logiques (`identifier` seul) NE SONT PAS contrôlées en type.

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

## ADDED Requirements

### Requirement: Validation des références — fullUrl d'entrée en désaccord avec l'id (REFERENCE_007)

Le système SHALL émettre `REFERENCE_007` (erreur), au `path` `Bundle.entry[i]`, lorsque le `fullUrl` absolu d'une entrée correspond entièrement à l'expression régulière d'URL RESTful `[base]/Type/id[/_history/v]` et que sa partie `id` diffère de l'`id` de la ressource de l'entrée (Bundle.entry.fullUrl : « The fullUrl SHALL NOT disagree with the id in the resource »). Aucun diagnostic SHALL être émis si la ressource n'a pas d'`id` ou si le `fullUrl` n'est pas RESTful (ex. `urn:uuid:`). La partie type n'est pas comparée (R5 ne nomme que l'id).

#### Scenario: id en désaccord

- **WHEN** une entrée a `fullUrl` `http://example.org/fhir/Patient/1` et une ressource d'`id` `2`
- **THEN** un Diagnostic d'erreur `REFERENCE_007` SHALL être émis avec `path` = `Bundle.entry[0]`

#### Scenario: id concordant, absent ou fullUrl non RESTful

- **WHEN** l'`id` concorde, la ressource n'a pas d'`id`, ou le `fullUrl` vaut `urn:uuid:...`
- **THEN** aucun Diagnostic `REFERENCE_007` SHALL être émis
