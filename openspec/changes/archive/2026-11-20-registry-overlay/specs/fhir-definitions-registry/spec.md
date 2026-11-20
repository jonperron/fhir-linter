## ADDED Requirements

### Requirement: Registry overlay avec définitions supplémentaires

Le système SHALL fournir `Registry::with_extra(base: Arc<Registry>, paths)` qui construit un registry overlay immuable. Chaque chemin SHALL être soit un fichier `.json` (une ressource ou un Bundle), soit un répertoire lu sans récursion pour ses fichiers réguliers `*.json` en ordre trié (liens symboliques et autres fichiers ignorés). Seules les `StructureDefinition`, `ValueSet` et `CodeSystem` SHALL être chargées ; les autres ressources et les définitions sans `url`, `name` ou `kind` SHALL être ignorées. Chaque recherche (`structure_definition`, `core`, `value_set`, `value_set_contains`, CodeSystems) SHALL lire l'overlay puis la base ; `is_choice_variant` SHALL faire l'union des deux couches ; la borne de pas de `type_ancestors` SHALL compter les deux couches. La base et `Registry::shared` SHALL rester inchangés et les overlays SHALL NOT être mis en cache.

#### Scenario: Définition supplémentaire résolue

- **WHEN** `with_extra` charge un fichier contenant une StructureDefinition d'URL inconnue de la base
- **THEN** `structure_definition(url)` SHALL la retourner et la base SHALL rester sans cette URL

#### Scenario: Repli sur la base

- **WHEN** une URL n'est pas dans l'overlay mais est dans la base
- **THEN** la recherche SHALL retourner la définition de la base

#### Scenario: Répertoire non récursif

- **WHEN** un chemin est un répertoire contenant des sous-répertoires, des liens symboliques et des fichiers non `.json`
- **THEN** seuls les fichiers réguliers `*.json` du premier niveau SHALL être chargés, en ordre trié

### Requirement: Erreurs de chargement de l'overlay

`with_extra` SHALL nommer le fichier fautif dans chaque erreur et SHALL NOT en reproduire le contenu : `Error::Conflict{url,file}` si une URL existe déjà dans la base (toute version, par type de ressource, y compris une StructureDefinition de base sans snapshot) ou apparaît deux fois parmi les extras ; `Error::Unsupported{file,message}` pour une StructureDefinition `derivation=specialization` ou un chemin qui n'est ni un fichier `.json` ni un répertoire ; `Error::Read{file,source}` pour un fichier illisible ou de plus de 64 MiB (`io::ErrorKind::FileTooLarge`) ; `Error::Limit{file,message}` au-delà de 256 MiB au total ou de 10 000 fichiers `.json` dans un répertoire ; `Error::Json` pour un JSON invalide.

#### Scenario: Conflit avec la base

- **WHEN** un extra déclare une URL déjà présente dans la base
- **THEN** `with_extra` SHALL retourner `Error::Conflict` avec l'URL et le fichier

#### Scenario: Doublon entre extras

- **WHEN** deux extras déclarent la même URL pour le même type de ressource
- **THEN** `with_extra` SHALL retourner `Error::Conflict`

#### Scenario: Spécialisation refusée

- **WHEN** un extra est une StructureDefinition `derivation=specialization`
- **THEN** `with_extra` SHALL retourner `Error::Unsupported` nommant le fichier

#### Scenario: Chemin invalide

- **WHEN** un chemin n'est ni un fichier `.json` ni un répertoire
- **THEN** `with_extra` SHALL retourner `Error::Unsupported`

#### Scenario: Fichier trop gros

- **WHEN** un fichier fait plus de 64 MiB
- **THEN** `with_extra` SHALL retourner `Error::Read` de kind `FileTooLarge`

#### Scenario: Limites globales

- **WHEN** le total dépasse 256 MiB ou un répertoire contient plus de 10 000 fichiers `.json`
- **THEN** `with_extra` SHALL retourner `Error::Limit`

#### Scenario: JSON invalide

- **WHEN** un extra n'est pas du JSON valide
- **THEN** `with_extra` SHALL retourner `Error::Json`

### Requirement: StructureDefinition sans snapshot

Une `StructureDefinition` sans snapshot (absent ou liste d'éléments vide) SHALL être écartée de la map de tout registry (base comprise ; aucune SD R5 embarquée n'en est dépourvue) et SHALL être signalée par `Registry::needs_snapshot(url) -> bool`.

#### Scenario: SD sans snapshot signalée

- **WHEN** une StructureDefinition chargée n'a pas de snapshot
- **THEN** `structure_definition(url)` SHALL retourner `None` et `needs_snapshot(url)` SHALL retourner `true`

#### Scenario: SD avec snapshot

- **WHEN** une StructureDefinition a un snapshot non vide
- **THEN** `needs_snapshot(url)` SHALL retourner `false`

### Requirement: Lecture plafonnée partagée

Le crate SHALL exporter `fhir_definitions::read_capped(path, cap)` et `MAX_INPUT_BYTES` (64 MiB), partagés avec la CLI, dont le comportement SHALL rester inchangé.

#### Scenario: Fichier au-delà du plafond

- **WHEN** `read_capped` lit un fichier plus grand que `cap`
- **THEN** il SHALL échouer sans lire le fichier en mémoire
