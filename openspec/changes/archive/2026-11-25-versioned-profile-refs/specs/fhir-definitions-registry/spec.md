## MODIFIED Requirements

### Requirement: Références de contenu et profils de types

Chaque `ElementDefinition` SHALL exposer `content_reference: Option<Arc<str>>` (un `#chemin` local perd le `#` ; une référence absolue `url#chemin` est conservée entière) et `type_profiles: Vec<TypeProfiles>` où `TypeProfiles` a `code`, `profiles`, `target_profiles`. Une entrée SHALL exister uniquement pour un code de type déclarant `profile` ou `targetProfile`, `profiles` et `target_profiles` conservés tels qu'écrits (version `|version` incluse) ; le validateur retire la version pour `targetProfile`, les discriminateurs `url` des slices d'extension (`Extension.url` porte le canonique nu) et les recherches `baseDefinition`.

#### Scenario: contentReference locale

- **WHEN** un élément déclare `contentReference` valant `#Bundle.entry`
- **THEN** `content_reference` SHALL valoir `Bundle.entry`

#### Scenario: contentReference absolue

- **WHEN** un élément déclare `contentReference` valant `http://x/SD#A.b`
- **THEN** `content_reference` SHALL valoir `http://x/SD#A.b`

#### Scenario: Type sans profil

- **WHEN** un type ne déclare ni `profile` ni `targetProfile`
- **THEN** `type_profiles` SHALL ne contenir aucune entrée pour ce code

#### Scenario: Profil versionné conservé

- **WHEN** un type déclare `profile` valant `http://x/SD|1.0`
- **THEN** `type_profiles[..].profiles` SHALL contenir `http://x/SD|1.0`
