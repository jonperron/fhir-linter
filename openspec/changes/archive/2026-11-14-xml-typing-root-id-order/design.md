## Context

Le parseur XML remonte `<id>` dans `Resource.id`, sans position.

## Goals / Non-Goals

Typer le XML comme le JSON ; contrôler l'ordre de l'id racine. Pas de changement de format de diagnostic.

## Decisions

- `ResourceId` conserve le span et se déréférence en `str` pour limiter l'impact sur les appelants.
- Les positions du snapshot viennent de l'index de chemins existant (`position`).

## Risks

Un texte XML non conforme reste `String` : pas de faux typage.
