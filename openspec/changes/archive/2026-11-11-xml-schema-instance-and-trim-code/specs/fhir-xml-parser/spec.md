## ADDED Requirements

### Requirement: Namespace schema instance XML

`parse_xml` SHALL NOT échouer sur le namespace XMLSchema-instance. Chaque élément qui déclare un namespace dont la valeur normalisée est `http://www.w3.org/2001/XMLSchema-instance`, ou qui porte un attribut lié à ce namespace (ex. `xsi:schemaLocation`, `xsi:type`), SHALL être enregistré une seule fois, sous la forme du `Span` de l'élément, dans `Resource::schema_instance: Vec<Span>`. Ces attributs ne portent aucune donnée et SHALL NOT produire d'erreur d'attribut de la racine ni d'attribut dans un namespace. Pour une ressource issue du JSON, `schema_instance` SHALL être vide. Le validateur signale chaque élément enregistré par `STRUCTURE_003`.

#### Scenario: Déclaration et schemaLocation sur la racine

- **WHEN** `parse_xml` reçoit `<Patient xmlns="http://hl7.org/fhir" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="...">`
- **THEN** il SHALL retourner `Ok(Resource)` et `schema_instance` SHALL contenir exactement un `Span`, celui de l'élément racine

#### Scenario: Forme non normalisée

- **WHEN** un élément déclare le namespace XMLSchema-instance sous une forme non normalisée de la même valeur
- **THEN** l'élément SHALL être enregistré dans `schema_instance`

#### Scenario: Ressource JSON

- **WHEN** une ressource est produite par `parse_json`
- **THEN** `schema_instance` SHALL être vide

## MODIFIED Requirements

### Requirement: Parse XML

Le système SHALL exposer une fonction `parse_xml(source: &str) -> Result<Resource, ParseError>` qui convertit une chaîne XML FHIR en `Resource`.

Un caractère littéral inférieur à U+0020 autre que tabulation, LF et CR SHALL rendre le document non bien formé (`Err(ParseError::XmlError)`), où qu'il se trouve, en XML 1.0 comme en XML 1.1. Dans un document XML 1.0 (version déclarée ou implicite), un tel caractère obtenu par référence de caractère (ex. `&#1;`) dans une valeur capturée ou dans le texte brut du div narratif (ex. `&#3;`, `&#x1;`) SHALL également rendre le document non bien formé ; en XML 1.1 ces références SHALL être acceptées (le validateur émet alors `TYPE_006` pour les valeurs). Limite connue : les références de caractère dans les attributs XHTML du div et de l'élément racine ne sont pas vérifiées.

#### Scenario: Ressource valide avec champs primitifs

- **WHEN** `parse_xml` reçoit une ressource FHIR XML valide (ex. `<Patient><status value="active"/></Patient>`)
- **THEN** elle SHALL retourner `Ok(Resource)` avec `resource_type == "Patient"` et le champ `status` dont la valeur est `Value::Str("active")`

#### Scenario: XML malformé (EOF inattendu)

- **WHEN** `parse_xml` reçoit une chaîne XML où un élément n'est pas fermé
- **THEN** elle SHALL retourner `Err(ParseError::XmlError)` avec un message décrivant l'erreur et la position (ligne, colonne)

#### Scenario: Document sans élément racine

- **WHEN** `parse_xml` reçoit une chaîne vide ou sans élément racine
- **THEN** elle SHALL retourner `Err(ParseError::XmlError)` avec le message `"document contains no root element"`

#### Scenario: Caractère de contrôle littéral

- **WHEN** `parse_xml` reçoit un document (XML 1.0 ou 1.1) contenant un caractère U+0001 littéral, dans une valeur, un commentaire ou ailleurs
- **THEN** elle SHALL retourner `Err(ParseError::XmlError)` positionnée sur ce caractère

#### Scenario: Référence de caractère de contrôle en XML 1.0

- **WHEN** `parse_xml` reçoit un document sans déclaration ou déclaré `version="1.0"` dont une valeur capturée contient `&#1;`
- **THEN** elle SHALL retourner `Err(ParseError::XmlError)`

#### Scenario: Référence de caractère de contrôle dans le div en XML 1.0

- **WHEN** `parse_xml` reçoit un document XML 1.0 dont le texte du div narratif contient `&#3;` ou `&#x1;`
- **THEN** elle SHALL retourner `Err(ParseError::XmlError)`, alors que le même div sous une déclaration `<?xml version="1.1"?>` SHALL être accepté

