## Context

Mise à jour de spec après coup : le code de `fix/fhirpath-operator-semantics` est déjà écrit et testé (`crates/fhir-fhirpath/tests/fhirpath_tests.rs`, suite officielle à 817 réussites).

## Goals / Non-Goals

**Goals:** décrire dans la spec le comportement observable des opérateurs corrigés.

**Non-Goals:** comparaison objet/primitive (reste vide jusqu'à F10) ; tri des date-heures par valeur dans `sort`/`min`/`max` (reste textuel).

## Decisions

- `EvalError::DivisionByZero` est conservé et documenté comme jamais produit, pour ne pas casser les `match` sur l'enum public.
- Les règles transverses (singleton, égalité, booléens, division) deviennent des exigences dédiées plutôt que d'être dispersées dans chaque famille de fonctions.
- Les exemples des scénarios reprennent les expressions des tests ajoutés sur la branche.

## Risks / Trade-offs

- Les invariants FHIR qui s'appuyaient sur l'ancienne tolérance peuvent maintenant échouer en erreur de type ; le validateur les signale déjà comme non vérifiés.
