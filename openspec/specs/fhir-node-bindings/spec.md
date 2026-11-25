# Spec: fhir-node-bindings

## Purpose

Addon Node.js natif (napi-rs) exposant le validateur et le linter. Paquet npm `@jonperron/fhir-linter` (tarball unique, tous les `.node` inclus, publié sur GitHub Packages et joint aux releases GitHub, pas de publication sur npmjs.org), ESM, Node.js uniquement.
## Requirements
### Requirement: Fonction validate

Le module SHALL exposer `validate(json: string, definitionsDir?: string, extraDefinitions?: string[]): Promise<Diagnostic[]>` qui analyse une ressource FHIR R5 JSON et résout avec les diagnostics de `fhir_validator::validate`. Le calcul SHALL s'exécuter hors de la boucle d'événements.

#### Scenario: Type de ressource inconnu

- **WHEN** `validate('{"resourceType": "Nope"}')` est appelé
- **THEN** la promesse SHALL résoudre avec un unique Diagnostic `STRUCTURE_000` de sévérité `error`

#### Scenario: JSON invalide

- **WHEN** le JSON ne peut pas être analysé
- **THEN** la promesse SHALL être rejetée avec une `Error`

### Requirement: Fonction lint

Le module SHALL exposer `lint(json: string, configToml?: string, definitionsDir?: string, extraDefinitions?: string[]): Promise<Diagnostic[]>` qui exécute les règles intégrées de `fhir-linter`. `configToml` est le contenu d'un `fhir-linter.toml`.

#### Scenario: Configuration invalide

- **WHEN** `configToml` n'est pas du TOML valide, ou référence une règle ou une sévérité inconnue
- **THEN** la promesse SHALL être rejetée avec une `Error`

### Requirement: Source des définitions

Les définitions FHIR R5 ne sont pas embarquées. `definitionsDir` SHALL provenir d'une source de confiance (aucune restriction de chemin ; un répertoire distinct charge un registre supplémentaire, jamais évincé). Le répertoire SHALL être `definitionsDir`, à défaut la variable d'environnement `FHIR_DEFINITIONS_DIR`. Le `Registry` SHALL être construit une fois par répertoire puis partagé.

#### Scenario: Aucun répertoire ou répertoire illisible

- **WHEN** aucun répertoire n'est défini, ou qu'il ne contient pas les définitions
- **THEN** la promesse SHALL être rejetée avec une `Error`

### Requirement: Définitions supplémentaires

`validate` et `lint` SHALL accepter un paramètre optionnel `extraDefinitions: string[]`. Chaque entrée SHALL désigner un fichier `.json` (une ressource de conformité ou un Bundle) ou un répertoire dont les fichiers `*.json` sont lus de façon non récursive. Les définitions SHALL être chargées via `fhir_definitions::Registry::with_extra` au-dessus du registre de base partagé, une fois par appel et sans mise en cache ; le registre de base SHALL rester partagé et inchangé. Absent ou vide, le comportement SHALL être identique à celui d'avant.

Toute erreur de chargement SHALL rejeter la promesse avec une `Error` dont le message est `cannot load extra definitions: <détail>`. Le détail SHALL nommer le fichier (et l'URL canonique en cas de conflit) et SHALL NOT contenir le contenu du fichier. Causes : URL canonique déjà présente dans la base ou fournie deux fois, StructureDefinition de dérivation `specialization`, fichier illisible, absent ou trop volumineux, JSON mal formé, Bundle mal formé, dépassement des limites.

#### Scenario: Profil fourni

- **WHEN** `validate` est appelé avec `extraDefinitions` fournissant un profil de contrainte avec snapshot, et une ressource le déclare dans `meta.profile`
- **THEN** les diagnostics SHALL refléter ce profil et l'avertissement « not known » SHALL disparaître

#### Scenario: Liste absente ou vide

- **WHEN** `extraDefinitions` est omis ou `[]`
- **THEN** les diagnostics SHALL être identiques à ceux d'un appel sans le paramètre

#### Scenario: Erreur de chargement

- **WHEN** une entrée redéfinit une URL canonique de la base, est manquante ou contient un JSON mal formé
- **THEN** la promesse SHALL être rejetée avec une `Error` « cannot load extra definitions: » nommant le fichier sans en reproduire le contenu

### Requirement: Type Diagnostic

`Diagnostic` SHALL être un objet avec `severity` (code FHIR R5 IssueSeverity : `fatal`, `error`, `warning` ou `information`), `code`, `message`, `path`, `line`, `col`, `offset` (absents sans localisation) et `rule` (absent pour le validateur). `col` (1-indexée) et `offset` (0-indexé) SHALL être des comptes d'octets UTF-8 de l'entrée telle que fournie (BOM inclus), et non des indices UTF-16 de chaîne JavaScript : pour retrouver la position, il faut indexer `Buffer.from(json)`.

#### Scenario: Diagnostic du validateur

- **WHEN** `validate` retourne un finding sans localisation
- **THEN** `line`, `col`, `offset` et `rule` SHALL être absents du Diagnostic

#### Scenario: Offset en octets UTF-8

- **WHEN** un Diagnostic localisé est produit pour une source contenant des caractères non ASCII avant la position signalée
- **THEN** `Buffer.from(json).subarray(d.offset)` SHALL commencer au nœud signalé

#### Scenario: Sévérité configurée en code IssueSeverity

- **WHEN** `lint` est appelé avec `[rules.narrative-absent] severity = "fatal"`, respectivement `"hint"`
- **THEN** le Diagnostic `narrative-absent` SHALL avoir `severity` égal à `"fatal"`, respectivement `"information"`

