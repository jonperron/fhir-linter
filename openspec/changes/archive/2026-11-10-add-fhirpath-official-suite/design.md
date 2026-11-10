## Context

La suite officielle FHIRPath R5 sert de mesure de conformité de l'évaluateur. Le harnais vit dans `crates/fhir-fhirpath/tests/official_suite.rs`.

## Goals / Non-Goals

**Goals:** mesurer le nombre de cas réussis et bloquer les régressions via un plancher.

**Non-Goals:** vérification statique de types (`contextType` ignoré) ; modes `cda`, `tx`, `custom` (infrastructure hors FHIRPath et FHIR R5).

## Decisions

- Plancher absolu (758 cas réussis) plutôt qu'un taux ; à relever quand l'évaluateur progresse.
- Les cas stricts et laxistes s'exécutent de la même façon ; certaines paires ne peuvent pas réussir toutes deux.
- `catch_unwind` isole chaque cas : une panique compte comme un échec sans arrêter la suite.

## Risks / Trade-offs

- Une mise à jour des fixtures peut changer le nombre de cas ; le plancher doit alors être revu.
