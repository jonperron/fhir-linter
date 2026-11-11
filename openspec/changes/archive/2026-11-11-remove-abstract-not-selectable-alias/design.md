## Context

`CodeSystem.not_selectable` alimente `Registry::value_set_contains`, qui renvoie `None` pour un concept non sélectionnable. L'audit précédent reconnaissait `notSelectable` et `abstract` par leur code, sans consulter `CodeSystem.property`.

## Goals / Non-Goals

**Goals:**
- Identifier la propriété notSelectable par son URI canonique, conformément à R5.
- Supprimer l'alias `abstract`, absent de R5.

**Non-Goals:**
- Interpréter d'autres propriétés de concept (`status`, `deprecated`, etc.).

## Decisions

- **Résolution par URI** : le chargeur lit `CodeSystem.property`, retient les codes locaux dont `uri` vaut `http://hl7.org/fhir/concept-properties#notSelectable`, puis teste ces codes sur les concepts, à tout niveau.
- **Repli** : si aucune propriété déclarée ne porte cette URI, le code `notSelectable` est utilisé, car de nombreux CodeSystems l'emploient sans le déclarer.
- **Pas d'alias `abstract`** : un code `abstract` n'est reconnu que s'il est déclaré avec l'URI ci-dessus.

## Risks / Trade-offs

- [Un CodeSystem déclare l'URI sous un code local et utilise aussi `notSelectable` non déclaré] → seul le code local compte ; conforme à R5, la déclaration fait foi.
