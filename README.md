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

## Install

Each GitHub release carries the `fhir-lint` CLI archives (Linux x86_64,
macOS arm64, Windows x86_64), the Python wheels and sdist, and an npm
tarball (`npm install ./fhir-linter-<version>.tgz`). Nothing is published
to PyPI, npm or crates.io yet.

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
