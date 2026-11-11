## Context

Le parseur retire `id` des champs de la ressource ; seul `resource.id` le porte, sans span.

## Goals / Non-Goals

**Goals:** appliquer `TYPE_006` à l'`id` racine.

**Non-Goals:** ajouter une localisation source à l'`id` racine.

## Decisions

- Réutiliser `type_check::control_char_warning` avec le chemin `<Type>.id`, sans localisation, comme `TYPE_005` sur l'`id` racine.

## Risks / Trade-offs

- Pas de localisation pour ce diagnostic, cohérent avec `TYPE_005`.
