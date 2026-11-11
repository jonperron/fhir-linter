## Context

`TYPE_006` couvrait à la fois les caractères de contrôle (#68) et les valeurs d'attribut XML épurées. Le namespace XMLSchema-instance faisait échouer le parse, ce qui masquait tout autre diagnostic de la ressource. Le texte brut du div narratif échappait à la règle XML 1.0 des références de caractère de contrôle.

## Goals / Non-Goals

**Goals:**
- Un code par règle : `TYPE_006` (contrôle), `TYPE_007` (épuration).
- Signaler le namespace schema instance comme diagnostic (`STRUCTURE_003`) plutôt que comme erreur fatale (décision utilisateur).
- Rejeter `&#3;` / `&#x1;` dans le texte du div en XML 1.0.

**Non-Goals:**
- Vérifier les références de caractère dans les attributs XHTML du div ou de la racine (limite connue).

## Decisions

- Le parseur enregistre un `Span` par élément concerné dans `Resource::schema_instance` (dédupliqué par élément), le validateur (`xml_shape.rs`) le transforme en `STRUCTURE_003`. Le parseur reste sans diagnostic, comme pour `trimmed_attributes`.
- `path` = type de ressource : l'élément n'a pas toujours de chemin d'instance utile (racine), la localisation suffit.
- Les attributs `xsi:*` ne portent aucune donnée : ils sont exclus des contrôles d'attributs de la racine et d'attributs dans un namespace.

## Risks / Trade-offs

- Changement de code `TYPE_006` → `TYPE_007` pour les valeurs épurées : les filtres utilisateurs existants doivent être adaptés.
- Un document xsi est désormais parsé : les consommateurs de `parse_xml` seul ne voient plus d'erreur sans passer par `validate`.
