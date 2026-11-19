## MODIFIED Requirements

### Requirement: Exécution des manifestes de test officiels

Le crate `fhir-validator` SHALL fournir un test d'intégration exécutant le manifeste `validator/manifest.json` de `fhir-test-cases` pour les cas R5 autonomes (sans `supporting`, `profile`, `profiles`, `packages`, `logical`), JSON (`parse_json`) et XML (`parse_xml`). Le résultat attendu SHALL provenir de l'entrée `java` : le fichier OperationOutcome qu'elle nomme sous `outcomes/`, un `outcome` en ligne ou `errorCount` ; le nombre d'erreurs attendues est le nombre d'issues `error` + `fatal`. Les cas que le manifeste déclare hors périmètre — clé `supporting5` (définitions supplémentaires non chargées) ou `for-publication` valant `true` ou `"true"` (règles de publication HL7) — SHALL être ignorés, comptés par format dans un compteur distinct « skipped (out of scope) » affiché dans le rapport, et exclus du dénominateur du taux de réussite. Les cas sans résultat attendu (fichier d'outcome nommé absent des fixtures) sont comptés comme ignorés ; un fichier d'outcome nommé présent mais illisible ou qui n'est pas un OperationOutcome SHALL faire échouer le test. Chaque cas est jugé sur « présence d'erreurs » contre « erreurs attendues » ; un échec d'analyse (y compris un fichier non UTF-8) compte comme « présence d'erreurs ». Les taux JSON et XML SHALL être affichés séparément (réussites, fausses erreurs, erreurs manquées, échecs d'analyse) et SHALL être au moins 0,87 pour JSON et 0,88 pour XML (planchers de régression). Le test SHALL échouer si, par format, les décomptes (cas jugés, ignorés sans résultat attendu, hors périmètre) diffèrent exactement de JSON (108, 1, 3) et XML (107, 1, 1) sur les fixtures épinglées (dérive des fixtures ou des filtres). En plus du jugement « présence d'erreurs » (inchangé), le test SHALL comparer, pour chaque cas jugé, le nombre de diagnostics de sévérité Error (un échec d'analyse compte pour 1) au nombre d'erreurs attendues (`errorCount` + `fatalCount`, ou issues `error` + `fatal` de l'`outcome` en ligne ou du fichier `outcomes/`), et afficher par format une table des écarts (cas, format, attendu, obtenu, delta) triée par |delta| décroissant puis par nom de fichier, avec un résumé matched/over/under. Le décompte ne SHALL jamais modifier le verdict réussite/échec d'un cas. Le test SHALL échouer si le nombre de cas concordants descend sous le plancher exact `MIN_ERROR_COUNT_MATCHES` = (78, 78) (JSON, XML), à relever à mesure que les décomptes convergent. Le répertoire de fixtures SHALL pouvoir être surchargé par `FHIR_FIXTURES_DIR` ; si le manifeste est absent, le test SHALL être ignoré.

#### Scenario: Taux de réussite sous le plancher

- **WHEN** le taux de réussite JSON est inférieur à 0,87 ou le taux XML inférieur à 0,88
- **THEN** le test SHALL échouer

#### Scenario: Cas hors périmètre

- **WHEN** un cas du manifeste porte une clé `supporting5` ou `for-publication` valant `true` ou `"true"`
- **THEN** il SHALL être compté dans « skipped (out of scope) » et exclu du dénominateur du taux de réussite

#### Scenario: Sous-module absent

- **WHEN** le manifeste est introuvable
- **THEN** le test SHALL se terminer sans échec

#### Scenario: Écarts de décompte d'erreurs

- **WHEN** le nombre d'erreurs obtenu d'un cas diffère de l'attendu
- **THEN** le cas SHALL figurer dans la table des écarts du format, sans changer son verdict réussite/échec

#### Scenario: Plancher de concordance des décomptes

- **WHEN** le nombre de cas concordants JSON ou XML est inférieur à 78
- **THEN** le test SHALL échouer
