# fhir-linter

Python bindings for the FHIR R5 linter and validator. Requires Python 3.10+.

```bash
pip install fhir-linter
```

Wheels are built for Linux x86_64 (manylinux2014, glibc 2.17), macOS arm64
and Windows x86_64.

## Quick start

```python
import json
import fhir_linter

resource_json = json.dumps({"resourceType": "Patient", "id": "p1"})

for d in fhir_linter.validate(resource_json) + fhir_linter.lint(resource_json):
    print(d.severity, d.code, d.path, d.message)
```

Both functions take the resource as a JSON **string**, not a `dict`. XML input
is not supported by the bindings.

## FHIR definitions

The wheel does not bundle the FHIR R5 definitions (about 60 MB). Download them
once with `scripts/download-definitions.sh` from the
[repository](https://github.com/jonperron/fihr-linter), then either pass
`definitions_dir=` to each call or set `FHIR_DEFINITIONS_DIR`. Without them,
calls raise `ValueError`.

The first call for a directory loads the definitions, which takes a few
seconds; later calls in the same process reuse them. Both functions release
the GIL, so they can run from several threads.

`definitions_dir` is read from disk: only pass paths you trust.

## API

### `validate(json, definitions_dir=None) -> list[Diagnostic]`

Checks structure, cardinality, types, terminology and invariants (FHIRPath)
against the R5 definitions.

### `lint(json, config_toml=None, definitions_dir=None) -> list[Diagnostic]`

Runs the lint rules. `config_toml` is the **content** of a `fhir-linter.toml`
file, not a path:

```python
from pathlib import Path

config = Path("fhir-linter.toml").read_text()
diagnostics = fhir_linter.lint(resource_json, config_toml=config)
```

```toml
[rules.narrative-absent]
severity = "error"      # error | warning | information | hint

[rules.missing-identifier]
enabled = false
```

Built-in rules, all `warning` by default:

| Rule | Code | Flags |
|------|------|-------|
| `narrative-absent` | `LINT_001` | No narrative text |
| `missing-identifier` | `LINT_002` | No identifier |
| `missing-meta-profile` | `LINT_003` | No `meta.profile` |

### `Diagnostic`

Read-only object with these attributes:

| Attribute | Type | Meaning |
|-----------|------|---------|
| `severity` | `str` | `error`, `warning`, `information` or `hint` |
| `code` | `str` | Stable code, e.g. `STRUCTURE_000`, `INVARIANT_dom-6`, `LINT_001` |
| `message` | `str` | Human-readable description |
| `path` | `str` | Element path in the resource |
| `line` | `int \| None` | 1-indexed line |
| `col` | `int \| None` | 1-indexed column, in UTF-8 bytes |
| `offset` | `int \| None` | 0-indexed offset, in UTF-8 bytes |
| `rule` | `str \| None` | Lint rule id; `None` for validator findings |

## Locations

`col` and `offset` count UTF-8 bytes, not characters: after non-ASCII text
such as `"ééé"`, `resource_json[d.offset:]` lands too far. Index the encoded
bytes instead: `resource_json.encode()[d.offset:]`. `line`, `col` and
`offset` are all `None` without a location.

## Errors

- `ValueError`: unparsable JSON, invalid lint config, unknown rule, no definitions.
- `RuntimeError`: the definitions directory cannot be loaded.
