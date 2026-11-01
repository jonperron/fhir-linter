# fhir-validator Specification

## Purpose
TBD - created by archiving change fhir-validator-phase5. Update Purpose after archive.
## Requirements
### Requirement: API publique de validation

Le système SHALL exposer une fonction `validate(resource: &Resource, registry: &Registry) -> Vec<Diagnostic>` qui applique toutes les couches de validation dans l'ordre et retourne l'ensemble des findings.

#### Scenario: Ressource valide retourne une liste vide

- **WHEN** `validate` est appelé avec une ressource Patient valide et un registry R5 chargé
- **THEN** la fonction SHALL retourner un `Vec<Diagnostic>` vide (aucune erreur)

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

Le système SHALL émettre exactement un `Diagnostic` avec code `STRUCTURE_000` et severity `Error` lorsque le type de ressource n'est pas connu du registry, et SHALL arrêter toute validation supplémentaire.

#### Scenario: Type de ressource inconnu

- **WHEN** `validate` est appelé avec une ressource dont `resourceType` n'existe pas dans le registry
- **THEN** la fonction SHALL retourner exactement un Diagnostic avec code `STRUCTURE_000` et severity `Error`

---

### Requirement: Validation structurelle — élément inconnu (STRUCTURE_001)

Le système SHALL émettre un `Diagnostic` avec code `STRUCTURE_001` et severity `Error` pour chaque champ de la ressource qui ne correspond à aucun chemin de profondeur 1 dans le snapshot de la StructureDefinition.

#### Scenario: Élément inconnu détecté

- **WHEN** une ressource contient un champ `unknownField` absent du snapshot
- **THEN** un Diagnostic `STRUCTURE_001` SHALL être émis dont `path` contient le nom du champ

#### Scenario: Champ connu ne produit pas d'erreur

- **WHEN** une ressource contient uniquement des champs définis dans le snapshot
- **THEN** aucun Diagnostic `STRUCTURE_001` ne SHALL être émis

#### Scenario: Extensions et extensions primitives toujours autorisées

- **WHEN** une ressource contient des champs `extension`, `modifierExtension`, ou commençant par `_`
- **THEN** aucun Diagnostic `STRUCTURE_001` ne SHALL être émis pour ces champs

#### Scenario: Champ polymorphique valide (value[x])

- **WHEN** une ressource contient `valueBoolean` et la StructureDefinition déclare `value[x]`
- **THEN** aucun Diagnostic `STRUCTURE_001` ne SHALL être émis

---

### Requirement: Validation de cardinalité — élément requis absent (CARDINALITY_001)

Le système SHALL émettre un `Diagnostic` avec code `CARDINALITY_001` et severity `Error` pour chaque élément déclaré avec `min > 0` dans le snapshot et absent de la ressource.

#### Scenario: Élément requis absent

- **WHEN** une ressource Observation est fournie sans le champ `status` (min=1)
- **THEN** un Diagnostic `CARDINALITY_001` SHALL être émis mentionnant `status`

#### Scenario: Élément optionnel absent ne produit pas d'erreur

- **WHEN** une ressource est fournie sans un champ optionnel (min=0)
- **THEN** aucun Diagnostic `CARDINALITY_001` ne SHALL être émis pour ce champ

#### Scenario: Placeholders polymorphiques ignorés

- **WHEN** le snapshot contient un élément `value[x]`
- **THEN** la cardinalité de ce placeholder SHALL être ignorée (pas de CARDINALITY_001)

---

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

Le système SHALL émettre un `Diagnostic` avec code `TYPE_001` et severity `Error` lorsqu'une valeur n'est pas du type JSON attendu par la StructureDefinition (ex. : nombre fourni pour un champ `string`, ou chaîne pour un champ `boolean`).

#### Scenario: Boolean attendu, string reçu

- **WHEN** un champ de type `boolean` contient une valeur JSON string
- **THEN** un Diagnostic `TYPE_001` SHALL être émis

#### Scenario: String attendue, nombre reçu

- **WHEN** un champ de type `string` contient une valeur JSON numérique
- **THEN** un Diagnostic `TYPE_001` SHALL être émis

---

### Requirement: Validation de type — contrainte de valeur entière (TYPE_002)

Le système SHALL émettre un `Diagnostic` avec code `TYPE_002` et severity `Error` lorsqu'une valeur entière viole une contrainte de valeur : valeur négative pour `unsignedInt`, ou valeur inférieure ou égale à zéro pour `positiveInt`.

#### Scenario: unsignedInt négatif

