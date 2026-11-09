## Context

`xml_shape.rs` déplaçait `id`/`extension` vers `_nom` sans les remodeler, et ignorait les primitifs vides.

## Goals / Non-Goals

**Goals:** typer les primitifs dans les extensions de primitif ; signaler les primitifs vides.
**Non-Goals:** modifier le remodelage des `contentReference` et des `contained`.

## Decisions

- `_nom` est remodelé via la StructureDefinition `Element` du registry (aucune forme codée en dur).
- Un primitif vide reste un objet vide, que la vérification de type rejette (`TYPE_001`).

## Risks / Trade-offs

- Des ressources XML auparavant acceptées peuvent produire `TYPE_001` (conforme à FHIR).
