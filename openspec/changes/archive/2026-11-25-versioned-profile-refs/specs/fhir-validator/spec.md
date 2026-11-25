## MODIFIED Requirements

### Requirement: Validation de profil — profil inconnu (PROFILE_001)

Le système SHALL émettre un Diagnostic de sévérité warning `PROFILE_001` pour chaque URL de `meta.profile` inconnue du Registry (suffixe `|version` séparé de l'URL pour la résolution ; une référence avec `#fragment` — `url|version#id` désigne la ressource contenue `id` de cette version, que le Registry n'indexe pas — SHALL NOT être résolue et produit `PROFILE_001`, « not known »), avec le `path` `<Type>.meta.profile[i]`. Une URL nommant une StructureDefinition fournie sans snapshot (voir `PROFILE_010`) SHALL produire `PROFILE_010` à la place de `PROFILE_001`.

#### Scenario: Profil inconnu

- **WHEN** `meta.profile` contient une URL absente du Registry
- **THEN** un Diagnostic `PROFILE_001` de sévérité warning SHALL être émis

#### Scenario: Référence avec fragment

- **WHEN** `meta.profile` contient `http://hl7.org/fhir/StructureDefinition/vitalsigns|5.0.0#vs`
- **THEN** un `PROFILE_001` de sévérité warning SHALL être émis, et aucun `PROFILE_003`

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
