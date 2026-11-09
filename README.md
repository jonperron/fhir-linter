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

Each GitHub release carries the Python wheels and sdist, an npm tarball
(`npm install ./fhir-linter-<version>.tgz`), and `fhir-lint` CLI archives
(Linux x86_64, macOS arm64, Windows x86_64). The CLI is a preview stub: it
only prints its version until it is implemented. Nothing is published
to PyPI, npm or crates.io yet. On Linux, the CLI and the npm addon need
glibc 2.35 or newer (Ubuntu 22.04, Debian 12, RHEL 10); the wheels are
manylinux2014 (glibc 2.17).

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
| `fhir-cli` | The `fhir-lint` binary (not yet implemented: it only prints its version) |

Specs live in `openspec/specs/`.

## License

See [LICENSE](LICENSE).
