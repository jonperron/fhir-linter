## ADDED Requirements

### Requirement: Validation des profils — contentReferenceProfile (DEFINITION_009)

Sur une StructureDefinition, pour chaque élément (différentiel et snapshot) portant l'extension `http://hl7.org/fhir/StructureDefinition/contentReferenceProfile` avec un `valueUri`, le système SHALL émettre une erreur `DEFINITION_009` à `StructureDefinition.<differential|snapshot>.element[i].extension[j]` lorsque la valeur n'est pas de la forme `<canonique>#<id d'élément>` (deux côtés non vides, « not a valid element reference »), ou lorsque l'élément visé est introuvable (« could not be found »). Base : « The value refers to an element definition in a profile, not to the profile itself, using the same format as the contentReference (a canonical URL for the profile, followed by '#' and the element id). » Le canonique (`|version` retiré) égal à l'`url` de la définition elle-même SHALL se résoudre contre elle (ids du snapshot et du différentiel, puis éléments de sa base par chemin) ; sinon contre une StructureDefinition du Registry ou de la surcouche, dont l'id SHALL être l'un des éléments. Une valeur du snapshot égale à celle de l'élément du différentiel de même id SHALL n'être signalée qu'une fois (sur le différentiel). Aucun Diagnostic SHALL être émis pour un id de tranche (contenant `:`), une cible chargée sans snapshot (`needs_snapshot`), ni une auto-référence dont la base n'est pas dans le Registry.

#### Scenario: Référence mal formée

- **WHEN** la valeur n'a pas de `#`, ou un côté du `#` est vide
- **THEN** une erreur `DEFINITION_009` SHALL être émise (« not a valid element reference »)

#### Scenario: Élément introuvable

- **WHEN** la valeur est bien formée mais l'id n'existe ni dans la définition elle-même ni dans la StructureDefinition visée
- **THEN** une erreur `DEFINITION_009` SHALL être émise (« could not be found »)

#### Scenario: Référence résolue ou exemptée

- **WHEN** l'id existe dans la cible, est un id de tranche, ou la cible n'a pas de snapshot
- **THEN** aucun Diagnostic `DEFINITION_009` SHALL être émis

#### Scenario: Doublon snapshot et différentiel

- **WHEN** l'élément du snapshot porte la même valeur que l'élément du différentiel de même id
- **THEN** un seul Diagnostic SHALL être émis, sur le différentiel

## MODIFIED Requirements

### Requirement: Exécution des manifestes de test officiels

