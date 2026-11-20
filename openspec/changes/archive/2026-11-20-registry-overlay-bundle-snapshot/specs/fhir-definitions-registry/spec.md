## MODIFIED Requirements

### Requirement: Erreurs de chargement de l'overlay

`with_extra` SHALL nommer le fichier fautif dans chaque erreur et SHALL NOT en reproduire le contenu : `Error::Conflict{url,file}` si une URL existe déjà dans la base (toute version, par type de ressource, y compris une StructureDefinition de base sans snapshot) ou apparaît deux fois parmi les extras ; `Error::Unsupported{file,message}` pour une StructureDefinition `derivation=specialization` ou un chemin qui n'est ni un fichier `.json` ni un répertoire ; `Error::Read{file,source}` pour un fichier illisible ou de plus de 64 MiB (`io::ErrorKind::FileTooLarge`) ; `Error::Limit{file,message}` au-delà de 256 MiB au total ou de 10 000 fichiers `.json` dans un répertoire ; `Error::Json` pour un JSON invalide ; `Error::MalformedBundle` pour un Bundle dont `entry` est présent mais n'est pas un tableau. Un Bundle sans `entry` ne charge rien, sans erreur.

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

#### Scenario: Entry de Bundle mal formé

- **WHEN** un extra est un Bundle dont `entry` est présent mais n'est pas un tableau
- **THEN** `with_extra` SHALL retourner `Error::MalformedBundle` nommant le fichier

#### Scenario: Bundle sans entry

- **WHEN** un extra est un Bundle sans `entry`
- **THEN** `with_extra` SHALL réussir sans rien charger de ce Bundle

### Requirement: StructureDefinition sans snapshot

Une `StructureDefinition` sans snapshot (absent ou liste d'éléments vide) SHALL être écartée de la map de tout registry (base comprise ; aucune SD R5 embarquée n'en est dépourvue) et SHALL être signalée par `Registry::needs_snapshot(url) -> bool`. Lorsqu'une même URL est chargée plusieurs fois dans une couche, seul son dernier état SHALL être conservé : elle est soit dans la map (avec snapshot), soit dans `needs_snapshot`, jamais les deux.

#### Scenario: SD sans snapshot signalée

- **WHEN** une StructureDefinition chargée n'a pas de snapshot
- **THEN** `structure_definition(url)` SHALL retourner `None` et `needs_snapshot(url)` SHALL retourner `true`

#### Scenario: SD avec snapshot

- **WHEN** une StructureDefinition a un snapshot non vide
- **THEN** `needs_snapshot(url)` SHALL retourner `false`

#### Scenario: Dernier état conservé

- **WHEN** une même URL de StructureDefinition est chargée plusieurs fois dans une couche, avec puis sans snapshot (ou l'inverse)
- **THEN** seul le dernier état SHALL subsister : dans la map ou dans `needs_snapshot`, jamais les deux
