## ADDED Requirements

### Requirement: Registry partagé par répertoire

Le système SHALL fournir `Registry::shared(dir) -> Result<Arc<Registry>>` qui charge le registry d'un répertoire une seule fois par processus et le partage. Le cache SHALL être indexé par le chemin canonicalisé (`defs`, `./defs` ou un lien symbolique désignent le même répertoire), SHALL être thread-safe, SHALL ne pas conserver les chargements en échec et SHALL ne jamais évincer d'entrée.

#### Scenario: Chemins équivalents partagent le même registry

- **WHEN** `Registry::shared` est appelé plusieurs fois avec des chemins désignant le même répertoire de définitions
- **THEN** chaque appel SHALL retourner le même `Arc` (`Arc::ptr_eq` vrai)

#### Scenario: Chemin non canonicalisable retourne une erreur

- **WHEN** `Registry::shared` est appelé avec un chemin inexistant
- **THEN** il SHALL retourner une erreur `Error::Io` et rien ne SHALL être mis en cache
