# fhir-lint

Command-line validator and linter for FHIR R5 resources in JSON or XML.

## Install

The CLI is not released yet. Build it from the repository:

```bash
./scripts/download-definitions.sh      # FHIR R5 definitions, SHA-256 verified
cargo install --path crates/fhir-cli   # or: cargo run -p fhir-cli -- <args>
```

## Usage

```text
fhir-lint [OPTIONS] <PATHS>...
```

| Option | Default | Meaning |
|--------|---------|---------|
| `<PATHS>...` | required | Files, or directories scanned recursively for `*.json` and `*.xml` |
| `--format <text\|json\|sarif>` | `text` | Report format |
| `--config <FILE>` | none | `fhir-linter.toml` rule configuration |
| `--definitions <DIR>` | `$FHIR_DEFINITIONS_DIR`, else `definitions/r5` | FHIR definitions directory |
| `--fhir-version <r5>` | `r5` | FHIR version of the inputs; only `r5` exists |
| `--strict` | off | Warnings also fail the run |
| `-V`, `--version` | | Print the tool version |

The `definitions/r5` default is relative to the current directory: outside
the repository, pass `--definitions` or set `FHIR_DEFINITIONS_DIR`.

### Inputs

- A file named explicitly is always checked, whatever its extension.
- Directories are walked recursively in sorted order; only `*.json` and
  `*.xml` (any case) are picked up. Symlinked directories are skipped.
- `.xml` files are parsed as XML, everything else as JSON.
- A file reached twice (e.g. `dir` and `dir/a.json`) is checked once.

Each file is validated (structure, cardinality, types, terminology,
FHIRPath invariants) and then linted. A file that does not parse, or is not
UTF-8, yields one `PARSE_001` error instead of stopping the run.

## Examples

```console
$ fhir-lint patient.json
patient.json:5:22: error STRUCTURE_001 Patient.notAFhirElement: Unknown element 'Patient.notAFhirElement'
patient.json: warning INVARIANT_dom-6 Patient: A resource should have narrative for robust management
patient.json: warning LINT_001 Patient.text: Resource narrative (text.div) is absent or empty
patient.json: warning LINT_002 Patient.identifier: Resource has no identifier
patient.json: warning LINT_003 Patient.meta.profile: Resource does not declare meta.profile
$ echo $?
1
```

```bash
fhir-lint resources/                                   # whole tree
fhir-lint --strict --config fhir-linter.toml resources/
fhir-lint --format sarif resources/ > fhir-lint.sarif  # for code scanning
```

## Configuration

`--config` takes a `fhir-linter.toml`. Each rule can be disabled or
re-graded; unknown rules or keys are a tool error.

```toml
[rules.narrative-absent]
severity = "error"      # fatal | error | warning | information (hint = information)

[rules.missing-identifier]
enabled = false
```

| Rule | Code | Default | Flags |
|------|------|---------|-------|
| `narrative-absent` | `LINT_001` | warning | No narrative text |
| `missing-identifier` | `LINT_002` | warning | No identifier |
| `missing-meta-profile` | `LINT_003` | warning | No `meta.profile` |

Validator findings (`STRUCTURE_*`, `INVARIANT_*`, ...) are not configurable.

## Output formats

**text**: one line per finding,
`file[:line:col]: severity CODE [path]: message`. Line and column are
omitted when unknown. Control and bidi characters from the resource are
escaped and multi-line messages are folded onto one line.

**json**: one array of objects:

```json
[{"file":"patient.json","severity":"warning","code":"LINT_002",
  "message":"Resource has no identifier","path":"Patient.identifier",
  "line":null,"col":null,"rule":"missing-identifier"}]
```

`rule` is `null` for validator findings; `col` counts UTF-8 bytes.

**sarif**: SARIF 2.1.0, one run with tool `fhir-lint`. `ruleId` is the
diagnostic code; `fatal` and `error` map to SARIF level `error`, `warning`
to `warning` and `information` to `note`.
Absolute paths become `file://` URIs; relative paths stay relative.

Nothing is printed when there are no findings.

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | No errors (warnings allowed unless `--strict`) |
| `1` | At least one `fatal` or `error`, or a `warning` with `--strict` |
| `2` | Tool error: bad arguments, missing or unreadable path, invalid config, definitions not loadable |

`information` never fails the run. On a tool error, `fhir-lint`
prints `fhir-lint: <message>` to stderr and no report.
