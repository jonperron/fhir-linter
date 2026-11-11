## RENAMED Requirements

- FROM: `### Requirement: Grammaire PEG FHIRPath 3.0`
- TO: `### Requirement: Grammaire PEG FHIRPath release 2`

## MODIFIED Requirements

### Requirement: Grammaire PEG FHIRPath release 2

Le système SHALL implémenter une grammaire PEG via `pest` couvrant la syntaxe de FHIRPath release 2 (2.0.0), version utilisée par FHIR R5, plus quelques ajouts ultérieurs (par ex. le littéral Long).

#### Scenario: Littéraux pris en charge

- **WHEN** l'expression contient un littéral (booléen, entier, Long `45L`, décimal, chaîne entre guillemets simples, date `@YYYY-MM-DD`, dateTime complet `@YYYY-MM-DDThh:mm:ss`, dateTime partiel `@YYYY-MM-DDT` / `@YYYY-MMT` / `@YYYYT`, heure `@Thh:mm:ss`, quantité `1.5 'mg'`)
- **THEN** le parser SHALL produire le nœud AST correspondant sans erreur

#### Scenario: Opérateurs binaires

- **WHEN** l'expression contient un opérateur (`+`, `-`, `*`, `/`, `div`, `mod`, `&`, `=`, `!=`, `~`, `!~`, `<`, `<=`, `>`, `>=`, `and`, `or`, `xor`, `implies`, `in`, `contains`, `is`, `as`, `|`)
- **THEN** le parser SHALL produire le nœud AST avec la bonne associativité et priorité

#### Scenario: Priorité de is/as et de l'union

- **WHEN** l'expression combine `|` avec `is` ou `as` (ex. `a | b is X`)
- **THEN** `is`/`as` SHALL lier plus fort que `|` (`a | (b is X)`)

#### Scenario: Limites de taille et d'imbrication

- **WHEN** l'expression dépasse 1 KiB ou plus de 16 niveaux d'imbrication de `(` / `[` (hors chaînes, backticks et commentaires)
- **THEN** `parse` SHALL retourner `Err(ParseError::Syntax(...))` sans dépasser la pile ; toute expression acceptée au maximum de ces limites (chaînes `a.a.a…`, `1+1+…`, parenthèses imbriquées) SHALL pouvoir être parsée, évaluée et libérée dans un thread de 2 Mo en debug

#### Scenario: Identifiants délimités par backticks

- **WHEN** l'expression contient un DELIMITEDIDENTIFIER (ex. ``text.`div` ``) dans un chemin, un nom de fonction ou un spécificateur de type
- **THEN** il SHALL être évalué exactement comme l'identifiant simple correspondant

#### Scenario: Séquences d'échappement

- **WHEN** une chaîne littérale ou un identifiant délimité contient une séquence d'échappement
- **THEN** le parser SHALL accepter l'ensemble ESC de la spécification : `` \` ``, `\'`, `\"`, `\\`, `\/`, `\f`, `\n`, `\r`, `\t`, `\uXXXX`

#### Scenario: Identifiants débutant par true/false

- **WHEN** l'expression contient un identifiant tel que `trueValue`
- **THEN** il SHALL être analysé comme identifiant et non comme littéral booléen

#### Scenario: Appels de fonctions

- **WHEN** l'expression contient un appel de fonction ou méthode (ex. `name.exists()`, `where(active = true)`)
- **THEN** le parser SHALL produire `Expr::Method` ou `Expr::FuncCall` avec les arguments corrects

### Requirement: AST Expr

Le système SHALL exposer un enum `Expr` couvrant toutes les constructions de la grammaire prise en charge.

#### Scenario: Variantes de l'AST

- **WHEN** un AST est construit
- **THEN** `Expr` SHALL couvrir : littéraux (`Null`, `Bool`, `Integer`, `Long`, `Decimal`, `String`, `Temporal`, `Quantity`), navigation (`DollarThis`, `DollarIndex`, `Ident`, `Dot`, `Index`), appels (`FuncCall`, `Method`), arithmétique (`Add`, `Sub`, `Mul`, `Div`, `DivInt`, `Mod`, `Concat`, `Neg`), comparaisons (`Eq`, `Neq`, `Lt`, `Lte`, `Gt`, `Gte`, `Equiv`, `NotEquiv`), booléens (`And`, `Or`, `Xor`, `Implies`), types (`Is`, `As`), ensembles (`Union`, `In`, `Contains`)

### Requirement: Fonctions diverses

Le système SHALL implémenter `not()`, `iif(condition, trueResult, otherwiseResult)`, `trace(name)`, `hasValue()`, `getValue()`, `type()`. Les autres fonctions diverses de FHIRPath ne sont pas implémentées (voir « Limitations connues »).

#### Scenario: not inverse un booléen

- **WHEN** `true.not()` est évalué
- **THEN** le résultat SHALL être `[Bool(false)]`

#### Scenario: iif retourne la branche correcte

- **WHEN** `iif(true, 'yes', 'no')` est évalué
- **THEN** le résultat SHALL être `[String("yes")]`

### Requirement: Limitations connues

Le système SHALL documenter les limitations suivantes de l'implémentation actuelle.

#### Scenario: $index non supporté

- **WHEN** une expression utilise `$index` dans un contexte d'itération
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::UndefinedFunction("$index"))`

#### Scenario: aggregate sans $total

- **WHEN** `aggregate()` est utilisé avec une référence à `$total`
- **THEN** l'accumulateur `$total` ne sera pas accessible (comportement non défini dans l'implémentation actuelle)

#### Scenario: Fonctions non implémentées retournent UndefinedFunction

- **WHEN** `resolve()`, `memberOf()`, `conformsTo()`, `today()`, `now()`, `timeOfDay()`, `lowBoundary()`, `highBoundary()`, `precision()`, `htmlChecks()` ou `comparable()` est évalué
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::UndefinedFunction(name))` où `name` est le nom de la fonction
- **AND** il SHALL NOT retourner de résultat deviné (collection vide, booléen), afin que le validateur signale la contrainte comme non vérifiée (`INVARIANT_SKIPPED`)
