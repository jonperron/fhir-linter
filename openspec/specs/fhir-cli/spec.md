# Spec: fhir-cli

## Purpose
Binaire `fhir-lint` : valide et lint des ressources FHIR R5 en ligne de commande, avec rapport texte, JSON ou SARIF.
## Requirements
### Requirement: Arguments du binaire fhir-lint

Le crate `fhir-cli` SHALL fournir le binaire `fhir-lint` acceptant :

| Argument | Rôle |
|---|---|
| `<PATHS>...` | Obligatoire. Fichiers ou répertoires à analyser |
| `--format text\|json\|sarif` | Format du rapport, `text` par défaut |
| `--config <fichier>` | Configuration `fhir-linter.toml` |
| `--fhir-version r5` | Version FHIR des entrées ; `r5` est la seule valeur acceptée |
| `--definitions <dir>` | Répertoire des définitions ; variable `FHIR_DEFINITIONS_DIR`, défaut `definitions/r5` relatif au répertoire courant |
| `--strict` | Les avertissements font aussi échouer l'exécution |

`--version` SHALL afficher la version de l'outil (écart avec PLAN.md, où `--version` désignait la version FHIR). Le binaire SHALL NOT proposer d'option `--profile` : seuls les profils déclarés dans `meta.profile` sont vérifiés.

#### Scenario: Version FHIR inconnue

- **WHEN** `fhir-lint --fhir-version r4 <dir>` est exécuté
- **THEN** le code de sortie SHALL être 2

#### Scenario: Version r5

- **WHEN** `fhir-lint --fhir-version r5` est exécuté sur un répertoire de ressources valides
- **THEN** le code de sortie SHALL être 0

#### Scenario: Aucun chemin

- **WHEN** `fhir-lint` est exécuté sans `<PATHS>`
- **THEN** le code de sortie SHALL être 2

### Requirement: Collecte des entrées

Un fichier passé explicitement SHALL être analysé quelle que soit son extension. Un répertoire SHALL être parcouru récursivement, en ordre trié, pour les fichiers `*.json` et `*.xml` (extension insensible à la casse) ; les liens symboliques vers des répertoires SHALL être ignorés. Un fichier d'extension `.xml` SHALL être parsé en XML, tout autre fichier en JSON. Un chemin inexistant ou illisible SHALL être une erreur d'outil.

#### Scenario: Parcours récursif

- **WHEN** un répertoire contient `bare.json` et un sous-répertoire contenant `bare.xml`
- **THEN** les diagnostics SHALL porter sur les deux fichiers, `bare.json` avant `bare.xml`

#### Scenario: Fichier XML

- **WHEN** un fichier `patient.xml` valide est passé
- **THEN** il SHALL être parsé en XML et le code de sortie SHALL être 0

#### Scenario: Entrées répétées ou spéciales

- **WHEN** un même fichier est passé plusieurs fois (ou via des chemins qui se recouvrent)
- **THEN** il SHALL être analysé une seule fois
- **WHEN** un chemin explicite n'est ni un fichier régulier ni un répertoire (FIFO, périphérique)
- **THEN** le code de sortie SHALL être 2 (erreur d'outil), sans lecture bloquante

#### Scenario: Chemin manquant

- **WHEN** `does-not-exist.json` est passé
- **THEN** le code de sortie SHALL être 2 et stderr SHALL nommer `does-not-exist.json`

### Requirement: Analyse par ressource

Pour chaque fichier, `fhir-lint` SHALL exécuter `fhir_validator::validate` puis `RuleRegistry::default().run` avec la configuration chargée. Un échec de parsing de la ressource ou un contenu non UTF-8 SHALL produire un unique Diagnostic d'erreur `PARSE_001` (avec ligne et colonne lorsqu'elles sont connues), et non une erreur d'outil.

#### Scenario: Ressource invalide

- **WHEN** une ressource contient l'élément inconnu `notAFhirElement`
- **THEN** la sortie SHALL contenir une erreur mentionnant `notAFhirElement` et le code de sortie SHALL être 1

#### Scenario: Ressource mal formée

- **WHEN** un fichier JSON n'est pas syntaxiquement valide
- **THEN** la sortie SHALL contenir `PARSE_001` pour ce fichier et le code de sortie SHALL être 1

#### Scenario: Règle désactivée par configuration

- **WHEN** `--config` désactive `missing-meta-profile`
- **THEN** la sortie SHALL ne pas contenir `LINT_003` mais SHALL contenir `LINT_002`

### Requirement: Codes de sortie

`fhir-lint` SHALL sortir avec :

