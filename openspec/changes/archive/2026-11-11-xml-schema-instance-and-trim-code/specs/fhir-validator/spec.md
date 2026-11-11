## ADDED Requirements

### Requirement: Validation de type — valeur d'attribut XML épurée (TYPE_007)

Pour une ressource XML, chaque valeur d'attribut épurée par le parseur (`Resource::trimmed_attributes`) sur un élément connu SHALL produire un `Diagnostic` avec code `TYPE_007` et severity `Warning`. Le `path` SHALL être le chemin d'instance de l'élément (ex. `Patient.active`, `Patient._gender.id`, `Patient._birthDate.extension.url`). Les blancs retirés SHALL NOT produire d'erreur de type (`TYPE_001`). Ce code est distinct de `TYPE_006`, réservé aux caractères de contrôle.

#### Scenario: Valeur primitive épurée

- **WHEN** `validate` reçoit une ressource XML `Patient` avec `<active value=" true"/>`
- **THEN** un Diagnostic `TYPE_007` de severity `Warning` SHALL être émis au chemin `Patient.active`, sans `TYPE_001` ni `TYPE_006`

#### Scenario: id et url épurés

- **WHEN** `validate` reçoit un `id` épuré sur `<gender>` ou un `url` épuré sur une extension de `<birthDate>`
- **THEN** un `TYPE_007` de severity `Warning` SHALL être émis au chemin respectif `Patient._gender.id` ou `Patient._birthDate.extension.url`

### Requirement: Validation structurelle — namespace schema instance XML (STRUCTURE_003)

Pour une ressource XML, le système SHALL émettre, pour chaque élément enregistré dans `Resource::schema_instance`, un `Diagnostic` avec code `STRUCTURE_003`, severity `Error`, `path` = type de la ressource, localisation = span de l'élément, et message `The XML schema instance namespace is not allowed in FHIR resources` (xml.html 2.1.6.3.4). Une ressource JSON SHALL NOT produire ce diagnostic.

#### Scenario: xsi:schemaLocation sur la racine

- **WHEN** `validate` reçoit un `Patient` XML dont la racine déclare `xmlns:xsi` et porte `xsi:schemaLocation`
- **THEN** exactement un Diagnostic `STRUCTURE_003` de severity `Error` SHALL être émis avec `path` = `Patient`, la localisation de la racine et le message `The XML schema instance namespace is not allowed in FHIR resources`

#### Scenario: Plusieurs éléments concernés

- **WHEN** deux éléments distincts de la ressource déclarent le namespace XMLSchema-instance
- **THEN** deux Diagnostics `STRUCTURE_003` SHALL être émis, chacun à la localisation de son élément

## MODIFIED Requirements

### Requirement: Remodelage des ressources XML

Quand `Resource.from_xml` est vrai (positionné par `parse_xml`), `validate` SHALL remodeler la ressource une seule fois en forme JSON avant toute validation : les primitifs `boolean`, `integer`, `unsignedInt`, `positiveInt` et `decimal` sont typés d'après les types d'éléments du Registry, et `id`/`extension` d'un primitif sont déplacés vers `_nom`. La partie `_nom` SHALL elle-même être remodelée comme un `Element`, de sorte que les valeurs primitives de ses extensions soient typées. Un primitif XML sans valeur ni `id`/`extension` (par exemple `<active/>`) SHALL être conservé tel quel afin que `TYPE_001` se déclenche. Un texte mal formé SHALL rester une chaîne afin que `TYPE_001` se déclenche. Les éléments `contentReference` et les ressources `contained` SHALL NOT être remodelés. Les tableaux SHALL conserver le regroupement XML (frères répétés uniquement).

Le remodelage SHALL vérifier la forme XML de chaque élément d'après `ElementDefinition.xml_attr` : un élément dont la `representation` contient `xmlAttr` (valeur d'un primitif, `Element.id`, `Extension.url`) donné comme élément enfant, et tout autre élément donné comme attribut, SHALL produire un Diagnostic `STRUCTURE_001` au chemin d'instance de cet élément.

Une valeur d'attribut épurée par le parseur (`Resource::trimmed_attributes`) sur un élément connu SHALL produire un Diagnostic `TYPE_007` de sévérité `Warning` au chemin d'instance de l'élément, au lieu d'une erreur de type.

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
