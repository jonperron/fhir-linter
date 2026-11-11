## MODIFIED Requirements

### Requirement: Validation de type — format date/heure (TYPE_003)

Le système SHALL émettre un `Diagnostic` avec code `TYPE_003` et severity `Error` lorsqu'une valeur de type `date`, `dateTime`, `instant` ou `time` ne respecte pas le format attendu, conforme aux expressions régulières R5 :

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

### Requirement: Validation de type — whitespace dans un code (TYPE_004)

Le système SHALL émettre un `Diagnostic` avec code `TYPE_004` et severity `Error` lorsqu'un champ de type `code` ne respecte pas la regex R5 `[^\s]+( [^\s]+)*` : des mots non vides séparés par un unique espace. Une chaîne vide, un espace en début ou en fin, deux espaces consécutifs, ou tout whitespace autre qu'un espace simple (tabulation, saut de ligne, espace insécable...) SHALL produire un `TYPE_004`.

#### Scenario: Code avec espaces simples entre mots

- **WHEN** un champ de type `code` contient `"text/plain; charset=utf-8"` ou `"a b c"`
- **THEN** aucun Diagnostic `TYPE_004` ne SHALL être émis

#### Scenario: Code avec espace mal placé

- **WHEN** un champ de type `code` contient `"a  b"` (deux espaces) ou `" a"` (espace initial)
- **THEN** un Diagnostic `TYPE_004` SHALL être émis

#### Scenario: Code sans whitespace

- **WHEN** un champ de type `code` contient `"final"`
- **THEN** aucun Diagnostic `TYPE_004` ne SHALL être émis
