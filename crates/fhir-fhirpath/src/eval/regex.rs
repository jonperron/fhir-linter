use std::cell::RefCell;
use std::collections::HashMap;

use regex_lite::Regex;

use super::helpers::check_string_len;
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
    let re = build_regex(pattern, flags, "replaceMatches()")?;
    // Expand one group reference at a time, so each step grows the result by
    // at most one piece plus one match before the length check runs.
    let pieces = replacement_pieces(replacement);
    let mut result = String::new();
    let mut chars = 0;
    let mut last = 0;
    for caps in re.captures_iter(text) {
        let Some(found) = caps.get(0) else { continue };
        let gap = &text[last..found.start()];
        chars += gap.chars().count();
        check_string_len(chars)?;
        result.push_str(gap);
        for piece in &pieces {
            let before = result.len();
            caps.expand(piece, &mut result);
            chars += result[before..].chars().count();
            check_string_len(chars)?;
        }
        last = found.end();
    }
    let tail = &text[last..];
    check_string_len(chars + tail.chars().count())?;
    result.push_str(tail);
    Ok(result)
}

/// Split a replacement before each `$` token (`$$`, `${name}` and `$name`
/// kept whole), so every piece holds at most one group reference and
/// expanding the pieces in turn gives the same text as the whole.
fn replacement_pieces(replacement: &str) -> Vec<&str> {
    let bytes = replacement.as_bytes();
    // Every `}` position, found once: a rescan per `${` would be quadratic.
    let closes: Vec<usize> = replacement.match_indices('}').map(|(at, _)| at).collect();
    let mut starts = vec![0];
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'$' {
            index += 1;
            continue;
        }
        if index > 0 {
            starts.push(index);
        }
        index += match bytes.get(index + 1) {
            Some(b'$') => 2,
            // An unclosed `${` is a literal `$`: what follows may hold references.
            Some(b'{') => closes
                .get(closes.partition_point(|close| *close < index))
                .map_or(1, |close| close - index + 1),
            _ => 1,
        };
    }
    starts.push(bytes.len());
    starts
        .windows(2)
        .map(|w| &replacement[w[0]..w[1]])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // The piecewise expansion must match regex-lite's own `replace_all`.
    #[test]
    fn piecewise_expansion_matches_replace_all() -> Result<(), EvalError> {
        let text = "ab-cd ab é";
        let pattern = r"(?<x>a)(b)|(?<y>é)";
        let re = build_regex(pattern, "", "test")?;
        for replacement in [
            "", "z", "$1", "${1}x", "$1x", "$$1", "$$$1", "a$", "$", "$$", "${x}${y}", "${x$y}",
            "${1", "x${", "$2$1$0", "[$x]", "$x$", "é$1é", "$name", "${$1", "${a$1", "${a$1$1",
            "${}", "${}$1", "${a$1}", "${a$1$1}",
        ] {
            assert_eq!(
                regex_replace_all(text, pattern, replacement, "")?,
                re.replace_all(text, replacement),
                "{replacement}"
            );
        }
        Ok(())
    }

    // regex-lite reads an unclosed `${` as a literal `$`, so the references
    // after it are live and must land in pieces of their own.
    #[test]
    fn unclosed_brace_does_not_merge_references() {
        assert_eq!(replacement_pieces("${$1$1"), ["${", "$1", "$1"]);
        assert_eq!(replacement_pieces("a${b}$1"), ["a", "${b}", "$1"]);
    }

    // Many unclosed `${` must not rescan the rest of the replacement each
    // time: 200,000 of them took seconds when the split was quadratic.
    #[test]
    fn unclosed_braces_split_in_linear_time() {
        let replacement = "${".repeat(200_000);
        let started = std::time::Instant::now();
        assert_eq!(replacement_pieces(&replacement).len(), 200_000);
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
    }

    // Multi-digit group numbers stay one reference: `$10` is group 10.
    #[test]
    fn multi_digit_group_reference_expands_whole() -> Result<(), EvalError> {
        let pattern = "(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)";
        assert_eq!(
            regex_replace_all("abcdefghij", pattern, "[$10|${1}0]", "")?,
            "[j|a0]"
        );
        Ok(())
    }
}
