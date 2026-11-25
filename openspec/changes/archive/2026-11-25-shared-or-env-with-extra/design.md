## Context

Python et Node répétaient la même logique `shared_or_env` puis `with_extra`.

## Goals / Non-Goals

- Goal : une seule implémentation partagée.
- Non-goal : changer le comportement ou mettre en cache les surcouches.

## Decisions

- Sans `extra`, l'`Arc` de base est retourné tel quel ; sinon `Registry::with_extra` est enveloppé dans un nouvel `Arc`, construit à chaque appel.
- `SharedRegistryError` distingue `Base` et `Extra` pour que les bindings gardent leurs messages.

## Risks

Aucun : refactor neutre.
