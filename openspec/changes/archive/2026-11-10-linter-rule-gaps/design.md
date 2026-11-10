## Context

Les règles intégrées testaient la seule présence d'une clé JSON, sans suivre les règles R5 sur le contenu du narratif ni le commentaire de `Bundle.identifier`.

## Goals / Non-Goals

**Goals:** aligner les déclencheurs sur R5 ; localiser les diagnostics.
**Non-Goals:** parser XHTML complet ; dédupliquer LINT_001 et dom-6.

## Decisions

- Contenu du div : balises retirées, il reste du texte non blanc (références d'entité comprises) ou un élément `<img>`. Raccourci : une balise finit au premier `>`.
- LINT_001 reste un surensemble délibéré de dom-6 (décision utilisateur) : lint et validation sont deux sorties distinctes.
- Un élément répétable n'est « donné » que si son tableau contient au moins un élément non nul.
- Bundle : identité persistante requise seulement pour `document`, `message`, `collection` ; un Bundle sans `type` n'est pas signalé.
- Position : span de `meta` pour LINT_002/003 quand `meta` existe, span de `text` pour LINT_001.

## Risks / Trade-offs

- Un `>` dans un attribut ou un commentaire peut masquer un div vide (faux négatif accepté).
- Doublon INVARIANT_dom-6 + LINT_001 quand `text` est absent.
