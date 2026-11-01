use std::cell::RefCell;
use std::collections::HashMap;

use regex_lite::Regex;

use crate::error::EvalError;

thread_local! {
    static CACHE: RefCell<HashMap<String, Regex>> = RefCell::new(HashMap::new());
}

/// Build (or fetch from the per-thread cache) a `Regex` with optional FHIRPath 3.0 flags
/// (`i`, `m`, `s`).
pub(super) fn build_regex(pattern: &str, flags: &str, context: &str) -> Result<Regex, EvalError> {
    let prefix: String = flags
        .chars()
        .filter(|c| matches!(c, 'i' | 'm' | 's'))
        .map(|c| format!("(?{c})"))
        .collect();
    let full = format!("{prefix}{pattern}");
    CACHE.with(|cache| {
        if let Some(re) = cache.borrow().get(&full) {
            return Ok(re.clone());
        }
        let re = Regex::new(&full)
            .map_err(|e| EvalError::Type(format!("{context}: invalid pattern: {e}")))?;
        let mut cache = cache.borrow_mut();
        if cache.len() >= 256 {
            cache.clear();
        }
        cache.insert(full, re.clone());
        Ok(re)
    })
}

pub(super) fn regex_matches(text: &str, pattern: &str, flags: &str) -> Result<bool, EvalError> {
    build_regex(pattern, flags, "matches()").map(|re| re.is_match(text))
}

pub(super) fn regex_matches_full(
    text: &str,
    pattern: &str,
    flags: &str,
) -> Result<bool, EvalError> {
    let anchored = format!(r"\A(?:{pattern})\z");
    build_regex(&anchored, flags, "matchesFull()").map(|re| re.is_match(text))
}

pub(super) fn regex_replace_all(
    text: &str,
    pattern: &str,
    replacement: &str,
    flags: &str,
) -> Result<String, EvalError> {
    build_regex(pattern, flags, "replaceMatches()")
        .map(|re| re.replace_all(text, replacement).into_owned())
}
