## MODIFIED Requirements

### Requirement: Validation des invariants FHIRPath (INVARIANT_<key>)

Le système SHALL évaluer les expressions FHIRPath de contrainte définies sur l'élément racine du snapshot. La ressource SHALL être convertie par `fhir_fhirpath::resource_to_typed_value` avec `Registry::child_element`, de sorte que les primitives `date`, `dateTime`, `instant` et `time` soient comparées comme des valeurs temporelles. Pour chaque contrainte dont l'expression ne retourne pas `[true]` (une seule valeur vraie, y compris un booléen FHIR portant des extensions), il SHALL émettre un `Diagnostic` avec code `INVARIANT_<key>` où `<key>` est la valeur du champ `key` de la contrainte. Le code `INVARIANT_SKIPPED` est réservé : il signale une contrainte non vérifiée et n'est jamais une violation. La severity R5 d'une contrainte est liée (binding required) à constraint-severity (`error | warning`) : seule la valeur exacte `warning` SHALL donner `Severity::Warning` ; toute autre valeur (`error`, mais aussi `information`, `Warning` ou un code inconnu) SHALL donner `Severity::Error`.

#### Scenario: Contrainte violée émet INVARIANT_<key>

- **WHEN** une contrainte avec `key="dom-2"` et `expression="..."` évalue à `[false]`
- **THEN** un Diagnostic avec code `INVARIANT_dom-2` SHALL être émis avec la severity correspondant à la severity de la contrainte

#### Scenario: Contrainte satisfaite n'émet pas de diagnostic

- **WHEN** une contrainte FHIRPath évalue à `[true]`
- **THEN** aucun Diagnostic `INVARIANT_` ne SHALL être émis pour cette contrainte

#### Scenario: Erreur d'évaluation FHIRPath signalée INVARIANT_SKIPPED

- **WHEN** l'évaluation FHIRPath d'une contrainte retourne une erreur
- **THEN** aucun Diagnostic `INVARIANT_<key>` ne SHALL être émis pour cette contrainte
- **AND** exactement un Diagnostic de severity `Information` avec code `INVARIANT_SKIPPED` SHALL être émis, au `path` du type de ressource, avec un message nommant la `key` de la contrainte et la raison de l'erreur

#### Scenario: Expression non analysable signalée INVARIANT_SKIPPED

- **WHEN** l'expression FHIRPath d'une contrainte ne peut pas être analysée (par ex. les variables d'environnement `%` telles que `%resource`, non encore supportées par le parseur, si bien que `dom-3` est toujours ignorée)
- **THEN** la contrainte SHALL être ignorée tant que cette lacune du parseur n'est pas comblée
- **AND** exactement un Diagnostic `Information` `INVARIANT_SKIPPED` nommant la `key` et la raison SHALL être émis, afin qu'un résultat sans erreur ne masque jamais une contrainte non vérifiée

#### Scenario: Invariant dom-6 évalué (identifiant entre backticks)

- **WHEN** une DomainResource R5 n'a pas de `text.div` (contrainte `dom-6`, ``text.`div`.exists()``, severity `warning`)
- **THEN** un Diagnostic `Warning` avec code `INVARIANT_dom-6` SHALL être émis
- **AND** aucun Diagnostic `INVARIANT_SKIPPED` ne SHALL être émis pour `dom-6`

#### Scenario: Invariant dgr-1 ignoré (resolve() non implémenté)

- **WHEN** une DiagnosticReport R5 avec `composition` est validée (contrainte `dgr-1`, dont l'expression appelle `resolve()`)
- **THEN** aucun Diagnostic `INVARIANT_dgr-1` ne SHALL être émis
- **AND** exactement un Diagnostic `Information` `INVARIANT_SKIPPED` nommant `dgr-1` SHALL être émis

#### Scenario: Severity hors de constraint-severity mappée en Error

- **WHEN** une contrainte violée a `severity="information"` ou `severity="Warning"` (codes sensibles à la casse)
- **THEN** le Diagnostic `INVARIANT_<key>` SHALL avoir la severity `Error`

#### Scenario: Contrainte sans expression ignorée

- **WHEN** une contrainte n'a pas de champ `expression`
- **THEN** aucun Diagnostic ne SHALL être émis pour cette contrainte

#### Scenario: Invariants évalués avec navigation des éléments choix

- **WHEN** une contrainte est évaluée
- **THEN** elle SHALL l'être via `evaluate_parsed_with_choices` avec `Registry::is_choice_variant`, de sorte qu'un nom de base (`value`) trouve sa variante (`valueQuantity`)

#### Scenario: fhs-2 et mrp-2 sans faux positif

- **WHEN** une ressource valide renseigne l'élément choix visé par `fhs-2` ou `mrp-2` via une variante typée
- **THEN** aucun Diagnostic `INVARIANT_fhs-2` ni `INVARIANT_mrp-2` ne SHALL être émis

#### Scenario: obs-6 détecte dataAbsentReason avec valueQuantity

- **WHEN** une Observation contient à la fois `dataAbsentReason` et `valueQuantity`
- **THEN** un Diagnostic `INVARIANT_obs-6` SHALL être émis

#### Scenario: app-5 compare des instants, pas du texte

- **WHEN** une Appointment a `start` = `2020-01-01T10:00:00+02:00` et `end` = `2020-01-01T09:30:00Z` (`app-5`, `start <= end`)
- **THEN** aucun Diagnostic `INVARIANT_app-5` ne SHALL être émis (08:00Z précède 09:30Z, alors que le texte « 10 » suit « 09 »)

#### Scenario: Primitive sans valeur dans un invariant

- **WHEN** une Bundle `document` n'a que `_timestamp` (extension data-absent-reason) (`bdl-10`)
- **THEN** un Diagnostic `INVARIANT_bdl-10` SHALL être émis
