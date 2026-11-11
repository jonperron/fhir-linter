## 1. Parseur XML

- [x] 1.1 Rejeter DTD, encodage non UTF-8, texte/entités/CDATA entre éléments
- [x] 1.2 Contrôler namespaces FHIR/XHTML, attributs de la racine, attributs dans un namespace, XMLSchema-instance, 8 déclarations au plus
- [x] 1.3 Refuser attribut et élément de même nom
- [x] 1.4 Conserver le div comme tranche de source brute ; borner la profondeur dans le div
- [x] 1.5 Épurer les valeurs d'attribut et remplir `Resource::trimmed_attributes`

## 2. Registry

- [x] 2.1 Ajouter `ElementDefinition.xml_attr` depuis `representation`

## 3. Validateur

- [x] 3.1 `STRUCTURE_001` pour une forme attribut/élément contraire à `xml_attr`
- [x] 3.2 `TYPE_006` Warning pour une valeur d'attribut épurée

## 4. Linter

- [x] 4.1 `narrative-absent` analyse le div XML comme XHTML

## 5. Tests

- [x] 5.1 Tests de conformité XML du parseur, tests du remodelage et du linter
