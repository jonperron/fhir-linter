## ADDED Requirements

### Requirement: Validation de profil — valeur minimale et maximale (PROFILE_011)

Le système SHALL émettre `PROFILE_011` (erreur) pour chaque occurrence d'un élément dont l'élément de profil (ou slice) porte `minValue[x]` ou `maxValue[x]` (R5 ElementDefinition, bornes inclusives) et dont la valeur est inférieure au minimum ou supérieure au maximum, à tout niveau de profondeur, en JSON et en XML. Le `path` SHALL être le chemin d'instance de l'occurrence ; le message SHALL être « Value at '<chemin>' is below the minimum <clé minValueX> <borne JSON> required by '<chemin de l'élément>' » (ou « is above the maximum <clé maxValueX> ... »). Les nombres (integer, decimal, positiveInt, unsignedInt) SHALL être comparés par valeur (`2.50` égale `2.5`) et integer64 (chaînes JSON) comme i64. Les date, dateTime, instant et time SHALL être ordonnés selon FHIRPath (précision par précision, fuseaux ramenés en UTC) ; lorsque les précisions diffèrent et que la valeur chevauche la borne, l'ordre est indéterminé et aucun diagnostic SHALL être émis. Une Quantity SHALL être comparée uniquement si la valeur et la borne ont les mêmes `system` et `code` et aucun comparateur ; sinon aucun diagnostic SHALL être émis. Les unités ne sont jamais converties (UCUM) : limite connue. Une valeur non analysable ou de type incompatible SHALL NOT produire de constat (les contrôles de type la signalent).

#### Scenario: Entier sous le minimum

- **WHEN** le profil porte `minValueInteger` 5 et l'instance vaut 3
- **THEN** un Diagnostic `PROFILE_011` SHALL être émis au chemin de l'occurrence

#### Scenario: Valeur égale à la borne

- **WHEN** la valeur est égale à `minValue[x]` ou `maxValue[x]`
- **THEN** aucun `PROFILE_011` SHALL être émis

#### Scenario: Précision de date chevauchant la borne

- **WHEN** le profil porte `minValueDate` 2000-06-15 et l'instance vaut `2000`
- **THEN** aucun `PROFILE_011` SHALL être émis

#### Scenario: Quantity dans une autre unité

- **WHEN** la Quantity de l'instance et la borne ont un `code` ou un `system` différent
- **THEN** aucun `PROFILE_011` SHALL être émis
