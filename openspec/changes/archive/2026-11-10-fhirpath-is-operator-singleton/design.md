## Context

N1 §6.3.1 définit l'opérateur `is` comme équivalent à la fonction `is()`.

## Goals / Non-Goals

- Goal : l'opérateur `is` et `is()` partagent la même sémantique.
- Non-goal : imposer la règle du singleton à l'opérateur `as` (inchangé, filtre toujours).

## Decisions

- L'opérateur applique la même logique que `is()` (vide, singleton, erreur `EvalError::Type`).

## Risks / Trade-offs

- Les expressions appliquant l'opérateur `is` à plusieurs éléments échouent désormais à l'évaluation, conformément à la spécification.
