## Context

Voir `crates/fhir-definitions`. Les specs sont en français.

## Goals / Non-Goals

Documenter l'API et le comportement ; pas de nouveau comportement.

## Decisions

`expand` et `translate` renvoient `None` quand le résultat n'est pas exact, comme `value_set_contains`.

## Risks

`conceptmaps.json` manquant fait échouer la construction (`Error::Io`).