- 0 si aucun diagnostic ne fait échouer l'exécution ;
- 1 si au moins un diagnostic `error` est produit, ou un `warning` avec `--strict` ; `information` et `hint` SHALL NOT faire échouer l'exécution ;
- 2 en cas d'erreur d'outil : erreur d'usage clap, chemin manquant ou illisible, configuration illisible ou invalide (règle ou sévérité inconnue, vérifiée par `check_config` avant la lecture des entrées), échec du chargement des définitions.

La configuration SHALL être chargée et vérifiée avant la collecte des chemins : une configuration invalide SHALL être signalée même si un chemin est manquant. Si stdout est fermé par le lecteur (`BrokenPipe`), `fhir-lint` SHALL NOT paniquer et SHALL sortir avec le code du verdict.

En cas d'erreur d'outil, `fhir-lint` SHALL écrire `fhir-lint: <message>` sur stderr et SHALL NOT produire de rapport.

#### Scenario: Avertissements seuls

- **WHEN** une ressource ne produit que des avertissements (dont `LINT_003`)
- **THEN** le code de sortie SHALL être 0

#### Scenario: Mode strict

- **WHEN** la même ressource est analysée avec `--strict`
- **THEN** le code de sortie SHALL être 1

#### Scenario: Définitions introuvables

- **WHEN** `--definitions /nonexistent/definitions` est passé
- **THEN** le code de sortie SHALL être 2

#### Scenario: Règle inconnue dans la configuration

- **WHEN** `--config` référence `no-such-rule`
- **THEN** le code de sortie SHALL être 2 et stderr SHALL contenir `no-such-rule`

### Requirement: Format texte

Le format `text` SHALL écrire une ligne par diagnostic : `fichier[:ligne:col]: sévérité CODE [chemin]: message`. Le fichier, le chemin et le message SHALL être repliés sur une seule ligne et leurs caractères de contrôle échappés. Sans diagnostic, rien ne SHALL être écrit.

#### Scenario: Une ligne par diagnostic

- **WHEN** une ressource est analysée en `text` et en `json`
- **THEN** le nombre de lignes de la sortie texte SHALL égaler le nombre d'éléments du tableau JSON

#### Scenario: Caractères de contrôle

- **WHEN** un message contient `\u{1b}` ou `\r`
- **THEN** la sortie texte SHALL les écrire échappés

### Requirement: Format JSON

Le format `json` SHALL écrire un tableau d'objets `{file, severity, code, message, path, line, col, rule}` ; `line`, `col` et `rule` SHALL valoir `null` lorsqu'ils sont inconnus.

#### Scenario: Diagnostic de lint

- **WHEN** une ressource sans `meta.profile` est analysée en `json`
- **THEN** le tableau SHALL contenir un objet `code = "LINT_003"`, `severity = "warning"`, `rule = "missing-meta-profile"` et un `file` désignant le fichier

### Requirement: Format SARIF

Le format `sarif` SHALL écrire un journal SARIF 2.1.0 : `$schema`, `version = "2.1.0"`, `runs[0].tool.driver` avec `name = "fhir-lint"`, `version` de l'outil et `rules` (un `{id}` par code distinct, triés). Chaque résultat SHALL avoir `ruleId` (le code), `level` (`error`, `warning`, ou `note` pour `information` et `hint`), `message.text` et `locations[0].physicalLocation` avec `artifactLocation.uri` et, si la position est connue, `region.startLine` / `startColumn` au moins égaux à 1. L'URI SHALL rester relative pour un chemin relatif, être préfixée `file://` pour un chemin absolu, et encoder en pourcentage tout octet hors `[A-Za-z0-9-._~/]`.

#### Scenario: Journal minimal

- **WHEN** une ressource invalide est analysée en `sarif`
- **THEN** un résultat `level = "error"` SHALL exister, son `ruleId` SHALL figurer dans `driver.rules` et son URI SHALL désigner le fichier

#### Scenario: Ressources valides

- **WHEN** un répertoire de ressources valides est analysé en `sarif`
- **THEN** tous les résultats SHALL être de niveau `note`

#### Scenario: Encodage d'URI

- **WHEN** le fichier est `dir/a b.json`, `javascript:x.json` ou `/tmp/x.json`
- **THEN** l'URI SHALL être respectivement `dir/a%20b.json`, `javascript%3Ax.json` et `file:///tmp/x.json`

#### Scenario: Région jamais nulle

- **WHEN** un diagnostic a pour position ligne 0, colonne 0
- **THEN** la région SHALL être `startLine = 1`, `startColumn = 1`

