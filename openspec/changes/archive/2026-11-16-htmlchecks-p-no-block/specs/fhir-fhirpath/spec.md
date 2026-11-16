## MODIFIED Requirements

### Requirement: Fonction htmlChecks (extension FHIR)

`htmlChecks()` SHALL retourner un Boolean pour une entrée à une seule chaîne, et la collection vide pour une entrée vide ou non chaîne. Elle SHALL retourner `true` quand le balisage (un `div` ou un fragment) ne contient que les éléments autorisés (HTML 4.0 chapitres 7 à 11 sauf 9.4, chapitre 15, non dépréciés, plus `a`, `img`, `map`, `area`) sans espace de noms ou dans l'espace de noms XHTML ; uniquement des attributs autorisés hors événements (liste plate, raccourci documenté) ; aucune URL `javascript:`, `vbscript:` ou `data:` non image dans `href`, `src`, `cite`, `longdesc`, `usemap` ; aucun DTD, déclaration ou instruction de traitement ; aucun `<` ou `>` dans les commentaires ou sections CDATA ; et au moins du texte non blanc (espaces Unicode, y compris en CDATA, non comptés), une entité prédéfinie XML ou un `img`. Une entité nommée autre que les cinq prédéfinies (`&nbsp;`, `&Tab;`), une référence de caractère invalide ou un balisage mal formé SHALL donner `false`. Un élément de type %block (`p`, `h1` à `h6`, `ul`, `ol`, `pre`, `dl`, `div`, `blockquote`, `hr`, `table`, `address`) situé n'importe où à l'intérieur d'un `p` SHALL donner `false` (HTML 4.01 `<!ELEMENT P - O (%inline;)*>`) ; un descendant `map` réinitialise cette règle, car MAP contient %block.

#### Scenario: Div valide

- **WHEN** `htmlChecks()` est évalué sur `<div xmlns="http://www.w3.org/1999/xhtml">texte</div>`
- **THEN** le résultat SHALL être `true`

#### Scenario: Contenu interdit

- **WHEN** l'entrée contient `<script>`, un attribut `onclick`, un lien `javascript:` ou un balisage mal formé
- **THEN** le résultat SHALL être `false`

#### Scenario: Entité non définie

- **WHEN** `htmlChecks()` est évalué sur un `div` contenant `&nbsp;`
- **THEN** le résultat SHALL être `false`

#### Scenario: Entrée vide ou non chaîne

- **WHEN** l'entrée est vide ou n'est pas une chaîne
- **THEN** le résultat SHALL être la collection vide

#### Scenario: Bloc dans un paragraphe

- **WHEN** `htmlChecks()` est évalué sur `<p>a<div>b</div></p>` puis sur `<p>a</p><div><p>b</p></div>`
- **THEN** le premier résultat SHALL être `false` et le second `true`

