## MODIFIED Requirements

### Requirement: Extraction du champ id

Le système SHALL extraire l'attribut `value` de l'élément racine `<id>` de la ressource XML vers `Resource.id`, de type `Option<ResourceId>` (`value` et `span` de l'élément `<id>` ; déréférence en `str`). Lorsque `<id>` porte aussi une `<extension>` ou un attribut `id`, le système SHALL conserver le reste dans `Resource.fields` sous la clé `id`, comme `Value::Object` sans clé `value`, à la même position et avec le span de l'élément `id`, afin que le reshape XML du validateur le convertisse en `_id` comme le fait JSON.

#### Scenario: Champ id présent et primitif

- **WHEN** la ressource XML contient `<id value="patient-1"/>`
- **THEN** `Resource.id` SHALL être `Some("patient-1")` et `id` ne SHALL pas apparaître dans `Resource.fields`

#### Scenario: Champ id avec extension ou attribut id

- **WHEN** la ressource XML contient `<id value="p1">` avec un élément enfant `<extension>` ou un attribut `id`
- **THEN** `Resource.id` SHALL être `Some("p1")` et `Resource.fields` SHALL conserver `id` comme `Value::Object` sans clé `value`, à la même position, avec le span de l'élément `id`

#### Scenario: Champ id sans attribut value

- **WHEN** la ressource XML contient `<id id="e1"/>`
- **THEN** `Resource.id` SHALL être `None` et l'objet `id` SHALL être conservé dans `Resource.fields`

#### Scenario: Élément enfant value non remonté

- **WHEN** l'élément `<id>` contient un élément enfant `<value>` (et non l'attribut `value`)
- **THEN** cet enfant SHALL rester dans l'objet `id` sans être remonté dans `Resource.id`, afin que le validateur signale STRUCTURE_001

#### Scenario: Champ id vide ou répété

- **WHEN** la ressource XML contient un élément `<id/>` vide ou plusieurs éléments `<id>`
- **THEN** `parse_xml` SHALL retourner `Err(ParseError::XmlError)` avec le message `'id' element must be a primitive string value`
