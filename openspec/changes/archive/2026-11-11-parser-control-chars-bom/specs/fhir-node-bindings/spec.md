## MODIFIED Requirements

### Requirement: Type Diagnostic

`Diagnostic` SHALL être un objet avec `severity` (`error`, `warning`, `information`, `hint`), `code`, `message`, `path`, `line`, `col`, `offset` (absents sans localisation) et `rule` (absent pour le validateur). `col` (1-indexée) et `offset` (0-indexé) SHALL être des comptes d'octets UTF-8 de l'entrée telle que fournie (BOM inclus), et non des indices UTF-16 de chaîne JavaScript : pour retrouver la position, il faut indexer `Buffer.from(json)`.

#### Scenario: Diagnostic du validateur

- **WHEN** `validate` retourne un finding sans localisation
- **THEN** `line`, `col`, `offset` et `rule` SHALL être absents du Diagnostic

#### Scenario: Offset en octets UTF-8

- **WHEN** un Diagnostic localisé est produit pour une source contenant des caractères non ASCII avant la position signalée
- **THEN** `Buffer.from(json).subarray(d.offset)` SHALL commencer au nœud signalé
