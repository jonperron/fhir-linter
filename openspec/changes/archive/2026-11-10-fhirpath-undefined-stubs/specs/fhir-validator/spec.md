## MODIFIED Requirements

### Requirement: Validation des invariants FHIRPath (INVARIANT_<key>)

Le système SHALL évaluer les expressions FHIRPath de contrainte définies sur l'élément racine du snapshot. Pour chaque contrainte dont l'expression ne retourne pas `[true]`, il SHALL émettre un `Diagnostic` avec code `INVARIANT_<key>` où `<key>` est la valeur du champ `key` de la contrainte. Le code `INVARIANT_SKIPPED` est réservé : il signale une contrainte non vérifiée et n'est jamais une violation.

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


#### Scenario: Severity `information` mappée en Warning

- **WHEN** une contrainte violée a `severity="information"`
- **THEN** le Diagnostic `INVARIANT_<key>` SHALL avoir la severity `Warning`

#### Scenario: Contrainte sans expression ignorée

- **WHEN** une contrainte n'a pas de champ `expression`
- **THEN** aucun Diagnostic ne SHALL être émis pour cette contrainte
