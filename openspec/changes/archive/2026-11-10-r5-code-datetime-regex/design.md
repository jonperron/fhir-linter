## Context

Les formats primitifs suivent les regex R5 (`.claude/rules/fhir.md` : la spec R5 a toujours raison). `code` et `dateTime` s'en écartaient.

## Goals / Non-Goals

**Goals :** aligner `code` et `dateTime` sur leurs regex R5.
**Non-Goals :** renommer l'exigence TYPE_004 ; toucher aux autres types primitifs.

## Decisions

- `is_valid_code` : découpe sur `' '`, chaque mot non vide et sans whitespace (`char::is_whitespace` : tabulation, saut de ligne, espace insécable rejetés). Pas de dépendance regex.
- `is_valid_datetime` : en plus de `date` et `instant`, un préfixe de 7 ou 10 caractères valide comme `date` suivi d'un fuseau complet (`is_valid_zone`). Le `+`/`-` nu que la regex tolère est rejeté, comme pour `instant`.

## Risks / Trade-offs

- Des ressources auparavant en erreur (codes à espaces simples, dateTime date+fuseau) passent désormais : conforme à R5.
