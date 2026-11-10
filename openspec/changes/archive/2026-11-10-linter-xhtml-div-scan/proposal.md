## Why

Le repérage des balises par recherche du premier `>` laissait du texte derrière un `>` placé dans un commentaire ou une valeur d'attribut, ce qui faisait taire `narrative-absent`. En entrée XML, un primitif portant une extension est stocké comme objet avec `value`, si bien que `text.status` et `Bundle.type` étaient ignorés.

## What Changes

- Le `text.div` JSON SHALL être analysé comme XHTML (commentaires et attributs ignorés, CDATA compté, blancs XML seulement, entités nommées comptées comme texte, `<img>` à tout préfixe et profondeur, XHTML mal formé compté comme contenu).
- En entrée XML, seul le texte non blanc compte ; un div ne contenant qu'une image est signalé (limite connue).
- En entrée XML, `text.status` et `Bundle.type` SHALL être lus aussi depuis un primitif portant une extension.

## Capabilities

### New Capabilities

Aucune.

### Modified Capabilities

- `fhir-linter` : exigence « Règles intégrées ».

## Impact

`crates/fhir-linter/src/rules.rs`, nouvelle dépendance `quick-xml` dans `crates/fhir-linter/Cargo.toml`, tests dans `crates/fhir-linter/tests/rules_tests.rs`.
