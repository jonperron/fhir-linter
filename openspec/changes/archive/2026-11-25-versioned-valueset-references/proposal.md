## Why

ValueSet references like `url|version` had their version stripped, so a binding pinned to another ValueSet version was checked against the loaded one and could raise false TERMINOLOGY_001.

## What Changes

- Loader keeps `|version` on binding, additional binding and `compose.include.valueSet` references; stores `ValueSet.version`.
- `Registry::value_set` matches a versioned reference only on exact version; mismatch is undecided (`None`) for `value_set_contains`, `expand`, memberOf.
- Validator reports nothing for a mismatched pinned binding; messages print the canonical as written.

## Capabilities

### Modified Capabilities

- `fhir-definitions-registry`
- `fhir-validator`

## Impact

`fhir-definitions` model (`ValueSet.version`), loader, registry; validator terminology checks.
