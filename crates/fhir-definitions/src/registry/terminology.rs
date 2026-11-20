//! ValueSet expansion and ConceptMap translation from the loaded definitions.

use std::collections::HashSet;

use super::{MAX_IMPORT_DEPTH, Registry};
use crate::model::{ConceptMap, Include};

/// Expanded codes as `(system, code, display)`.
type Expansion<'a> = Vec<(&'a str, &'a str, Option<&'a str>)>;

impl Registry {
    /// Look up a `ConceptMap` by its canonical URL.
    pub fn concept_map(&self, url: &str) -> Option<&ConceptMap> {
        self.concept_maps
            .get(url)
            .or_else(|| self.base.as_ref()?.concept_map(url))
    }

    /// The codes of a ValueSet as `(system, code, display)`, in compose order,
    /// each `(system, code)` once.
    ///
    /// Only an exact enumeration is returned: every include must name a whole
    /// complete CodeSystem or list its concepts, possibly intersected with
    /// imported ValueSets that expand the same way. `None` for an unknown
    /// ValueSet, a filter, an exclude, a CodeSystem not loaded, incomplete or in
    /// another version, a notSelectable code (it would need the expansion's
    /// `abstract` flag), or imports deeper than the cycle guard. A concept list
    /// stands alone, as in `value_set_contains`: its CodeSystem may be unknown,
    /// and its display comes from the include, else from the CodeSystem.
    ///
    /// shortcut: `compose.inactive` is not read, so a whole CodeSystem gives its
    /// inactive concepts too (R5's usual default); no bundled ValueSet sets it to
    /// false. Read it when a caller needs active-only expansions.
    pub fn expand(&self, url: &str) -> Option<Vec<(&str, &str, Option<&str>)>> {
        self.expand_at(url, 0)
    }

    fn expand_at(&self, url: &str, depth: u8) -> Option<Expansion<'_>> {
        if depth > MAX_IMPORT_DEPTH {
            return None;
        }
        let vs = self.value_set(url)?;
        if vs.has_exclude {
            return None;
        }
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for inc in &vs.includes {
            for entry in self.expand_include(inc, depth)? {
                if seen.insert((entry.0, entry.1)) {
                    out.push(entry);
                }
            }
        }
        Some(out)
    }

    /// One include: its system part (or first import), kept only where every
    /// (other) imported ValueSet has the same `(system, code)`.
    fn expand_include<'a>(&'a self, inc: &'a Include, depth: u8) -> Option<Expansion<'a>> {
        if inc.has_filter {
            return None;
        }
        let (mut entries, imports) = match inc.system.as_deref() {
            Some(system) => (self.expand_system(system, inc)?, &inc.value_sets[..]),
            None => {
                let (first, rest) = inc.value_sets.split_first()?;
                (self.expand_at(first, depth + 1)?, rest)
            }
        };
        for import in imports {
            let kept: HashSet<_> = self
                .expand_at(import, depth + 1)?
                .into_iter()
                .map(|(system, code, _)| (system, code))
                .collect();
            entries.retain(|(system, code, _)| kept.contains(&(*system, *code)));
        }
        Some(entries)
    }

    /// The concepts an include takes from `system`: those it lists, or the whole
    /// CodeSystem when it lists none.
    fn expand_system<'a>(&'a self, system: &'a str, inc: &'a Include) -> Option<Expansion<'a>> {
        let cs = self.include_code_system(system, inc);
        let entries: Expansion<'a> = if inc.concepts.is_empty() {
            let cs = cs.filter(|cs| cs.complete)?;
            cs.concepts
                .iter()
                .map(|c| (system, c.code.as_ref(), c.display.as_deref()))
                .collect()
        } else {
            inc.concepts
                .iter()
                .map(|c| {
                    let display = c.display.as_deref().or_else(|| {
                        cs?.concepts
                            .iter()
                            .find(|known| known.code == c.code)?
                            .display
                            .as_deref()
                    });
                    (system, c.code.as_ref(), display)
                })
                .collect()
        };
        let not_selectable = |code: &str| cs.is_some_and(|cs| cs.not_selectable.contains(code));
        if entries.iter().any(|(_, code, _)| not_selectable(code)) {
            return None;
        }
        Some(entries)
    }

    /// Translate `code` (from `system`, when known) through a ConceptMap, giving
    /// every `(target system, target code, relationship)` its R5 groups list for
    /// it; empty when no group maps it.
    ///
    /// `None` when the answer is not sure: unknown map; no `system` and groups
    /// from several source systems; a `system` and a group without `source`; a
    /// matching group without `target`; a matching element with a target that is
    /// no plain code mapping (no `code` or `relationship`, or a `dependsOn`
    /// condition); or a code a matching group does not list while it declares
    /// `unmapped` (not applied) or keys an element by `valueSet`. `product`
    /// outputs are not returned.
    pub fn translate(
        &self,
        url: &str,
        system: Option<&str>,
        code: &str,
    ) -> Option<Vec<(&str, &str, &str)>> {
        let map = self.concept_map(url)?;
        if system.is_none() {
            let sources: HashSet<_> = map.groups.iter().map(|g| g.source.as_deref()).collect();
            if sources.len() > 1 {
                return None;
            }
        }
        let mut matches = Vec::new();
        for group in &map.groups {
            match (system, group.source.as_deref()) {
                (Some(_), None) => return None,
                (Some(system), Some(source)) if system != source => continue,
                _ => {}
            }
            let mut listed = false;
            for element in group.elements.iter().filter(|e| e.code.as_ref() == code) {
                listed = true;
                if element.undecided {
                    return None;
                }
                for target in &element.targets {
                    let target_system = group.target.as_deref()?;
                    matches.push((
                        target_system,
                        target.code.as_ref(),
                        target.relationship.as_ref(),
                    ));
                }
            }
            if !listed && group.unlisted_undecided {
                return None;
            }
        }
        Some(matches)
    }
}
