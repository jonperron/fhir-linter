## Context

The registry loaded only `profiles-resources.json`, `profiles-types.json` and
`valuesets.json`.

## Goals / Non-Goals

- Goal: resolve core HL7 profiles (vitalsigns, etc.) by canonical URL.
- Non-goal: slicing or fixed-value checks for these profiles.

## Decisions

- Add the file to the existing profile loop; keep it required like the other
  bundles (the download script and `definitions.json.zip` always provide it).

## Risks / Trade-offs

- Custom definitions directories without `profiles-others.json` now fail to
  load with `Error::Io`.
