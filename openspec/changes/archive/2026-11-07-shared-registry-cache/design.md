## Context

Les deux bindings maintenaient chacun un `Mutex<HashMap<PathBuf, Arc<Registry>>>` et un `match` `Severity` → chaîne identiques.

## Goals / Non-Goals

**Goals:** une seule implémentation du cache et de la conversion, dans les crates Rust.

**Non-Goals:** éviction du cache, modification de l'API des bindings.

## Decisions

- `Registry::shared` utilise un `OnceLock<Mutex<HashMap<PathBuf, Arc<Registry>>>>` statique, indexé par `fs::canonicalize(dir)`.
- Le verrou est tenu pendant le chargement pour qu'aucun répertoire ne soit construit deux fois en parallèle ; un verrou empoisonné est récupéré.
- Un chargement en échec n'est pas inséré, l'appel suivant réessaie.
- `Severity::as_str` retourne `&'static str`.

## Risks / Trade-offs

- Le verrou global sérialise les premiers chargements de répertoires différents (acceptable : chargement rare).
- Pas d'éviction : chaque répertoire distinct reste en mémoire ; `definitionsDir` doit venir d'une source de confiance.
