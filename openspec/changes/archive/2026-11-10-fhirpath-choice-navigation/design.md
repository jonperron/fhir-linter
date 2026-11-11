## Context

L'évaluateur FHIRPath travaille sur du JSON sans information de type ; il ne peut pas savoir seul qu'un champ `valueQuantity` est une variante de `value[x]`.

## Goals / Non-Goals

**Goals:** navigation `value` → `valueQuantity` pour les invariants ; aucun faux positif sur un frère non-choix (`modifierExtension` n'est jamais lu comme `modifier`).

**Non-Goals:** résolution par chemin d'élément ; navigation des choix dans `evaluate` / `evaluate_parsed`.

## Decisions

- Le prédicat `is_choice_variant(base, suffix)` est injecté par l'appelant (`&dyn Fn(&str, &str) -> bool`) : `fhir-fhirpath` reste indépendant de `fhir-definitions`.
- Une clé explicite `name` est prioritaire ; les clés `_name...` (éléments primitifs étendus) ne correspondent jamais.
- L'index du Registry associe le nom de base aux codes de type déclarés (capitalisés) de tous les éléments `*.base[x]`, toutes ressources confondues.

## Risks / Trade-offs

- Raccourci documenté : la correspondance se fait par nom, pas par chemin (`dose` trouve `Immunization.doseQuantity` partout). Faux positif possible si un élément non-choix porte le nom `base + Type` d'un choix d'une autre ressource ; à remplacer par une résolution par chemin si cela se produit.
