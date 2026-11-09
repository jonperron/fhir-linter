## Context

Le parseur XML produit `Value::Str` pour tous les primitifs, avec `id`/`extension` en enfants.

## Goals / Non-Goals

- Goal: le validateur voit la même forme que pour JSON.
- Non-goal: remodeler `contentReference` et les ressources `contained` (le validateur n'y descend pas).

## Decisions

- Remodelage unique au début de `validate`, piloté par `Resource.from_xml` et les types du Registry.
- Texte mal formé conservé en chaîne afin que TYPE_001 se déclenche.
- Les tableaux gardent le regroupement XML (frères répétés uniquement).

## Risks

- Un élément XML unique de cardinalité `*` reste un scalaire.
