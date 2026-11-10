use std::sync::Arc;

use crate::ast::Expr;
use crate::error::EvalError;
use crate::value::{Collection, Value};

use super::Evaluator;
use super::helpers::{check_arity, check_arity_range, eval_string_arg};

impl Evaluator {
    /// Handle miscellaneous functions (logic, system, FHIR-specific stubs).
    /// Returns `None` if the function name is not in this category.
    pub(super) fn call_misc_function(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Option<Result<Collection, EvalError>> {
        self.try_misc(name, args, focus, ctx).transpose()
    }

    fn try_misc(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Result<Option<Collection>, EvalError> {
        let col: Collection = match name {
            // ── Logic ─────────────────────────────────────────────────────
            "not" => {
                check_arity(name, args, 0)?;
                match focus.first() {
                    Some(Value::Bool(b)) => vec![Value::Bool(!b)],
                    None => vec![],
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "not() requires Boolean, got {}",
                            v.type_name()
                        )));
                    }
                }
            }
            "iif" => {
                check_arity_range(name, args, 2, 3)?;
                let cond = self.eval(&args[0], focus)?;
                let truthy = cond.iter().any(Value::is_truthy);
                let result = if truthy {
                    self.eval(&args[1], focus)?
                } else if args.len() == 3 {
                    self.eval(&args[2], focus)?
                } else {
                    vec![]
                };
                return Ok(Some(result));
            }

            // ── System / environment ───────────────────────────────────────
            "trace" => {
                // trace(name, expr?) — returns focus unchanged; side-effect only
                focus.to_vec()
            }

            // ── FHIR-specific helpers ──────────────────────────────────────
            "extension" => {
                check_arity(name, args, 1)?;
                let url = eval_string_arg(&self.eval(&args[0], ctx)?, "extension url")?;
                let mut out = Vec::new();
                for item in focus {
                    if let Value::Object(fields) = item {
                        if let Some(exts) = fields.get("extension") {
                            for ext in exts {
                                if let Value::Object(ext_fields) = ext {
                                    if let Some(urls) = ext_fields.get("url") {
                                        if urls.iter().any(|u| {
                                            u.as_string()
                                                .map(|s| s.as_ref() == url.as_ref())
                                                .unwrap_or(false)
                                        }) {
                                            out.push(ext.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                out
            }
            "hasValue" => {
                check_arity(name, args, 0)?;
                vec![Value::Bool(
                    focus.len() == 1 && !matches!(focus[0], Value::Null),
                )]
            }
            "getValue" => {
                check_arity(name, args, 0)?;
                focus
                    .iter()
                    .filter(|v| !matches!(v, Value::Null))
                    .cloned()
                    .collect()
            }

            // ── Introspection ─────────────────────────────────────────────
            "type" => {
                check_arity(name, args, 0)?;
                focus
                    .iter()
                    .map(|v| Value::String(Arc::from(v.type_name())))
                    .collect()
            }

            // Unimplemented functions (resolve, memberOf, conformsTo, now, today,
            // timeOfDay, htmlChecks, comparable, lowBoundary, highBoundary,
            // precision) fall through to `UndefinedFunction`: a fake answer
            // would turn an unchecked invariant into a verdict.
            _ => return Ok(None),
        };
        Ok(Some(col))
    }
}
