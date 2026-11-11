## Context

La présence d'un élément est déterminée par `present()` dans `nested.rs`, utilisée par le parcours unique de validation et par la couche de cardinalité de profil.

## Goals / Non-Goals

**Goals :** traiter `[]` et `[null, ...]` comme absence ; avertir pour un élément optionnel.

**Non-Goals :** les variantes `[x]` (max=1, un tableau produit déjà `STRUCTURE_002`).

## Decisions

- `present()` filtre la valeur par `is_empty_array` avant de retomber sur `_nom` : un `_nom` contenant un objet reste une présence valide.
- Élément optionnel absent mais dont le champ existe sous forme de tableau vide : `CARDINALITY_003` (Warning), `path` = chemin d'instance, `location` = span du tableau.
- Coexistence assumée : `STRUCTURE_002` pour un `[]` sur `max=1` ; base `CARDINALITY_003` + profil `CARDINALITY_001` lorsqu'un profil rend l'élément requis (la déduplication code+path ne fusionne pas des codes différents).

## Risks / Trade-offs

- Des ressources auparavant sans diagnostic reçoivent un warning → conforme à R5 (SHOULD).
