## MODIFIED Requirements

### Requirement: Validation des références — type inconnu (REFERENCE_001)

Le système SHALL émettre `REFERENCE_001` (erreur) lorsque `Reference.reference` d'un élément de premier niveau de type `Reference` est une référence relative (sans schéma URI au sens de RFC 3986 ; un `:` hors schéma de tête, comme dans `Foo/a:b`, ne rend pas la référence absolue) dont le type n'est pas un type de ressource concret connu du Registry (type inconnu ou abstrait). Les références conditionnelles (`Patient?identifier=...`) SHALL être acceptées si le type est valide. Les références absolues et `urn:` SHALL être ignorées par cette règle (non résolues) ; les références absolues ne sont vérifiées que par `REFERENCE_003`.

#### Scenario: Type de ressource inconnu

- **WHEN** `reference` vaut `Foo/123` ou `Foo/a:b`
- **THEN** un Diagnostic `REFERENCE_001` SHALL être émis avec `path` se terminant par `.reference`

#### Scenario: Type abstrait

- **WHEN** `reference` vaut `DomainResource/1`
- **THEN** un Diagnostic `REFERENCE_001` SHALL être émis

#### Scenario: Référence conditionnelle, absolue ou urn

- **WHEN** `reference` vaut `Patient?identifier=x`, `http://example.org/Patient/1` ou `urn:uuid:...`
- **THEN** aucun Diagnostic `REFERENCE_001` SHALL être émis

### Requirement: Validation des références — référence relative mal formée (REFERENCE_004)

Le système SHALL émettre un Diagnostic de sévérité warning `REFERENCE_004` lorsqu'une référence relative (sans schéma URI (RFC 3986), sans `?`, ne commençant pas par `#`) dont le premier segment est un type de ressource concret connu ne correspond pas à `Type/id` ou `Type/id/_history/version`, avec id et version de la forme `[A-Za-z0-9\-\.]{1,64}`. L'expression régulière n'étant pas énoncée comme un SHALL par la spécification FHIR, la sévérité est warning. Une référence relative à un type inconnu ou abstrait SHALL relever uniquement de `REFERENCE_001`.

#### Scenario: Référence relative mal formée

- **WHEN** `reference` vaut `Organization`, `Practitioner/a b`, `Practitioner/a:b` ou `Practitioner/1/_history`
- **THEN** un Diagnostic `REFERENCE_004` de sévérité warning SHALL être émis avec `path` se terminant par `.reference`

#### Scenario: Référence relative bien formée

- **WHEN** `reference` vaut `Practitioner/1` ou `Practitioner/1/_history/2`
- **THEN** aucun Diagnostic `REFERENCE_004` SHALL être émis

#### Scenario: Type inconnu

- **WHEN** `reference` vaut `Foo`
- **THEN** seul `REFERENCE_001` SHALL être émis, sans `REFERENCE_004`
