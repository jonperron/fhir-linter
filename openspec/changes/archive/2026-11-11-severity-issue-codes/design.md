## Context

FHIR R5 définit IssueSeverity (`fatal | error | warning | information | success`) et lie la severity des contraintes à constraint-severity (`error | warning`, binding required). Le type `Severity` utilisait un niveau `Hint` hors FHIR et rétrogradait `information` en Warning.

## Goals / Non-Goals

**Goals :** aligner `Severity` et sa forme textuelle sur IssueSeverity ; garder les configurations `hint` existantes fonctionnelles.

**Non-Goals :** exposer `success` (ce n'est pas un niveau de diagnostic) ; émettre des diagnostics `Fatal` depuis le validateur (le variant existe pour la configuration du linter).

## Decisions

- `Severity::ALL` sert de source unique : `FromStr` cherche dans `ALL` le variant dont `as_str` égale l'entrée, ce qui garantit l'aller-retour exact.
- `hint` est traité uniquement par `parse_severity` du linter (alias de config), pas par `FromStr`, pour que le type reste strictement IssueSeverity.
- Invariants : seule la chaîne exacte `warning` rétrograde ; une valeur hors binding est traitée comme `error` plutôt que de masquer une violation.
- `RuleRegistry::register` supprimé : aucun appelant, et les règles personnalisées n'étaient pas configurables par TOML.

## Risks / Trade-offs

- Changement cassant pour les consommateurs Rust qui utilisaient `Severity::Hint` ou `register`, et pour ceux des bindings qui comparaient à `"hint"`. Atténué par l'alias de configuration.
