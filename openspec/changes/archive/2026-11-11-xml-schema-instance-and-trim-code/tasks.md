## 1. Parseur XML

- [x] 1.1 Ajouter `Resource::schema_instance: Vec<Span>` (vide pour le JSON)
- [x] 1.2 Enregistrer une fois chaque élément déclarant ou utilisant le namespace XMLSchema-instance, sans erreur de parse
- [x] 1.3 Résoudre les références de caractère du texte brut du div pour la règle XML 1.0

## 2. Validateur

- [x] 2.1 Émettre `TYPE_007` (Warning) pour les valeurs d'attribut épurées
- [x] 2.2 Émettre `STRUCTURE_003` (Error) par élément de `schema_instance`

## 3. Tests

- [x] 3.1 Couvrir les positions schema instance et les références de contrôle du div
- [x] 3.2 Ajuster le plancher du manifeste XML
