## Why

Les références canoniques versionnées (`url|version`) de `type.profile`, des profils de slice et de `meta.profile` n'étaient pas traitées de façon uniforme : la version était retirée au chargement et `url|version#id` était confondu avec `url`.

## What Changes

- Le chargeur conserve `type.profile` et `targetProfile` tels qu'écrits (version incluse).
- Le validateur sépare l'URL (clé du Registry) de la version ; il retire la version pour `targetProfile`, les discriminateurs `url` des slices d'extension et les recherches `baseDefinition`.
- Une référence `url|version#id` désigne une ressource contenue (R5 references.html, « Canonical URLs ») que le Registry n'indexe pas : elle n'est pas résolue.
- `PROFILE_003` s'applique à `meta.profile`, `type.profile` et aux profils de slice d'un discriminateur `profile`.
- Un profil fourni sans snapshot sous forme versionnée donne `PROFILE_010` partout.

## Capabilities

### Modified Capabilities
- `fhir-validator`: PROFILE_001, PROFILE_003, PROFILE_010, type.profile.
- `fhir-definitions-registry`: profils de types conservés tels qu'écrits.

## Impact

`fhir-definitions` (chargeur) et `fhir-validator` (profils). Aucun changement des planchers de suites.
