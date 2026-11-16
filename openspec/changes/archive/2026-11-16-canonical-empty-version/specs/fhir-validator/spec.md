## MODIFIED Requirements

### Requirement: Validation de type — format date/heure et expressions régulières R5 (TYPE_003)

Le système SHALL émettre un `Diagnostic` avec code `TYPE_003` et severity `Error` lorsqu'une valeur de type `string`, `markdown`, `base64Binary`, `uri`, `url`, `canonical`, `oid` ou `uuid` ne correspond pas à l'expression régulière R5 du type, lue depuis le Registry et ancrée en `\A(?:...)\z`. Une chaîne vide pour ces types SHALL produire `TYPE_003` (R5 json.html : les valeurs de propriété string ne peuvent jamais être vides ; les regex `uri` et `base64Binary` acceptent `""`). Une valeur non vide composée uniquement de whitespace qui satisfait la regex (en pratique `string`, `markdown` et `base64Binary`, dont le whitespace est ignoré) SHALL produire `TYPE_008` et non `TYPE_003` (voir TYPE_008). Le message `TYPE_003` SHALL être `Invalid <type> format at '<path>': '<value>'`, sauf pour `base64Binary` : `Invalid base64Binary format at '<path>'` (sans la charge utile). Pour `base64Binary`, le whitespace ASCII SHALL être ignoré avant l'application de la regex (R5 datatypes.html, RFC 4648). Une valeur `canonical` qui passe la regex et `maxLength` mais se termine par `|` (version vide après la barre, ex. `http://x.org/SD/x|`, `|`, `#x|`) SHALL produire `TYPE_003` avec le message `Canonical at '<path>' has an empty version after '|': '<value>'` (R5 datatypes.html : la version est ajoutée après `|`, et une version est une chaîne, jamais vide ; cas officiel patient-ig-ig) ; `uri` et `url` terminés par `|` ne sont pas concernés. `xhtml` n'a pas de regex R5 (chaîne uniquement). `decimal`, la famille des dates, les types entiers, `integer64`, `code` et `id` conservent des contrôles écrits à la main (la regex R5 de `decimal` contient une coquille).

Pour les types date/heure, le format SHALL être conforme aux expressions régulières R5 :

- `date` : `YYYY(-MM(-DD)?)?`, année de `0001` à `9999`, jour valide pour le mois (années bissextiles comprises) ;
- `dateTime` : une `date`, ou une date complète suivie de `Thh:mm:ss(.f{1,9})?` et d'un fuseau obligatoire `Z` ou `(+|-)hh:mm` (décalage au plus `14:00`) ; conformément à la regex R5, un fuseau SHALL être accepté sur une valeur réduite à une année-mois ou à une date complète (`2020-01Z`, `2020-01-01+02:00`), mais SHALL être rejeté sur une année seule (`2020Z`) ; une heure sans fuseau SHALL être rejetée ;
- `instant` : toujours la forme complète avec fuseau ;
- `time` : `hh:mm:ss(.f{1,9})?` sans fuseau, heures `00`-`23`, minutes `00`-`59`, secondes `00`-`60`.

#### Scenario: Date invalide

- **WHEN** un champ de type `date` contient la chaîne `"not-a-date"`
- **THEN** un Diagnostic `TYPE_003` SHALL être émis

#### Scenario: Date valide ne produit pas d'erreur

- **WHEN** un champ de type `date` contient `"2024-01-15"`
- **THEN** aucun Diagnostic `TYPE_003` ne SHALL être émis

#### Scenario: Jour inexistant pour le mois

- **WHEN** un champ de type `date` contient `"2020-02-31"`
- **THEN** un Diagnostic `TYPE_003` SHALL être émis

#### Scenario: instant partiel

- **WHEN** un champ de type `instant` contient `"2020"`
- **THEN** un Diagnostic `TYPE_003` SHALL être émis

#### Scenario: dateTime suivi de caractères parasites

- **WHEN** un champ de type `dateTime` contient `"2020-01-01T10:00:00garbage"`
- **THEN** un Diagnostic `TYPE_003` SHALL être émis

#### Scenario: dateTime réduit à la date avec fuseau

- **WHEN** un champ de type `dateTime` contient `"2020-01Z"`, `"2020-01-01Z"` ou `"2020-01-01+02:00"`
- **THEN** aucun Diagnostic `TYPE_003` ne SHALL être émis

#### Scenario: dateTime réduit à l'année avec fuseau

- **WHEN** un champ de type `dateTime` contient `"2020Z"`
- **THEN** un Diagnostic `TYPE_003` SHALL être émis

#### Scenario: uri avec espace

- **WHEN** un champ de type `uri` contient `"http://a b"`
- **THEN** un Diagnostic `TYPE_003` SHALL être émis

#### Scenario: oid valide

- **WHEN** un champ de type `oid` contient `"urn:oid:1.2.3"`
- **THEN** aucun Diagnostic `TYPE_003` ne SHALL être émis

#### Scenario: oid invalide

- **WHEN** un champ de type `oid` contient `"1.2.3"`
- **THEN** un Diagnostic `TYPE_003` SHALL être émis

#### Scenario: string vide

- **WHEN** un champ de type `string` contient `""`
- **THEN** un Diagnostic `TYPE_003` SHALL être émis

#### Scenario: string whitespace seul

- **WHEN** un champ de type `string` contient `"   "`
- **THEN** un Diagnostic `TYPE_008` de severity `Warning` SHALL être émis et aucun `TYPE_003`

#### Scenario: Message base64Binary

- **WHEN** un champ de type `base64Binary` contient `"not base64!"`
- **THEN** le message `TYPE_003` SHALL être `Invalid base64Binary format at '<path>'`, sans la valeur

#### Scenario: base64Binary avec whitespace ignoré

- **WHEN** un champ de type `base64Binary` contient `"aGVs bG8="` ou une valeur base64 coupée par des sauts de ligne
- **THEN** aucun Diagnostic `TYPE_003` ne SHALL être émis

#### Scenario: base64Binary invalide

- **WHEN** un champ de type `base64Binary` contient `"not base64!"`
- **THEN** un Diagnostic `TYPE_003` SHALL être émis

#### Scenario: canonical avec version vide

- **WHEN** un champ de type `canonical` contient `"http://x.org/SD/x|"`
- **THEN** un Diagnostic `TYPE_003` SHALL être émis avec le message `Canonical at '<path>' has an empty version after '|': 'http://x.org/SD/x|'`
- **AND** un `uri` ou `url` de même valeur ne SHALL produire aucun Diagnostic `TYPE_003`

