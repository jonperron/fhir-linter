## Context

Audit des règles terminologiques et de références de `fhir-validator` face à terminologies.html 4.1.4.2, references.html 2.1.3.0.1 et l'invariant ref-1.

## Goals / Non-Goals

- Objectifs : bindings `extensible`, Coding sans `system`, texte seul sous `required`, `#` seul, cohérence `Reference.type`, forme des références relatives.
- Hors périmètre : résolution des URL absolues, éléments imbriqués, bindings `preferred`/`example`.

## Decisions

- `extensible` en warning : la spécification ne fixe pas de sévérité de validateur.
- Coding sans `system` toujours hors ValueSet : son code n'a pas de sens défini ; `TERMINOLOGY_002` le signale en plus au chemin du Coding.
- `#` seul en erreur : seule la ressource racine est validée, donc ref-1 échoue toujours.
- `REFERENCE_003` en erreur (SHALL) ; `REFERENCE_004` en warning (regex non normative). Les deux réutilisent un même analyseur de la regex littérale R5, sans dépendance `regex`.
- `entries()` partagé : un objet unique est lu comme un tableau d'un élément (XML).

## Risks / Trade-offs

- Nouveaux Diagnostics sur des ressources auparavant acceptées (texte seul, Coding sans `system`, `#`).
