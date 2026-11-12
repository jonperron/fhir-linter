## MODIFIED Requirements

### Requirement: Type Binding sur ElementDefinition

Le système SHALL fournir un type `Binding` public avec les champs : `strength: Arc<str>`, `value_set: Option<Arc<str>>`, `additional: Vec<AdditionalBinding>`. Chaque `ElementDefinition` SHALL exposer un champ `binding: Option<Binding>`. `AdditionalBinding` SHALL avoir les champs `purpose`, `value_set` (version canonique retirée) et `any: bool` (défaut `false`).

#### Scenario: Binding chargé depuis le snapshot

- **WHEN** un élément du snapshot déclare un binding (`binding` dans le JSON)
- **THEN** `element.binding` SHALL être `Some(Binding)` avec `strength` et `value_set` renseignés

#### Scenario: Binding sans valueSet

- **WHEN** un binding JSON ne possède pas de champ `valueSet`
- **THEN** `Binding.value_set` SHALL être `None`

#### Scenario: Élément sans binding

- **WHEN** un élément du snapshot ne déclare pas de binding
- **THEN** `element.binding` SHALL être `None`

#### Scenario: Binding additionnel chargé

- **WHEN** un binding déclare une extension de binding additionnel avec `purpose` et `valueSet` versionné
- **THEN** `Binding.additional` SHALL contenir une `AdditionalBinding` dont `value_set` n'a pas de version et dont `any` vaut `false` si absent

#### Scenario: Binding additionnel incomplet

- **WHEN** un binding additionnel n'a pas de `purpose` ou pas de `valueSet`
- **THEN** il SHALL être ignoré, comme un binding sans `strength`

### Requirement: Export public de Constraint et Binding

Le crate `fhir-definitions` SHALL exporter `Constraint`, `Binding`, `AdditionalBinding`, `ChoiceValue`, `Discriminator`, `Slicing` et `TypeProfiles` dans son API publique de premier niveau.

#### Scenario: Importation directe depuis fhir-definitions

- **WHEN** un crate dépendant importe `use fhir_definitions::{Binding, Constraint, AdditionalBinding, ChoiceValue, Discriminator, Slicing, TypeProfiles}`
- **THEN** la compilation SHALL réussir sans erreur

## ADDED Requirements

### Requirement: Références de contenu et profils de types

Chaque `ElementDefinition` SHALL exposer `content_reference: Option<Arc<str>>` (un `#chemin` local perd le `#` ; une référence absolue `url#chemin` est conservée entière) et `type_profiles: Vec<TypeProfiles>` où `TypeProfiles` a `code`, `profiles`, `target_profiles`. Une entrée SHALL exister uniquement pour un code de type déclarant `profile` ou `targetProfile`, versions canoniques retirées.

#### Scenario: contentReference locale

- **WHEN** un élément déclare `contentReference` valant `#Bundle.entry`
- **THEN** `content_reference` SHALL valoir `Bundle.entry`

#### Scenario: contentReference absolue

- **WHEN** un élément déclare `contentReference` valant `http://x/SD#A.b`
- **THEN** `content_reference` SHALL valoir `http://x/SD#A.b`

#### Scenario: Type sans profil

- **WHEN** un type ne déclare ni `profile` ni `targetProfile`
- **THEN** `type_profiles` SHALL ne contenir aucune entrée pour ce code

### Requirement: Valeurs fixed, pattern, minValue et maxValue

Chaque `ElementDefinition` SHALL exposer `fixed`, `pattern`, `min_value`, `max_value` de type `Option<Box<ChoiceValue>>`, où `ChoiceValue` a `key` (clé JSON, ex. `fixedCode`) et `value: serde_json::Value`. Une clé SHALL correspondre uniquement si le préfixe est suivi d'une majuscule.

#### Scenario: fixedCode chargé

- **WHEN** un élément porte `fixedCode: "x"`
- **THEN** `fixed` SHALL être `Some` avec `key == "fixedCode"` et la valeur `"x"`

#### Scenario: Clé de primitive étendue ignorée

- **WHEN** un élément ne porte que `_fixedCode`
- **THEN** `fixed` SHALL être `None`

### Requirement: Slicing et longueur maximale

Chaque `ElementDefinition` SHALL exposer `slicing: Option<Box<Slicing>>` avec `discriminators: Vec<Discriminator{kind, path}>`, `rules` et `ordered` (défaut `false`), ainsi que `max_length: Option<u32>`. Un slicing sans `rules`, ou un discriminateur sans `type` ou `path`, SHALL faire échouer le chargement du bundle avec `MalformedBundle` nommant l'élément.

#### Scenario: Slicing chargé

- **WHEN** un élément déclare un slicing avec un discriminateur, `rules` et sans `ordered`
- **THEN** `slicing` SHALL être `Some` avec `ordered == false`

#### Scenario: Slicing sans rules

- **WHEN** un élément déclare un slicing sans `rules`
- **THEN** le chargement SHALL échouer avec `MalformedBundle` nommant l'élément

#### Scenario: Discriminateur sans path

- **WHEN** un discriminateur n'a pas de `path` ou de `type`
- **THEN** le chargement SHALL échouer avec `MalformedBundle` nommant l'élément
