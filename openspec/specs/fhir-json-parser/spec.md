# fhir-json-parser Specification

## Purpose
TBD - created by archiving change fhir-parser-phase2. Update Purpose after archive.
## Requirements
### Requirement: Type Resource

Le système SHALL exposer un type `Resource` représentant une ressource FHIR parsée depuis JSON.

#### Scenario: Champs obligatoires

- **WHEN** une ressource FHIR JSON est parsée avec succès
- **THEN** le `Resource` retourné SHALL contenir `resource_type: Arc<str>` (valeur du champ `"resourceType"`), `id: Option<Arc<str>>` (valeur du champ `"id"` si présent), et `fields: IndexMap<Arc<str>, Node>` (tous les champs restants)

---

### Requirement: Type Node

Le système SHALL exposer un type `Node` associant une valeur FHIR à une position source.

#### Scenario: Composition d'un Node

- **WHEN** un champ JSON est parsé
- **THEN** le `Node` résultant SHALL contenir `value: Value` (la valeur) et `span: Span` (la position dans la source)

---

### Requirement: Type Value

Le système SHALL exposer un enum `Value` représentant les types JSON possibles dans une ressource FHIR.

#### Scenario: Variantes couvertes

- **WHEN** un nœud JSON est parsé
- **THEN** `Value` SHALL avoir les variantes : `Null`, `Bool(bool)`, `Integer(i64)`, `Decimal(f64)`, `Str(Arc<str>)`, `Array(Vec<Node>)`, `Object(IndexMap<Arc<str>, Node>)`

---

### Requirement: Type Span

Le système SHALL exposer un type `Span` décrivant la position d'un nœud dans la source texte. `col` et `offset` SHALL être des comptes d'octets UTF-8 de l'entrée telle que fournie (BOM inclus), et non des comptes de caractères.

#### Scenario: Champs d'un Span

- **WHEN** un nœud est localisé dans la source
- **THEN** le `Span` SHALL contenir `line: u32` (ligne 1-indexée), `col: u32` (colonne 1-indexée, en octets UTF-8 depuis le début de la ligne), `offset: u32` (offset 0-indexé, en octets UTF-8 depuis le début de l'entrée)

#### Scenario: Caractères multi-octets avant un nœud

- **WHEN** un caractère `ü` (2 octets UTF-8) précède un nœud sur sa ligne
- **THEN** `col` et `offset` du nœud SHALL compter ce caractère pour 2

---

### Requirement: Parse JSON

Le système SHALL exposer une fonction `parse_json(source: &str) -> Result<Resource, ParseError>` qui convertit une chaîne JSON en `Resource`. Un unique BOM UTF-8 (U+FEFF) en tête de la source SHALL être ignoré ; les spans SHALL continuer à compter ses 3 octets.

#### Scenario: Ressource valide

- **WHEN** `parse_json` reçoit une ressource FHIR JSON valide avec un champ `"resourceType"`
- **THEN** elle SHALL retourner `Ok(Resource)` avec `resource_type` égal à la valeur du champ `"resourceType"`

#### Scenario: JSON malformé

- **WHEN** `parse_json` reçoit une chaîne JSON syntaxiquement invalide
- **THEN** elle SHALL retourner `Err(ParseError)` contenant un message et une position (ligne, colonne)

#### Scenario: resourceType manquant

- **WHEN** `parse_json` reçoit un objet JSON valide sans champ `"resourceType"`
- **THEN** elle SHALL retourner `Err(ParseError::MissingResourceType)`

#### Scenario: Clé d'objet dupliquée

- **WHEN** `parse_json` reçoit un objet contenant deux fois la même clé
- **THEN** elle SHALL retourner `Err(ParseError::JsonError)`

#### Scenario: Imbrication trop profonde

- **WHEN** `parse_json` reçoit des objets/tableaux imbriqués sur plus de 128 niveaux
- **THEN** elle SHALL retourner `Err(ParseError::JsonError)` sans débordement de pile

#### Scenario: BOM UTF-8 initial

- **WHEN** `parse_json` reçoit `"\u{FEFF}{\"resourceType\":\"Patient\",\"active\":true}"`
- **THEN** elle SHALL retourner `Ok(Resource)` de type `Patient` et le `Span.offset` de `active` SHALL être l'offset en octets de `true` dans la source, BOM compris

#### Scenario: BOM final ou second BOM

- **WHEN** `parse_json` reçoit une source avec un BOM après le document, ou deux BOM en tête
- **THEN** elle SHALL retourner `Err(ParseError)`

---

### Requirement: Conversion des nombres JSON

Le système SHALL convertir chaque nombre JSON en `Value` selon sa forme lexicale : un nombre sans `.` ni exposant représentable sur 64 bits signés, autre que `-0`, SHALL devenir `Value::Integer` ; tout autre nombre fini SHALL devenir `Value::Decimal(f64)` (arrondi à la précision `f64`). Le littéral `-0` SHALL devenir `Value::Decimal(-0.0)`, la regex R5 `integer` (`[0]|[-+]?[1-9][0-9]*`) l'excluant. Un nombre dont la conversion en `f64` n'est pas finie SHALL être rejeté.

#### Scenario: Entier dans la plage i64

- **WHEN** `parse_json` rencontre le nombre `42`
- **THEN** la valeur SHALL être `Value::Integer(42)`

#### Scenario: Moins zéro

- **WHEN** `parse_json` rencontre le nombre `-0`
- **THEN** la valeur SHALL être `Value::Decimal(-0.0)` (zéro de signe négatif)

#### Scenario: Entier hors de la plage i64

- **WHEN** `parse_json` rencontre un entier littéral dépassant i64, par exemple `12345678901234567890`
- **THEN** la valeur SHALL être `Value::Decimal` (arrondie à la précision `f64`) et le parsing SHALL réussir

#### Scenario: Nombre avec partie décimale ou exposant

- **WHEN** `parse_json` rencontre un nombre contenant `.` ou un exposant, par exemple `1.5` ou `1e2`
- **THEN** la valeur SHALL être `Value::Decimal`

#### Scenario: Sous-dépassement

- **WHEN** `parse_json` rencontre le nombre `1e-400`
- **THEN** la valeur SHALL être `Value::Decimal(0.0)`

#### Scenario: Nombre hors plage

- **WHEN** `parse_json` rencontre un nombre dont la valeur `f64` n'est pas finie, par exemple `1e400`, `-1e400` ou un entier de 400 chiffres
- **THEN** elle SHALL retourner `Err(ParseError::JsonError)` avec un message `number out of range: ...`

### Requirement: Positions sources précises

Le système SHALL associer une position source à chaque `Node` lors du parse JSON.

#### Scenario: Ligne correcte pour un champ

- **WHEN** un champ est présent à la ligne N de la source JSON
- **THEN** le `Span` du `Node` correspondant SHALL avoir `line == N`

### Requirement: Erreurs de parse XML

Le système SHALL exposer la variante `ParseError::XmlError { message: String, line: u32, col: u32 }` pour signaler toute erreur rencontrée lors du parse XML.

#### Scenario: XML invalide

- **WHEN** `parse_xml` rencontre du XML syntaxiquement invalide ou un document sans élément racine
- **THEN** elle SHALL retourner `Err(ParseError::XmlError)` contenant un `message` descriptif ainsi que les coordonnées `line` et `col` (1-indexées) pointant vers l'erreur dans la source

