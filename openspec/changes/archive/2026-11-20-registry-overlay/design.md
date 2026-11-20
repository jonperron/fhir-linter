## Context

Le registry de base est en lecture seule et mis en cache (`Registry::shared`).

## Goals / Non-Goals

- Goals: overlay immuable, erreurs nommant le fichier sans contenu, bornes de taille.
- Non-Goals: cache des overlays, répertoires récursifs, StructureDefinitions `specialization`.

## Decisions

- Toute recherche lit l'overlay puis la base ; `is_choice_variant` fait l'union ; la borne de `type_ancestors` compte les deux couches.
- Conflit d'URL refusé plutôt que masqué (toute version, par type de ressource).
- Une SD sans snapshot est hors map dans tous les registries ; `needs_snapshot` la signale.

## Risks

- Les SD sans snapshot sont inutilisables tant qu'un snapshot n'est pas généré ; aucune SD R5 embarquée n'est concernée.
