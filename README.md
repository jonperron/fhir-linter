# fhir-linter

A linter and validator for FHIR R5 resources.

## What it checks

- **Validation:** structure, cardinality, types, terminology, invariants (via FHIRPath).
- **Linting:** style rules you can turn on, off, or re-grade.

Built-in lint rules:

| Rule | Flags |
|------|-------|
| `narrative-absent` | Resource has no narrative text: `text` absent, `text.status` is `empty`, or the div holds only whitespace once tags are stripped and no `<img>` |
| `missing-identifier` | Resource has no identifier (empty arrays count as none); a Bundle only when its type is `document`, `message` or `collection` |
| `missing-meta-profile` | Resource has no `meta.profile` (empty arrays count as none) |

`narrative-absent` is a deliberate superset of the validator's `dom-6`
invariant: a resource without `text` gets both `INVARIANT_dom-6` from
`validate` and `LINT_001` from `lint`, and `LINT_001` also fires on empty
narrative that `dom-6` accepts.

## Install

Each GitHub release carries the Python wheels and sdist, and an npm
tarball (`npm install ./fhir-linter-<version>.tgz`), for Linux x86_64,
macOS arm64 and Windows x86_64. The `fhir-lint` CLI is not released yet.
Nothing is published to PyPI, npm or crates.io yet. On Linux, the npm
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
