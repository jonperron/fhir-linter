# fhir-linter

A linter and validator for FHIR R5 resources.

## What it checks

- **Validation:** structure, cardinality, types, terminology, invariants (via FHIRPath).
- **Linting:** style rules you can turn on, off, or re-grade.

Built-in lint rules:

| Rule | Flags |
|------|-------|
| `narrative-absent` | Resource has no narrative text |
| `missing-identifier` | Resource has no identifier |
| `missing-meta-profile` | Resource has no `meta.profile` |

## Setup

```bash
git clone --recurse-submodules <repo-url>
cd fihr-linter
./scripts/download-definitions.sh   # fetches FHIR R5 definitions, SHA-256 verified
cargo build --workspace
cargo test --workspace
```

## Configure

Create a `fhir-linter.toml`:

```toml
[rules.narrative-absent]
severity = "error"      # error | warning | information | hint

[rules.missing-identifier]
enabled = false
```

## Run

```bash
cargo run -p fhir-cli -- [--format text|json|sarif] [--config fhir-linter.toml] \
    [--strict] [--definitions definitions/r5] <FILES|DIRS>...
```

Directories are scanned recursively for `*.json` and `*.xml`. The definitions
directory defaults to `definitions/r5` (or `$FHIR_DEFINITIONS_DIR`).
Exit codes: `0` clean, `1` errors found (warnings too with `--strict`),
`2` tool error (bad arguments, unreadable file, bad config or definitions).

## Layout

| Crate | Job |
|-------|-----|
| `fhir-definitions` | Loads R5 StructureDefinitions into a registry |
| `fhir-parser` | JSON and XML to an AST with source spans |
| `fhir-fhirpath` | FHIRPath evaluator |
| `fhir-validator` | Validation checks |
| `fhir-linter` | Configurable lint rules |
| `fhir-cli` | The `fhir-lint` binary |

Specs live in `openspec/specs/`.

## License

See [LICENSE](LICENSE).
