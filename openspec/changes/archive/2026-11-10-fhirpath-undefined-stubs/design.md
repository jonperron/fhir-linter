## Context

Les stubs renvoyaient un résultat arbitraire pour des fonctions nécessitant un serveur terminologique, une résolution de références, une horloge ou une arithmétique de précision non implémentée. Le validateur sait déjà ignorer une contrainte dont l'évaluation échoue.

## Goals / Non-Goals

**Goals:** ne jamais juger un invariant sur un résultat deviné ; s'appuyer sur le chemin `INVARIANT_SKIPPED` existant.

**Non-Goals:** implémenter réellement `resolve`, `memberOf`, `conformsTo`, les fonctions d'horloge ou de bornes.

## Decisions

- Retirer les stubs plutôt que de les garder derrière une option : la fonction tombe dans le cas générique « fonction inconnue », qui retourne `EvalError::UndefinedFunction(name)`. Aucune nouvelle variante d'erreur.
- `today()`/`now()`/`timeOfDay()` sont aussi retirées, comme les autres stubs, en attendant une implémentation réelle.

## Risks / Trade-offs

- Davantage de contraintes ignorées (Information) : couverture apparente réduite, mais aucun faux résultat. Mitigation : diagnostic `INVARIANT_SKIPPED` explicite nommant la `key` et la raison.
