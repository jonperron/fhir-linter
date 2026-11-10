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

Each release is published to PyPI (`pip install fhir-linter`) and to
GitHub Packages (`@jonperron/fhir-linter`), and its wheels, sdist and
npm tarball are attached to the GitHub release, for Linux x86_64, macOS
arm64 and Windows x86_64. Installing from GitHub Packages needs
`@jonperron:registry=https://npm.pkg.github.com` in `.npmrc` and a token
with `read:packages`. The `fhir-lint` CLI is not released yet, and
nothing is published to crates.io or npmjs.org. On Linux, the npm
addon needs glibc 2.35 or newer (Ubuntu 22.04, Debian 12, RHEL 10); the
wheels are manylinux2014 (glibc 2.17).

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
