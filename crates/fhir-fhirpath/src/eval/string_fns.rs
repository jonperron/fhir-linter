use std::sync::Arc;

use crate::ast::Expr;
use crate::error::EvalError;
use crate::value::{Collection, Value};

use super::Evaluator;
use super::base64::{base64_decode, base64_decode_url, base64_encode, base64_encode_url};
use super::helpers::{
    check_arity, check_arity_range, check_string_len, eval_string_arg, json_unescape,
    single_integer, string_input, string_transform, string_value,
};
use super::regex::{regex_matches, regex_matches_full, regex_replace_all};

impl Evaluator<'_> {
    /// Handle string manipulation functions.
    /// Returns `None` if the function name is not in this category.
    pub(super) fn call_string_function(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Option<Result<Collection, EvalError>> {
        self.try_string(name, args, focus, ctx).transpose()
    }

    #[allow(clippy::too_many_lines)]
    fn try_string(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Result<Option<Collection>, EvalError> {
        let col: Collection = match name {
            "length" => {
                check_arity(name, args, 0)?;
                string_input(focus, name)?
                    .map(|s| Value::Integer(s.chars().count() as i64))
                    .into_iter()
                    .collect()
            }
            "startsWith" => {
                check_arity(name, args, 1)?;
                let prefix = optional_string_arg(&self.eval(&args[0], ctx)?, "startsWith")?;
                string_predicate(focus, name, prefix, |s, prefix| s.starts_with(prefix))?
            }
            "endsWith" => {
                check_arity(name, args, 1)?;
                let suffix = optional_string_arg(&self.eval(&args[0], ctx)?, "endsWith")?;
                string_predicate(focus, name, suffix, |s, suffix| s.ends_with(suffix))?
            }
            "contains" => {
                check_arity(name, args, 1)?;
                let sub = optional_string_arg(&self.eval(&args[0], ctx)?, "contains")?;
                string_predicate(focus, name, sub, |s, sub| s.contains(sub))?
            }
            "upper" => {
                check_arity(name, args, 0)?;
                return string_transform(focus, name, str::to_uppercase).map(Some);
            }
            "lower" => {
                check_arity(name, args, 0)?;
                return string_transform(focus, name, str::to_lowercase).map(Some);
            }
            "trim" => {
                check_arity(name, args, 0)?;
                return string_transform(focus, name, |s| s.trim().to_owned()).map(Some);
            }
            "escape" => {
                check_arity(name, args, 1)?;
                let target = eval_string_arg(&self.eval(&args[0], ctx)?, "escape")?;
                match string_input(focus, name)? {
                    Some(s) => {
                        let result = match target.as_ref() {
                            "html" => s
                                .replace('&', "&amp;")
                                .replace('<', "&lt;")
                                .replace('>', "&gt;")
                                .replace('"', "&quot;")
                                .replace('\'', "&#39;"),
                            "json" => s
                                .replace('\\', "\\\\")
                                .replace('"', "\\\"")
                                .replace('\n', "\\n")
                                .replace('\r', "\\r")
                                .replace('\t', "\\t"),
                            other => {
                                return Err(EvalError::Type(format!(
                                    "escape(): unknown target '{other}'; expected 'html' or 'json'"
                                )));
                            }
                        };
                        vec![string_value(&result)?]
                    }
                    None => vec![],
                }
            }
            "unescape" => {
                check_arity(name, args, 1)?;
                let target = eval_string_arg(&self.eval(&args[0], ctx)?, "unescape")?;
                match string_input(focus, name)? {
                    Some(s) => {
                        let result = match target.as_ref() {
                            "html" => s
                                .replace("&lt;", "<")
                                .replace("&gt;", ">")
                                .replace("&quot;", "\"")
                                .replace("&#39;", "'")
                                .replace("&amp;", "&"),
                            "json" => json_unescape(&s),
                            other => {
                                return Err(EvalError::Type(format!(
                                    "unescape(): unknown target '{other}'; expected 'html' or 'json'"
                                )));
                            }
                        };
                        vec![string_value(&result)?]
                    }
                    None => vec![],
                }
            }
            "substring" => {
                check_arity_range(name, args, 1, 2)?;
                let Some(s) = string_input(focus, name)? else {
                    return Ok(Some(vec![]));
                };
                let chars: Vec<char> = s.chars().collect();
                let start_col = self.eval(&args[0], ctx)?;
                if start_col.is_empty() {
                    return Ok(Some(vec![]));
                }
                let start_i = single_integer(&start_col, "substring start")?;
                // N1: a start outside the string (0..length) gives empty.
                let Some(start) = usize::try_from(start_i).ok().filter(|s| *s < chars.len()) else {
                    return Ok(Some(vec![]));
                };
                // N1: an empty length behaves as if no length was given.
                let len_col = match args.get(1) {
                    Some(arg) => self.eval(arg, ctx)?,
                    None => vec![],
                };
                let end = if len_col.is_empty() {
                    chars.len()
                } else {
                    let len_i = single_integer(&len_col, "substring length")?;
                    if len_i <= 0 {
                        return Ok(Some(vec![Value::String(Arc::from(""))]));
                    }
                    // Clamp: on 32-bit targets an i64 length can exceed usize.
                    let len = usize::try_from(len_i).unwrap_or(usize::MAX);
                    start.saturating_add(len).min(chars.len())
                };
                let sub: String = chars[start..end].iter().collect();
                vec![Value::String(Arc::from(sub.as_str()))]
            }
            "replace" => {
                check_arity(name, args, 2)?;
                let pattern = optional_string_arg(&self.eval(&args[0], ctx)?, "replace pattern")?;
                let replacement =
                    optional_string_arg(&self.eval(&args[1], ctx)?, "replace replacement")?;
                match (string_input(focus, name)?, pattern, replacement) {
                    (Some(s), Some(pattern), Some(replacement)) => {
                        // Size the result before building it: an empty pattern
                        // inserts the replacement at every character boundary.
                        let hits = s.matches(pattern.as_ref()).count();
                        let kept = s
                            .chars()
                            .count()
                            .saturating_sub(hits * pattern.chars().count());
                        check_string_len(
                            kept.saturating_add(hits.saturating_mul(replacement.chars().count())),
                        )?;
                        let result = s.replace(pattern.as_ref(), replacement.as_ref());
                        vec![string_value(&result)?]
                    }
                    _ => vec![],
                }
            }
            "matches" => {
                check_arity_range(name, args, 1, 2)?;
                let pattern = optional_string_arg(&self.eval(&args[0], ctx)?, "matches pattern")?;
                let flags = if args.len() == 2 {
                    eval_string_arg(&self.eval(&args[1], ctx)?, "matches flags")?
                } else {
                    Arc::from("")
                };
                match (string_input(focus, name)?, pattern) {
                    (Some(text), Some(pattern)) => {
                        vec![Value::Bool(regex_matches(&text, &pattern, &flags)?)]
                    }
                    _ => vec![],
                }
            }
            "replaceMatches" => {
                check_arity_range(name, args, 2, 3)?;
                let pattern =
                    optional_string_arg(&self.eval(&args[0], ctx)?, "replaceMatches pattern")?;
                let replacement =
                    optional_string_arg(&self.eval(&args[1], ctx)?, "replaceMatches replacement")?;
                let flags = if args.len() == 3 {
                    eval_string_arg(&self.eval(&args[2], ctx)?, "replaceMatches flags")?
                } else {
                    Arc::from("")
                };
                match (string_input(focus, name)?, pattern, replacement) {
                    (Some(s), Some(pattern), Some(replacement)) => {
                        let result = regex_replace_all(&s, &pattern, &replacement, &flags)?;
                        vec![string_value(&result)?]
                    }
                    _ => vec![],
                }
            }
            "matchesFull" => {
                check_arity_range(name, args, 1, 2)?;
                let pattern =
                    optional_string_arg(&self.eval(&args[0], ctx)?, "matchesFull pattern")?;
                let flags = if args.len() == 2 {
                    eval_string_arg(&self.eval(&args[1], ctx)?, "matchesFull flags")?
                } else {
                    Arc::from("")
                };
                match (string_input(focus, name)?, pattern) {
                    (Some(text), Some(pattern)) => {
                        vec![Value::Bool(regex_matches_full(&text, &pattern, &flags)?)]
                    }
                    _ => vec![],
                }
            }
            "indexOf" => {
                check_arity(name, args, 1)?;
                let sub = optional_string_arg(&self.eval(&args[0], ctx)?, "indexOf")?;
                let (Some(s), Some(sub)) = (string_input(focus, name)?, sub) else {
                    return Ok(Some(vec![]));
                };
                let idx = s
                    .char_indices()
                    .enumerate()
                    .find(|(_, (byte_pos, _))| s[*byte_pos..].starts_with(sub.as_ref()))
                    .map(|(char_idx, _)| char_idx as i64)
                    .unwrap_or(-1);
                vec![Value::Integer(idx)]
            }
            "lastIndexOf" => {
                check_arity(name, args, 1)?;
                let sub = optional_string_arg(&self.eval(&args[0], ctx)?, "lastIndexOf")?;
                let (Some(s), Some(sub)) = (string_input(focus, name)?, sub) else {
                    return Ok(Some(vec![]));
                };
                let chars: Vec<char> = s.chars().collect();
                let sub_chars: Vec<char> = sub.chars().collect();
                let mut last: i64 = -1;
                if !sub_chars.is_empty() && sub_chars.len() <= chars.len() {
                    for i in 0..=(chars.len() - sub_chars.len()) {
                        if chars[i..i + sub_chars.len()] == sub_chars[..] {
                            last = i as i64;
                        }
                    }
                } else if sub_chars.is_empty() {
                    last = chars.len() as i64;
                }
                vec![Value::Integer(last)]
            }
            "split" => {
                check_arity(name, args, 1)?;
                let sep = eval_string_arg(&self.eval(&args[0], ctx)?, "split")?;
                match string_input(focus, name)? {
                    Some(s) => s
                        .split(sep.as_ref())
                        .map(|part| Value::String(Arc::from(part)))
                        .collect(),
                    None => vec![],
                }
            }
            "join" => {
                check_arity_range(name, args, 0, 1)?;
                let sep = if args.is_empty() {
                    Arc::from("")
                } else {
                    eval_string_arg(&self.eval(&args[0], ctx)?, "join separator")?
                };
                if focus.is_empty() {
                    return Ok(Some(vec![]));
                }
                let joined: Vec<Arc<str>> = focus.iter().filter_map(|v| v.as_string()).collect();
                let separator_count = joined.len().saturating_sub(1);
                let total = joined.iter().map(|s| s.chars().count()).fold(
                    separator_count.saturating_mul(sep.chars().count()),
                    usize::saturating_add,
                );
                check_string_len(total)?;
                vec![Value::String(Arc::from(joined.join(sep.as_ref()).as_str()))]
            }
            "encode" => {
                check_arity(name, args, 1)?;
                let format = eval_string_arg(&self.eval(&args[0], ctx)?, "encode")?;
                match string_input(focus, name)? {
                    Some(s) => {
                        let result = match format.as_ref() {
                            "base64" => base64_encode(s.as_bytes()),
                            "urlbase64" => base64_encode_url(s.as_bytes()),
                            other => {
                                return Err(EvalError::Type(format!(
                                    "unknown encode format: {other}"
                                )));
                            }
                        };
                        vec![string_value(&result)?]
                    }
                    None => vec![],
                }
            }
            "decode" => {
                check_arity(name, args, 1)?;
                let format = eval_string_arg(&self.eval(&args[0], ctx)?, "decode")?;
                match string_input(focus, name)? {
                    Some(s) => {
                        let bytes = match format.as_ref() {
                            "base64" => base64_decode(s.as_bytes()),
                            "urlbase64" => base64_decode_url(s.as_bytes()),
                            other => {
                                return Err(EvalError::Type(format!(
                                    "unknown decode format: {other}"
                                )));
                            }
                        };
                        let decoded = bytes
                            .ok_or_else(|| EvalError::Type("decode: invalid base64".into()))
                            .and_then(|b| {
                                String::from_utf8(b)
                                    .map_err(|_| EvalError::Type("decode: invalid UTF-8".into()))
                            })?;
                        vec![string_value(&decoded)?]
                    }
                    None => vec![],
                }
            }

            _ => return Ok(None),
        };
        Ok(Some(col))
    }
}

/// A String argument that may be empty: N1 §5.6 makes the call return empty
/// when it is. Any other non-String value is still a type error.
fn optional_string_arg(col: &Collection, context: &str) -> Result<Option<Arc<str>>, EvalError> {
    if col.is_empty() {
        return Ok(None);
    }
    eval_string_arg(col, context).map(Some)
}

/// startsWith / endsWith / contains: empty input or argument gives empty.
fn string_predicate(
    focus: &[Value],
    name: &str,
    arg: Option<Arc<str>>,
    test: impl Fn(&str, &str) -> bool,
) -> Result<Collection, EvalError> {
    Ok(match (string_input(focus, name)?, arg) {
        (Some(s), Some(arg)) => vec![Value::Bool(test(&s, &arg))],
        _ => vec![],
    })
}
