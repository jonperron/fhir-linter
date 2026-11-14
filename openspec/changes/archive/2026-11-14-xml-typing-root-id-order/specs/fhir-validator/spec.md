## MODIFIED Requirements

### Requirement: Validation structurelle — ordre des éléments XML (STRUCTURE_004)

Pour une ressource XML uniquement, le système SHALL émettre un `Diagnostic` avec code `STRUCTURE_004` et severity `Error` pour chaque élément enfant écrit après un frère que le snapshot de la StructureDefinition de base liste plus tard (xml.html : « Elements must always appear in the order documented »). Le `path` SHALL être le chemin d'instance de l'élément fautif et la localisation sa position dans la source. Les frères SHALL être comparés par offset du span source (le parseur regroupe les frères répétés en tableaux). Une variante de choix `[x]` prend la position de son élément `[x]`. Tout frère situé après un frère listé plus tard SHALL être signalé (`gender, active, name` : `active` et `name`). La règle s'applique à toute profondeur (éléments backbone, types complexes, enfants d'extension de primitif, ressources contenues ou embarquées). Les attributs n'ont pas d'ordre : un élément `xmlAttr` écrit comme élément enfant reçoit seulement `STRUCTURE_001`, sans `STRUCTURE_004`. L'`<id>` simple de la racine, remonté par le parseur dans `Resource.id`, SHALL faire partie du contrôle d'ordre racine avec le span de son élément. Les positions du snapshot SHALL venir de l'index de chemins de la StructureDefinition (`StructureDefinition::position`). Une ressource JSON SHALL NOT produire ce diagnostic.

#### Scenario: Éléments dans le mauvais ordre

- **WHEN** `validate` reçoit un `Patient` XML avec `<gender/>` puis `<active/>`
- **THEN** un Diagnostic `STRUCTURE_004` SHALL être émis au chemin `Patient.active` avec sa localisation

#### Scenario: Frère répété intercalé

- **WHEN** un `Patient` XML contient `<name>` avec `<given/><family/><given/>`
- **THEN** un seul `STRUCTURE_004` SHALL être émis, au chemin `Patient.name.family`

#### Scenario: Id racine hors ordre

- **WHEN** `validate` reçoit `<Patient><active value="true"/><id value="p"/></Patient>`
- **THEN** un Diagnostic `STRUCTURE_004` SHALL être émis au chemin `Patient.id`, localisé sur l'élément `<id>`

#### Scenario: Ordre correct

- **WHEN** un `Patient` XML a ses éléments dans l'ordre du snapshot
- **THEN** aucun `STRUCTURE_004` ne SHALL être émis

#### Scenario: Attribut écrit comme élément

- **WHEN** un élément `xmlAttr` est écrit comme élément enfant
- **THEN** seul un `STRUCTURE_001` SHALL être émis, sans `STRUCTURE_004`
