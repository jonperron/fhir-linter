## Context

Voir `content_reference_profiles` et `reference_problem` dans `structure_definition.rs`.

## Goals / Non-Goals

Valider la forme et la résolution. Non-objectif : cibles sans snapshot, tranches.

## Decisions

Auto-référence résolue contre la définition puis sa base ; dédoublonnage snapshot/différentiel.

## Risks

Cibles sans snapshot non vérifiées (silence volontaire).