Le crate `fhir-validator` SHALL fournir un test d'intégration exécutant le manifeste `validator/manifest.json` de `fhir-test-cases` pour les cas R5 autonomes (sans `supporting`, `profile`, `profiles`, `packages`, `logical`) dans les taux principaux, JSON (`parse_json`) et XML (`parse_xml`). Le résultat attendu SHALL provenir de l'entrée `java` : le fichier OperationOutcome qu'elle nomme sous `outcomes/`, un `outcome` en ligne ou `errorCount` ; le nombre d'erreurs attendues est le nombre d'issues `error` + `fatal`. Les cas que le manifeste déclare hors périmètre — clé `supporting5` (définitions supplémentaires non chargées) ou `for-publication` valant `true` ou `"true"` (règles de publication HL7) — SHALL être ignorés, comptés par format dans un compteur distinct « skipped (out of scope) » affiché dans le rapport, et exclus du dénominateur du taux de réussite. Les cas sans résultat attendu (fichier d'outcome nommé absent des fixtures) sont comptés comme ignorés ; un fichier d'outcome nommé présent mais illisible ou qui n'est pas un OperationOutcome SHALL faire échouer le test. Chaque cas est jugé sur « présence d'erreurs » contre « erreurs attendues » ; un échec d'analyse (y compris un fichier non UTF-8) compte comme « présence d'erreurs ». Les taux JSON et XML SHALL être affichés séparément (réussites, fausses erreurs, erreurs manquées, échecs d'analyse) et SHALL être au moins 0,87 pour JSON et 0,88 pour XML (planchers de régression). Le test SHALL échouer si, par format, les décomptes (cas jugés, ignorés sans résultat attendu, hors périmètre) diffèrent exactement de JSON (108, 1, 3) et XML (107, 1, 1) sur les fixtures épinglées (dérive des fixtures ou des filtres). En plus du jugement « présence d'erreurs » (inchangé), le test SHALL comparer, pour chaque cas jugé, le nombre de diagnostics de sévérité Error (un échec d'analyse compte pour 1) au nombre d'erreurs attendues (`errorCount` + `fatalCount`, ou issues `error` + `fatal` de l'`outcome` en ligne ou du fichier `outcomes/`), et afficher par format une table des écarts (cas, format, attendu, obtenu, delta) triée par |delta| décroissant puis par nom de fichier, avec un résumé matched/over/under. Le décompte ne SHALL jamais modifier le verdict réussite/échec d'un cas. Le test SHALL échouer si le nombre de cas concordants descend sous le plancher exact `MIN_ERROR_COUNT_MATCHES` = (78, 79) (JSON, XML), à relever à mesure que les décomptes convergent. Les cas R5 portant l'une des clés `supporting`, `profile`, `profiles`, `packages`, `logical` SHALL aller dans un compteur `supporting` distinct (les deux formats ensemble), jamais dans les taux principaux. Un cas portant `profile` (exécution avec profil explicite, `-profile` de Java) est différé et compté « skipped (explicit profile, deferred) ». Sont comptés « skipped (needs snapshot/package/xml) » : les cas avec `packages` ou `logical`, un fichier `supporting`/`profiles` qui n'est pas `.json` ou est absent, un ensemble fourni rejeté par la surcouche (`Error::Unsupported`, ex. une spécialisation ; `Error::Conflict`, ex. deux versions d'un même canonique) et une StructureDefinition fournie sans snapshot (`needs_snapshot`). Les autres SHALL être validés avec `Registry::with_extra(base, fichiers)` et jugés comme les cas principaux, avec leur propre ligne de rapport et table des écarts de décompte. Le test SHALL échouer si les décomptes (jugés, ignorés sans résultat attendu, hors périmètre, needs snapshot/package/xml, profil explicite différé) diffèrent exactement de (42, 0, 0, 68, 69), ou si le nombre de cas réussis ou de concordances de décompte `supporting` descend sous 38 (planchers sans marge). Le répertoire de fixtures SHALL pouvoir être surchargé par `FHIR_FIXTURES_DIR` ; si le manifeste est absent, le test SHALL être ignoré.

#### Scenario: Taux de réussite sous le plancher

- **WHEN** le taux de réussite JSON est inférieur à 0,87 ou le taux XML inférieur à 0,88
- **THEN** le test SHALL échouer

#### Scenario: Cas hors périmètre

- **WHEN** un cas du manifeste porte une clé `supporting5` ou `for-publication` valant `true` ou `"true"`
- **THEN** il SHALL être compté dans « skipped (out of scope) » et exclu du dénominateur du taux de réussite

#### Scenario: Cas avec définitions supplémentaires

- **WHEN** un cas R5 porte `supporting` ou `profiles` (fichiers `.json` chargeables, sans snapshot manquant)
- **THEN** il SHALL être validé avec `Registry::with_extra`, compté dans le tally `supporting` et non dans les taux JSON/XML principaux ; un cas avec `profile` SHALL être compté « skipped (explicit profile, deferred) » et un cas avec `packages`, `logical`, fichier non `.json`, surcouche rejetée ou snapshot manquant « skipped (needs snapshot/package/xml) »

#### Scenario: Sous-module absent

- **WHEN** le manifeste est introuvable
- **THEN** le test SHALL se terminer sans échec

#### Scenario: Écarts de décompte d'erreurs

- **WHEN** le nombre d'erreurs obtenu d'un cas diffère de l'attendu
- **THEN** le cas SHALL figurer dans la table des écarts du format, sans changer son verdict réussite/échec

#### Scenario: Plancher de concordance des décomptes

- **WHEN** le nombre de cas concordants JSON ou XML est inférieur à son plancher (78 pour JSON, 79 pour XML)
- **THEN** le test SHALL échouer

---
