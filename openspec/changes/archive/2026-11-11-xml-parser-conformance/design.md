## Context

xml.html impose des règles que le parseur XML ne vérifiait pas. Le div narratif était réduit à son texte décodé, ce qui privait `narrative-absent` des éléments `<img>`. La forme attribut/élément d'un élément n'était pas contrôlée.

## Goals / Non-Goals

**Goals:**
- Rejeter au parsing ce que xml.html interdit, avec un message et une position.
- Conserver le div tel qu'écrit pour qu'il soit analysé comme en JSON.
- Contrôler la forme attribut/élément là où le chemin d'instance est connu.

**Non-Goals:**
- Valider le contenu XHTML du div au-delà du namespace et de la profondeur.

## Decisions

- Namespaces via le `NsReader` de quick-xml ; contrôles regroupés dans `xml_checks.rs`.
- Div : tranche de la source entre les positions de début et de fin de l'élément, sans réallocation ni décodage.
- Blancs d'attribut : le parseur épure et enregistre (position d'élément, nom) dans `Resource::trimmed_attributes` ; le validateur émet `TYPE_006` (Warning) au remodelage, seul endroit où le chemin d'instance est connu.
- Forme attribut/élément : `ElementDefinition.xml_attr` lu depuis `representation` ; contrôle dans `xml_shape.rs` (`STRUCTURE_001`).
- Limites : 8 déclarations de namespace par élément, profondeur 128 y compris dans le div, pour borner le coût.

## Risks / Trade-offs

- Des documents auparavant acceptés sont rejetés → conforme à xml.html, visible dans le corpus de tests officiel.
- Le div brut garde le balisage original : les consommateurs qui attendaient du texte doivent l'analyser.
