## Context

`narrative-absent` coupait les balises au premier `>`. Le parseur XML livre le div comme texte décodé, et stocke un primitif avec extension comme objet `{ value, extension }`.

## Goals / Non-Goals

**Goals:** analyser le div JSON comme XHTML ; lire `text.status`, `text.div` et `Bundle.type` depuis un primitif avec extension en XML.

**Non-Goals:** conserver le balisage du div dans le parseur XML (un div XML réduit à une image reste signalé).

## Decisions

- `quick-xml` (`Reader::from_str`) parcourt le div : `Text`/`CData` non blancs, références de caractère non blanches, entités nommées et `<img>` (nom local) comptent comme contenu.
- Blanc = espace, tabulation, CR, LF (définition XML) ; U+00A0 est du texte.
- Une erreur de lecture renvoie « contenu présent » : le XHTML mal formé relève du validateur.
- Helper `primitive()` : chaîne, ou `value` d'un objet seulement si `resource.from_xml` (en JSON l'extension est dans `_name`).

## Risks / Trade-offs

- Faux positif : un div XML ne contenant qu'une `<img>` déclenche LINT_001 (shortcut documenté dans le code).
