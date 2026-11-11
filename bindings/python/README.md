# fhir-linter

Python bindings for the FHIR R5 linter and validator.

```python
import fhir_linter

for d in fhir_linter.validate(resource_json) + fhir_linter.lint(resource_json):
    print(d.severity, d.code, d.path, d.message)
```

## Locations

`line` is 1-indexed. `col` (1-indexed) and `offset` (0-indexed) count UTF-8
bytes, not characters: after non-ASCII text such as `"ééé"`,
`resource_json[d.offset:]` lands too far. Index the encoded bytes instead:
`resource_json.encode()[d.offset:]`. All three are `None` without a location.

## FHIR definitions

The wheel does not bundle the FHIR R5 definitions (about 60 MB). Download them
once with `scripts/download-definitions.sh` from the
[repository](https://github.com/jonperron/fihr-linter), then either pass
`definitions_dir=` to each call or set `FHIR_DEFINITIONS_DIR`. Without them,
calls raise `ValueError`.

`definitions_dir` is read from disk: only pass paths you trust.

## Errors

- `ValueError`: unparsable JSON, invalid lint config, unknown rule, no definitions.
- `RuntimeError`: the definitions directory cannot be loaded.
