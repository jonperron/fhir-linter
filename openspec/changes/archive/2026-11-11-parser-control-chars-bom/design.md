## Context

Les fixtures officielles `unicode-problem.xml`, `unicode-problem.json` et `xml_v11.xml` attendent respectivement une erreur fatale, un warning et un warning. FHIR R5 (datatypes.html) indique qu'un `string` SHOULD NOT contenir de caractère sous U+0020 hors tabulation, CR, LF ; la regex `integer` `[0]|[-+]?[1-9][0-9]*` exclut `-0`.

## Goals / Non-Goals

**Goals:**
- Aligner le parse JSON/XML et la validation de type sur ces règles.
- Documenter les unités de localisation (octets UTF-8).

**Non-Goals:**
- Vérifier les références de caractère dans les attributs du `div` XHTML et de l'élément racine.
- Convertir les positions en caractères ou en unités UTF-16.

## Decisions

- **Prédicat partagé** : `fhir_parser::is_disallowed_control` sert au parseur XML et au validateur, pour une seule définition du caractère interdit.
- **`-0` en décimal** : le parseur garde la forme lexicale ; `TYPE_001` découle de la règle existante « entier attendu, décimal reçu ».
- **BOM** : seul le premier caractère est sauté ; `Span.offset` reste un offset dans l'entrée telle que fournie, pour pointer vers le texte original.
- **XML** : un contrôle littéral est rejeté avant le parse (non bien formé en 1.0, `RestrictedChar` en 1.1). En XML 1.0 (déclaré ou implicite), les valeurs capturées sont ensuite parcourues pour rejeter les références de caractère ; en XML 1.1 elles sont acceptées et le validateur émet `TYPE_006`.
- **Localisations** : décision utilisateur, `col` et `offset` sont des octets UTF-8 ; les bindings indiquent comment indexer (`resource_json.encode()`, `Buffer.from(json)`).

## Risks / Trade-offs

- [Références dans les attributs XHTML/racine non vérifiées] → limite connue ; à vérifier dans le reader si nécessaire.
- [Erreur XML 1.0 sur l'`id` racine localisée en 1:1] → l'`id` n'a pas de span propre.
