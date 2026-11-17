## Why
htmlChecks accepted block elements inside a paragraph, contrary to HTML 4.01 (`<!ELEMENT P - O (%inline;)*>`).

## What Changes
- htmlChecks returns false when a %block element sits anywhere inside a `p`; a `map` descendant resets the rule.

## Capabilities
### Modified Capabilities
- fhir-fhirpath

## Impact
crates/fhir-fhirpath/src/eval/html_checks.rs
