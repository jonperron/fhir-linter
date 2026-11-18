## ADDED Requirements

### Requirement: Validation des profils — meaningWhenMissing interdit (DEFINITION_001)

Pour toute StructureDefinition (ressource racine ou intégrée) dont `derivation` vaut `constraint`, le système SHALL émettre une erreur `DEFINITION_001` pour chaque `differential.element[i]` qui renseigne `meaningWhenMissing`, au `path` `<ressource>.differential.element[<i>].meaningWhenMissing`; le message SHALL nommer le chemin de l'élément. Selon ElementDefinition.meaningWhenMissing, une signification implicite ne peut être définie que sur une ressource, un type de données ou une extension, jamais dans un profil. Aucun diagnostic SHALL être émis si `derivation` vaut `specialization` ou est absente, si la définition est une extension (`type` Extension et `baseDefinition`, sans `|version`, égal à l'url de la StructureDefinition Extension du Registry), ni si la contrainte n'a pas de `type` (contrôle des champs obligatoires). Le snapshot SHALL NOT être vérifié. Un profil d'une autre extension SHALL être signalé.

#### Scenario: profil avec meaningWhenMissing

- **WHEN** une StructureDefinition `constraint` a un `differential.element[1]` avec `meaningWhenMissing`
- **THEN** une erreur `DEFINITION_001` SHALL être émise à `<ressource>.differential.element[1].meaningWhenMissing`, son message nommant le chemin de l'élément

#### Scenario: définitions exemptées

- **WHEN** la définition est une `specialization`, sans `derivation`, une extension de base, ou une contrainte sans `type`
- **THEN** aucun Diagnostic `DEFINITION_001` SHALL être émis

#### Scenario: profil d'une autre extension

- **WHEN** une StructureDefinition de type Extension a pour `baseDefinition` une autre extension et `derivation` `constraint`, avec `meaningWhenMissing` dans le différentiel
- **THEN** une erreur `DEFINITION_001` SHALL être émise