#### Scenario: Référence de caractère de contrôle en XML 1.1

- **WHEN** `parse_xml` reçoit un document `<?xml version="1.1"?>` dont une valeur contient `&#1;`
- **THEN** elle SHALL retourner `Ok(Resource)` et la valeur SHALL contenir U+0001

#### Scenario: Fixture officielle unicode-problem.xml

- **WHEN** la fixture `unicode-problem.xml` est parsée
- **THEN** le parse SHALL échouer (erreur fatale)

### Requirement: Conformité XML FHIR

`parse_xml` SHALL retourner `Err(ParseError::XmlError)` pour tout document que la représentation XML FHIR (xml.html) interdit :

- une DTD (`<!DOCTYPE ...>`) ;
- une déclaration XML dont l'`encoding` n'est pas UTF-8 (comparaison insensible à la casse) ;
- du texte non blanc, une référence d'entité ou une section CDATA entre éléments FHIR (les commentaires et instructions de traitement SHALL être ignorés) ;
- tout attribut de la racine de la ressource autre qu'une déclaration de namespace ou un attribut lié au namespace XMLSchema-instance, le message SHALL nommer tous les attributs fautifs ;
- un attribut et un élément enfant de même nom sur un même élément ;
- un élément hors du namespace `http://hl7.org/fhir` (un préfixe lié à ce namespace SHALL être accepté) ;
- le div narratif, ou tout élément qu'il contient, hors du namespace XHTML `http://www.w3.org/1999/xhtml` ;
- un attribut dans un namespace sur un élément FHIR, hors namespace XMLSchema-instance ;
- plus de 8 déclarations de namespace sur un même élément.

Le namespace XMLSchema-instance n'est pas une erreur de parse : il est enregistré dans `Resource::schema_instance` (voir « Namespace schema instance XML »).

#### Scenario: DTD refusée

- **WHEN** `parse_xml` reçoit un document contenant `<!DOCTYPE Patient>`
- **THEN** il SHALL retourner `Err(ParseError::XmlError)`

#### Scenario: Encodage non UTF-8

- **WHEN** la déclaration XML porte `encoding="ISO-8859-1"`
- **THEN** `parse_xml` SHALL retourner `Err(ParseError::XmlError)`, alors que `encoding="utf-8"` SHALL être accepté

#### Scenario: Texte entre éléments

- **WHEN** du texte non blanc, une référence d'entité ou une section CDATA apparaît entre deux éléments FHIR
- **THEN** `parse_xml` SHALL retourner `Err(ParseError::XmlError)`, alors qu'un commentaire ou une instruction de traitement au même endroit SHALL être ignoré

#### Scenario: Attribut sur la racine

- **WHEN** l'élément racine porte `id="a"` et `lang="en"` en plus de `xmlns`
- **THEN** `parse_xml` SHALL retourner `Err(ParseError::XmlError)` dont le message nomme `id` et `lang`

#### Scenario: Attribut et élément de même nom

- **WHEN** un élément porte un attribut `value` et un élément enfant `<value>`
- **THEN** `parse_xml` SHALL retourner `Err(ParseError::XmlError)`

#### Scenario: Namespace FHIR

- **WHEN** un élément FHIR n'est pas dans `http://hl7.org/fhir`
- **THEN** `parse_xml` SHALL retourner `Err(ParseError::XmlError)`, alors que `<f:Patient xmlns:f="http://hl7.org/fhir">` SHALL être accepté

#### Scenario: Namespace XHTML du div

- **WHEN** le div narratif ou un élément qu'il contient n'est pas dans le namespace XHTML
- **THEN** `parse_xml` SHALL retourner `Err(ParseError::XmlError)`

#### Scenario: Attribut dans un namespace

- **WHEN** un élément FHIR porte un attribut préfixé lié à un namespace autre que XMLSchema-instance (ex. `x:value`)
- **THEN** `parse_xml` SHALL retourner `Err(ParseError::XmlError)`

#### Scenario: Namespace XMLSchema-instance

- **WHEN** un élément déclare le namespace XMLSchema-instance ou porte `xsi:schemaLocation` / `xsi:type`
- **THEN** `parse_xml` SHALL retourner `Ok(Resource)` et l'élément SHALL figurer dans `schema_instance`

#### Scenario: Trop de déclarations de namespace

- **WHEN** un élément porte 9 déclarations de namespace
- **THEN** `parse_xml` SHALL retourner `Err(ParseError::XmlError)`
