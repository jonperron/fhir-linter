## Context

Le remodelage XML conserve le regroupement XML : un élément XML unique reste scalaire, la forme tableau n'y est donc pas significative.

## Goals / Non-Goals

**Goals:** signaler en JSON toute forme tableau/scalaire contraire au `max` de la définition de base, à toute profondeur.

**Non-Goals:** contrôle de forme pour l'XML, pour les compagnons `_nom`, ou selon la cardinalité des profils.

## Decisions

- Le contrôle est fait dans le parcours imbriqué (`check_array_shape`), sur chaque champ résolu, à partir de l'élément de la définition de base : `max != "1"` exige un tableau, `max = "1"` l'interdit.
- Activé seulement si `Resource.from_xml` est faux.
- Les valeurs `null` ne sont pas signalées.
- `STRUCTURE_002` et `CARDINALITY_002` peuvent coexister (ex. `"gender": ["male","female"]`) : ils décrivent deux défauts distincts.

## Risks / Trade-offs

- Double diagnostic pour un même champ dans certains cas ; accepté pour garder des contrôles indépendants.
