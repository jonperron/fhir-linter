## MODIFIED Requirements

### Requirement: Type Diagnostic

`Diagnostic` SHALL être immuable avec les attributs `severity` (`error`, `warning`, `information`, `hint`), `code`, `message`, `path`, `line`, `col`, `offset` (`None` sans localisation) et `rule` (`None` pour le validateur). `col` (1-indexée) et `offset` (0-indexé) SHALL être des comptes d'octets UTF-8 de l'entrée telle que fournie (BOM inclus), et non des indices de caractères Python : pour retrouver la position, il faut indexer `resource_json.encode()`.

#### Scenario: Offset en octets UTF-8

- **WHEN** un Diagnostic localisé est produit pour une source contenant des caractères non ASCII avant la position signalée
- **THEN** `resource_json.encode()[d.offset:]` SHALL commencer au nœud signalé
