use std::sync::Arc;

use crate::ast::Expr;
use crate::error::EvalError;
use crate::value::{Collection, Value};

use super::Evaluator;
use super::base64::{base64_decode, base64_decode_url, base64_encode, base64_encode_url};
use super::helpers::{
    check_arity, check_arity_range, eval_string_arg, json_unescape, single_integer,
    string_transform,
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
            "toString" => {
                check_arity(name, args, 0)?;
                focus
                    .iter()
                    .map(|v| Value::String(Arc::from(v.to_string().as_str())))
                    .collect()
            }
            "length" => {
                check_arity(name, args, 0)?;
                match focus.first() {
                    Some(Value::String(s)) => vec![Value::Integer(s.chars().count() as i64)],
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "length() requires String, got {}",
                            v.type_name()
                        )));
                    }
                    None => vec![],
                }
            }
            "startsWith" => {
                check_arity(name, args, 1)?;
                let prefix = optional_string_arg(&self.eval(&args[0], ctx)?, "startsWith")?;
                string_predicate(focus, prefix, |s, prefix| s.starts_with(prefix))
            }
            "endsWith" => {
                check_arity(name, args, 1)?;
                let suffix = optional_string_arg(&self.eval(&args[0], ctx)?, "endsWith")?;
                string_predicate(focus, suffix, |s, suffix| s.ends_with(suffix))
            }
            "contains" => {
                check_arity(name, args, 1)?;
                let sub = optional_string_arg(&self.eval(&args[0], ctx)?, "contains")?;
                string_predicate(focus, sub, |s, sub| s.contains(sub))
            }
            "upper" => {
                check_arity(name, args, 0)?;
                return string_transform(focus, |s| s.to_uppercase()).map(Some);
            }
            "lower" => {
                check_arity(name, args, 0)?;
                return string_transform(focus, |s| s.to_lowercase()).map(Some);
            }
            "trim" => {
                check_arity(name, args, 0)?;
                return string_transform(focus, |s| s.trim().to_owned()).map(Some);
            }
            "escape" => {
                check_arity(name, args, 1)?;
                let target = eval_string_arg(&self.eval(&args[0], ctx)?, "escape")?;
                match focus.first().and_then(|v| v.as_string()) {
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
                        vec![Value::String(Arc::from(result.as_str()))]
                    }
                    None => vec![],
                }
            }
            "unescape" => {
                check_arity(name, args, 1)?;
                let target = eval_string_arg(&self.eval(&args[0], ctx)?, "unescape")?;
                match focus.first().and_then(|v| v.as_string()) {
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
                        vec![Value::String(Arc::from(result.as_str()))]
                    }
                    None => vec![],
                }
            }
            "substring" => {
                check_arity_range(name, args, 1, 2)?;
                let s = match focus.first().and_then(|v| v.as_string()) {
                    Some(s) => s,
                    None => return Ok(Some(vec![])),
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
                match (
                    focus.first().and_then(|v| v.as_string()),
                    pattern,
                    replacement,
                ) {
                    (Some(s), Some(pattern), Some(replacement)) => {
                        let result = s.replace(pattern.as_ref(), replacement.as_ref());
                        vec![Value::String(Arc::from(result.as_str()))]
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
                match (focus.first(), pattern) {
                    (Some(item), Some(pattern)) => {
                        let text = item.as_string().unwrap_or_else(|| Arc::from(""));
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
                match (
                    focus.first().and_then(|v| v.as_string()),
                    pattern,
                    replacement,
                ) {
                    (Some(s), Some(pattern), Some(replacement)) => {
                        let result = regex_replace_all(&s, &pattern, &replacement, &flags)?;
                        vec![Value::String(Arc::from(result.as_str()))]
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
                match (focus.first(), pattern) {
                    (Some(item), Some(pattern)) => {
                        let text = item.as_string().unwrap_or_else(|| Arc::from(""));
                        vec![Value::Bool(regex_matches_full(&text, &pattern, &flags)?)]
                    }
                    _ => vec![],
                }
            }
            "indexOf" => {
                check_arity(name, args, 1)?;
                let sub = optional_string_arg(&self.eval(&args[0], ctx)?, "indexOf")?;
                let (Some(item), Some(sub)) = (focus.first(), sub) else {
                    return Ok(Some(vec![]));
                };
                match item.as_string() {
                    Some(s) => {
                        let idx = s
                            .char_indices()
                            .enumerate()
                            .find(|(_, (byte_pos, _))| s[*byte_pos..].starts_with(sub.as_ref()))
                            .map(|(char_idx, _)| char_idx as i64)
                            .unwrap_or(-1);
                        vec![Value::Integer(idx)]
                    }
                    None => vec![Value::Integer(-1)],
                }
            }
            "lastIndexOf" => {
                check_arity(name, args, 1)?;
                let sub = optional_string_arg(&self.eval(&args[0], ctx)?, "lastIndexOf")?;
                let (Some(item), Some(sub)) = (focus.first(), sub) else {
                    return Ok(Some(vec![]));
                };
                match item.as_string() {
                    Some(s) => {
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
                    None => vec![Value::Integer(-1)],
                }
            }
            "split" => {
                check_arity(name, args, 1)?;
                let sep = eval_string_arg(&self.eval(&args[0], ctx)?, "split")?;
                match focus.first().and_then(|v| v.as_string()) {
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
                let joined: Vec<String> = focus
                    .iter()
                    .filter_map(|v| v.as_string())
                    .map(|s| s.to_string())
                    .collect();
                vec![Value::String(Arc::from(joined.join(sep.as_ref()).as_str()))]
            }
            "encode" => {
                check_arity(name, args, 1)?;
                let format = eval_string_arg(&self.eval(&args[0], ctx)?, "encode")?;
                match focus.first().and_then(|v| v.as_string()) {
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
                        vec![Value::String(Arc::from(result.as_str()))]
                    }
                    None => vec![],
                }
            }
            "decode" => {
                check_arity(name, args, 1)?;
                let format = eval_string_arg(&self.eval(&args[0], ctx)?, "decode")?;
                match focus.first().and_then(|v| v.as_string()) {
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
                        vec![Value::String(Arc::from(decoded.as_str()))]
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
    arg: Option<Arc<str>>,
    test: impl Fn(&str, &str) -> bool,
) -> Collection {
    match (focus.first(), arg) {
        (Some(item), Some(arg)) => {
            vec![Value::Bool(
                item.as_string().is_some_and(|s| test(&s, &arg)),
            )]
        }
        _ => vec![],
    }
}
