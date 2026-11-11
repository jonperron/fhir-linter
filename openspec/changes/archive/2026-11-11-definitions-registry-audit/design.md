## Context

Le registry décide l'appartenance d'un code à un ValueSet depuis les définitions chargées et ne doit jamais affirmer ce qu'il ne peut pas prouver.

## Goals / Non-Goals

**Goals:** ne rien décider à tort (concepts non sélectionnables, versions différentes) ; ne perdre aucun élément de snapshot en silence.

**Non-Goals:** charger plusieurs versions d'un même CodeSystem ; exploiter la hiérarchie `baseDefinition`.

## Decisions

- `base_definition` est retiré plutôt que conservé : aucun consommateur, et le snapshot est déjà complet.
- Un concept est non sélectionnable si une `property` de code `notSelectable` ou `abstract` porte `valueBoolean: true` ; il est collecté dans `CodeSystem.not_selectable` à l'aplatissement des concepts. Un tel code donne `None` plutôt que `Some(false)` : son usage dépend du contexte.
- Version d'include : le CodeSystem chargé n'est retenu que si `include.version` est absente, vaut `*` ou égale `CodeSystem.version`. Sinon il est traité comme non chargé : un include sur le système entier donne `None`, un include énumérant ses codes décide d'après sa liste.
- Un élément sans `path` ou `max` produit `Error::MalformedBundle` (fichier, URL de la StructureDefinition, index ou path de l'élément). Une StructureDefinition sans `url`/`name`/`kind` reste ignorée, comme avant.

## Risks / Trade-offs

- Un bundle tiers mal formé, auparavant toléré, fait désormais échouer le chargement : voulu, pour ne pas valider contre un snapshot tronqué.
- Plus de résultats `None` pour les ValueSets versionnés : moins de décisions, mais aucune fausse.
