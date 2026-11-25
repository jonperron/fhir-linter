## ADDED Requirements

### Requirement: Registry partagé avec surcouche

Le système SHALL fournir `Registry::shared_or_env_with_extra(dir: Option<String>, extra: &[impl AsRef<Path>]) -> Option<Result<Arc<Registry>, SharedRegistryError>>`. La base SHALL être résolue comme `Registry::shared_or_env` (`None` lorsqu'aucun répertoire n'est désigné). Si `extra` est vide, la fonction SHALL retourner l'`Arc` de base partagé lui-même ; sinon elle SHALL retourner `Registry::with_extra(base, extra)` dans un nouvel `Arc`, construit à chaque appel et jamais mis en cache. `SharedRegistryError` (réexportée à la racine de la crate) SHALL valoir `Base(Error)` (affichée « cannot load definitions: {e} ») ou `Extra(Error)` (affichée « cannot load extra definitions: {e} »).

#### Scenario: Sans définitions supplémentaires

- **WHEN** `Registry::shared_or_env_with_extra` est appelé avec un `extra` vide et un répertoire valide
- **THEN** le résultat SHALL être l'`Arc` retourné par `Registry::shared` (`Arc::ptr_eq` vrai)

#### Scenario: Échec de la surcouche

- **WHEN** `extra` désigne un chemin non chargeable
- **THEN** le résultat SHALL être `Some(Err(SharedRegistryError::Extra(_)))`
