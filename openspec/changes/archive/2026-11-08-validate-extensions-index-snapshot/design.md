## Context

Les extensions sont déclarées dans les snapshots avec le type `Extension` ; les
traiter comme des éléments ordinaires suffit à les valider.

## Goals / Non-Goals

**Goals:** valider les extensions comme tout autre élément ; rejeter les scalaires
pour les types complexes ; recherche d'élément en O(1).

**Non-Goals:** validation des définitions d'extension (résolution de
`Extension.url`), parcours des ressources contenues.

## Decisions

- Suppression du filtre par nom : seul le préfixe `_` (extension de primitif)
  reste ignoré dans le parcours ; il compte uniquement pour la présence.
- Le contrôle scalaire/objet est fait là où le type complexe est résolu, les
  tableaux étant parcourus élément par élément.
- L'index `path -> position` conserve la première position pour les chemins
  répétés (slices), comme l'ancien `find` linéaire. `descendants` exploite
  l'ordre du snapshot (descendants contigus après leur parent).

## Risks / Trade-offs

- Le snapshot ne doit plus être modifié après `new` (champ public, index
  privé) ; le registre ne distribue que des références `&`.
