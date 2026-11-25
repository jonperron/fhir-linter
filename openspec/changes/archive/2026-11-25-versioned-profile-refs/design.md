## Context

Commit 7b5a642 : résolution des références versionnées de type et de slice.

## Goals / Non-Goals

- Goals : uniformiser le traitement de `url|version` sur les trois chemins de profil.
- Non-Goals : indexer les ressources contenues d'une StructureDefinition.

## Decisions

- Le chargeur ne retire plus la version ; le validateur la retire là où une URL nue est requise (targetProfile, `Extension.url`, baseDefinition).
- Un fragment `#id` rend la référence non résolue (PROFILE_001 pour meta.profile).
- PROFILE_003 reste un warning, la validation continue contre la version chargée (décision utilisateur V10).

## Risks

- Une référence versionnée à un profil sans snapshot donne PROFILE_010 (erreur) sur tous les chemins.