- **WHEN** un champ de type `unsignedInt` contient la valeur JSON `-1`
- **THEN** un Diagnostic `TYPE_002` SHALL être émis

#### Scenario: positiveInt à zéro

- **WHEN** un champ de type `positiveInt` contient la valeur JSON `0`
- **THEN** un Diagnostic `TYPE_002` SHALL être émis

---

### Requirement: Validation de type — format date/heure (TYPE_003)

Le système SHALL émettre un `Diagnostic` avec code `TYPE_003` et severity `Error` lorsqu'une valeur de type `date`, `dateTime`, `instant` ou `time` ne respecte pas le format attendu.

#### Scenario: Date invalide

- **WHEN** un champ de type `date` contient la chaîne `"not-a-date"`
- **THEN** un Diagnostic `TYPE_003` SHALL être émis

#### Scenario: Date valide ne produit pas d'erreur

- **WHEN** un champ de type `date` contient `"2024-01-15"`
- **THEN** aucun Diagnostic `TYPE_003` ne SHALL être émis

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

#### Scenario: Identifiant invalide

- **WHEN** un champ de type `id` contient `"invalid id!"`
- **THEN** un Diagnostic `TYPE_005` SHALL être émis

#### Scenario: Identifiant valide

- **WHEN** un champ de type `id` contient `"patient-123"`
- **THEN** aucun Diagnostic `TYPE_005` ne SHALL être émis

---

### Requirement: Validation des invariants FHIRPath (INVARIANT_<key>)

Le système SHALL évaluer les expressions FHIRPath de contrainte définies sur l'élément racine du snapshot. Pour chaque contrainte dont l'expression ne retourne pas `[true]`, il SHALL émettre un `Diagnostic` avec code `INVARIANT_<key>` où `<key>` est la valeur du champ `key` de la contrainte.

#### Scenario: Contrainte violée émet INVARIANT_<key>

- **WHEN** une contrainte avec `key="dom-2"` et `expression="..."` évalue à `[false]`
- **THEN** un Diagnostic avec code `INVARIANT_dom-2` SHALL être émis avec la severity correspondant à la severity de la contrainte

#### Scenario: Contrainte satisfaite n'émet pas de diagnostic

- **WHEN** une contrainte FHIRPath évalue à `[true]`
- **THEN** aucun Diagnostic `INVARIANT_` ne SHALL être émis pour cette contrainte

#### Scenario: Erreur d'évaluation FHIRPath silencieusement ignorée

- **WHEN** l'évaluation FHIRPath d'une contrainte retourne une erreur
- **THEN** aucun Diagnostic ne SHALL être émis pour cette contrainte (l'erreur est ignorée)

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

Le système SHALL émettre `PROFILE_002` (erreur) lorsqu'un profil connu de `meta.profile` s'applique à un autre type de ressource que celui de la ressource, sauf si le profil porte sur un parent abstrait (ex. `DomainResource`). Pour un profil connu compatible et distinct de la définition de base, les règles de cardinalité du profil SHALL être appliquées ; les constats déjà rapportés par la validation de base (même code et même `path`) SHALL être dédupliqués.

#### Scenario: Profil d'un autre type

- **WHEN** une ressource `Patient` déclare un profil dont le type est `Observation`
- **THEN** un Diagnostic `PROFILE_002` SHALL être émis

#### Scenario: Profil sur parent abstrait

- **WHEN** le profil est défini sur un type abstrait (ex. `DomainResource`)
- **THEN** aucun `PROFILE_002` SHALL être émis

#### Scenario: Cardinalité du profil dédupliquée

- **WHEN** un élément requis par le profil est aussi requis par la base et absent
- **THEN** un seul Diagnostic `CARDINALITY_001` SHALL être émis pour ce `path`

---

### Requirement: Exécution des manifestes de test officiels

Le crate `fhir-validator` SHALL fournir un test d'intégration exécutant le manifeste `validator/manifest.json` de `fhir-test-cases` pour les cas R5 JSON autonomes (avec `java.errorCount` et sans `supporting`, `profile`, `profiles`, `packages`, `logical`). Chaque cas est jugé sur « présence d'erreurs » contre « erreurs attendues » ; le taux de réussite SHALL être au moins 0,6 (plancher de régression). Le répertoire de fixtures SHALL pouvoir être surchargé par `FHIR_FIXTURES_DIR` ; si le manifeste est absent, le test SHALL être ignoré.

#### Scenario: Taux de réussite sous le plancher

- **WHEN** le taux de réussite est inférieur à 0,6
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

