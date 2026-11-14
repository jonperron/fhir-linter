## MODIFIED Requirements

### Requirement: Type Resource

Le système SHALL exposer un type `Resource` représentant une ressource FHIR parsée depuis JSON.

#### Scenario: Champs obligatoires

- **WHEN** une ressource FHIR JSON est parsée avec succès
- **THEN** le `Resource` retourné SHALL contenir `resource_type: Arc<str>` (valeur du champ `"resourceType"`), `id: Option<ResourceId>` (`value` et `span` de la valeur du champ `"id"` si présent ; déréférence en `str`), et `fields: IndexMap<Arc<str>, Node>` (tous les champs restants)
