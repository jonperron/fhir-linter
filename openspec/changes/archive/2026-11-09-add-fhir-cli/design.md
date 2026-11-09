## Context

`fhir-lint` assemble `fhir-parser`, `fhir-validator`, `fhir-linter` et `fhir-definitions` derrière une interface clap.

## Goals / Non-Goals

**Goals:** rapport déterministe (ordre de fichiers trié), erreurs d'outil distinctes des diagnostics, sortie SARIF minimale valide.

**Non-Goals:** option `--profile` (aucune API du validateur pour appliquer un profil arbitraire), versions FHIR autres que R5.

## Decisions

- **`--fhir-version` au lieu de `--version`** : clap réserve `--version` à la version de l'outil. Seule la valeur `r5` existe ; un match exhaustif force le traitement de toute nouvelle version.
- **Échec de parsing = diagnostic** : une ressource illisible (JSON/XML invalide, non-UTF-8) produit un diagnostic `PARSE_001` d'erreur, pas une erreur d'outil, pour qu'un fichier cassé n'arrête pas le lot.
- **Configuration validée avant les entrées** : `check_config` est appelé avant le chargement des définitions et la lecture des ressources, pour qu'une faute de frappe soit signalée même si aucune ressource ne parse.
- **Liens symboliques vers des répertoires ignorés** : évite les boucles infinies lors du parcours récursif.
- **Sortie texte sûre pour le terminal** : messages repliés sur une ligne et caractères de contrôle échappés, car le contenu provient des ressources.
- **URI SARIF** : chemins relatifs conservés relatifs, absolus préfixés `file://`, octets hors `[A-Za-z0-9-._~/]` encodés en pourcentage (`:` inclus, pour qu'un chemin relatif ne se lise pas comme un schéma).

## Risks / Trade-offs

- `ParseError` ne fournit pas d'offset : la position de `PARSE_001` n'a que ligne et colonne.
- Le chemin par défaut `definitions/r5` est relatif au répertoire courant ; hors du dépôt, il faut `--definitions` ou `FHIR_DEFINITIONS_DIR`.
