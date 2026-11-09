## Context

`walk` (nested.rs) sautait toute clé `_nom`. Les extensions de primitif ont pourtant une forme définie (`Element` : `id`, `extension`).

## Goals / Non-Goals

**Goals:** signaler les `_nom` orphelins ou portant sur un type complexe ; valider leur contenu.
**Non-Goals:** modifier les règles de présence/cardinalité de `_nom`.

## Decisions

- Résolution de `nom` par le même mécanisme que les champs ordinaires (variantes de choix incluses) ; primitif = nom de type commençant par une minuscule, comme ailleurs.
- Le contenu est parcouru via la StructureDefinition `Element` du registry (aucune forme codée en dur).
- Valeur non objet → `TYPE_001` ; `null` dans un tableau ignoré (alignement avec la valeur primitive).

## Risks / Trade-offs

- Des ressources auparavant acceptées peuvent désormais produire `STRUCTURE_001`/`TYPE_001` (comportement conforme à FHIR).
