## MODIFIED Requirements

### Requirement: Parse XML

Le système SHALL exposer une fonction `parse_xml(source: &str) -> Result<Resource, ParseError>` qui convertit une chaîne XML FHIR en `Resource`.

Un caractère littéral inférieur à U+0020 autre que tabulation, LF et CR SHALL rendre le document non bien formé (`Err(ParseError::XmlError)`), où qu'il se trouve, en XML 1.0 comme en XML 1.1. Dans un document XML 1.0 (version déclarée ou implicite), un tel caractère obtenu par référence de caractère (ex. `&#1;`) dans une valeur capturée SHALL également rendre le document non bien formé ; en XML 1.1 ces références SHALL être acceptées (le validateur émet alors `TYPE_006`). Limite connue : les références de caractère dans les attributs du `div` XHTML et de l'élément racine ne sont pas vérifiées.

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

#### Scenario: Référence de caractère de contrôle en XML 1.1

- **WHEN** `parse_xml` reçoit un document `<?xml version="1.1"?>` dont une valeur contient `&#1;`
- **THEN** elle SHALL retourner `Ok(Resource)` et la valeur SHALL contenir U+0001

#### Scenario: Fixture officielle unicode-problem.xml

- **WHEN** la fixture `unicode-problem.xml` est parsée
- **THEN** le parse SHALL échouer (erreur fatale)

### Requirement: Profondeur maximale

Le parser XML SHALL rejeter un document dont les éléments sont imbriqués sur plus de 128 niveaux.

#### Scenario: Imbrication trop profonde

- **WHEN** `parse_xml` reçoit des éléments imbriqués sur plus de 128 niveaux
- **THEN** il SHALL retourner `Err(ParseError::XmlError)` sans débordement de pile
