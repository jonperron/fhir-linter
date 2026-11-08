## Context

`present` et `check_choice` acceptaient toute valeur sous `_nom` pour un type primitif.

## Goals / Non-Goals

**Goals:** n'accepter que la forme JSON d'une extension de primitif.
**Non-Goals:** valider le contenu de l'extension elle-même.

## Decisions

- Un helper unique `is_extension_shape` (objet, ou tableau d'objets/`null` avec au moins un objet), utilisé par `present` et `check_choice`, donc appliqué à toute profondeur.

## Risks / Trade-offs

- Des ressources auparavant acceptées avec un `_nom` scalaire produisent désormais `CARDINALITY_001` ; c'est le comportement voulu.
