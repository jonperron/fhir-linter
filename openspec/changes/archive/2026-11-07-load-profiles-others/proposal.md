## Why

`profiles-others.json` (core profiles such as `vitalsigns`) is downloaded but
never loaded, so resources declaring a core profile in `meta.profile` get
`PROFILE_001` and the profile's rules never run.

## What Changes

- `Registry::from_definitions_dir` also loads `profiles-others.json`.
- The file is required like its siblings: a missing file returns `Error::Io`.

## Capabilities

### Modified Capabilities

- `fhir-definitions-registry`: loads core profiles from `profiles-others.json`.
- `fhir-bundle-loader`: `profiles-others.json` is a supported official bundle.

## Impact

- `crates/fhir-definitions/src/registry.rs`; load time increases by about
  100-200 ms, peak memory unchanged.
