## Context

La navigation stricte (N1 §3) existe dans fhir-fhirpath en opt-in via `Environment::strict`.

## Goals / Non-Goals

- Goal: signaler les noms inconnus au lieu de les évaluer à vide.
- Non-Goal: changer les planchers du manifeste ou rendre une instance invalide pour une expression de profil invalide.

## Decisions

- Un seul `Strict` par appel de `validate`, partagé par toutes les évaluations (cache `has_element`, index des chemins).
- Erreur stricte = information `INVARIANT_SKIPPED` / `SLICING_SKIPPED`, jamais une violation.

## Risks

- Des invariants du noyau pourraient nommer un élément inconnu : couvert par `core_invariants_resolve_every_name_on_a_corpus_sample`.
