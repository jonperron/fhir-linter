## ADDED Requirements

### Requirement: Conformité XML FHIR

`parse_xml` SHALL retourner `Err(ParseError::XmlError)` pour tout document que la représentation XML FHIR (xml.html) interdit :

- une DTD (`<!DOCTYPE ...>`) ;
- une déclaration XML dont l'`encoding` n'est pas UTF-8 (comparaison insensible à la casse) ;
- du texte non blanc, une référence d'entité ou une section CDATA entre éléments FHIR (les commentaires et instructions de traitement SHALL être ignorés) ;
- tout attribut de la racine de la ressource autre qu'une déclaration de namespace, le message SHALL nommer tous les attributs fautifs ;
- un attribut et un élément enfant de même nom sur un même élément ;
- un élément hors du namespace `http://hl7.org/fhir` (un préfixe lié à ce namespace SHALL être accepté) ;
- le div narratif, ou tout élément qu'il contient, hors du namespace XHTML `http://www.w3.org/1999/xhtml` ;
- un attribut dans un namespace sur un élément FHIR ;
- toute déclaration du namespace XMLSchema-instance (`http://www.w3.org/2001/XMLSchema-instance`), comparée sur la valeur normalisée ;
- plus de 8 déclarations de namespace sur un même élément.

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

- **WHEN** un élément FHIR porte un attribut préfixé lié à un namespace (ex. `x:value`)
- **THEN** `parse_xml` SHALL retourner `Err(ParseError::XmlError)`

#### Scenario: Namespace XMLSchema-instance

- **WHEN** un élément déclare le namespace XMLSchema-instance, y compris sous une forme non normalisée de la même valeur
- **THEN** `parse_xml` SHALL retourner `Err(ParseError::XmlError)`

#### Scenario: Trop de déclarations de namespace

- **WHEN** un élément porte 9 déclarations de namespace
- **THEN** `parse_xml` SHALL retourner `Err(ParseError::XmlError)`

### Requirement: Épuration des valeurs d'attribut

`parse_xml` SHALL retirer des valeurs d'attribut les blancs XML (espace, tabulation, CR, LF) en tête et en fin. Chaque attribut ainsi modifié SHALL être enregistré dans `Resource::trimmed_attributes` sous la forme (position en octets de l'élément, nom de l'attribut). Pour une ressource issue du JSON, `trimmed_attributes` SHALL être vide.

#### Scenario: Valeur épurée

- **WHEN** `parse_xml` reçoit `<active value=" true "/>`
- **THEN** `active` SHALL valoir `Value::Str("true")` et `trimmed_attributes` SHALL contenir la position de `<active>` avec le nom `value`

#### Scenario: Valeur sans blanc

- **WHEN** aucune valeur d'attribut n'a de blanc en tête ou en fin
- **THEN** `trimmed_attributes` SHALL être vide

#### Scenario: Ressource JSON

- **WHEN** une ressource est produite par `parse_json`
- **THEN** `trimmed_attributes` SHALL être vide

## MODIFIED Requirements

### Requirement: Contenu XHTML

Le système SHALL capturer l'élément `<div>` XHTML comme `Value::Str` contenant sa tranche de source brute, du `<div` ouvrant au `</div>` fermant inclus (ou l'élément `<div .../>` auto-fermant), sans décodage.

#### Scenario: Élément div XHTML

- **WHEN** un élément `<div>` (namespace XHTML) est rencontré
- **THEN** le champ `div` SHALL être `Value::Str` contenant le balisage source `<div ...>...</div>` tel qu'écrit, et les éléments enfants de `<div>` SHALL être ignorés structurellement

#### Scenario: Div auto-fermant

- **WHEN** le div est écrit `<div xmlns="http://www.w3.org/1999/xhtml"/>`
- **THEN** le champ `div` SHALL être `Value::Str` contenant exactement cette tranche de source

### Requirement: Profondeur maximale

`parse_xml` SHALL refuser une imbrication de plus de 128 niveaux, y compris à l'intérieur du div narratif.

#### Scenario: Imbrication trop profonde

- **WHEN** `parse_xml` reçoit des éléments imbriqués sur plus de 128 niveaux
- **THEN** il SHALL retourner `Err(ParseError::XmlError)` sans débordement de pile

#### Scenario: Imbrication trop profonde dans le div

- **WHEN** le div narratif contient des éléments XHTML imbriqués au-delà de 128 niveaux
- **THEN** `parse_xml` SHALL retourner `Err(ParseError::XmlError)`
