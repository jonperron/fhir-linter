use std::sync::Arc;

use crate::ast::Expr;
use crate::error::EvalError;
use crate::value::{Collection, Value};

use super::Evaluator;
use super::helpers::{
    check_arity, check_arity_range, check_collection_len, criteria_bool, eval_string_arg,
    singleton, singleton_bool,
};

impl Evaluator<'_> {
    /// Handle miscellaneous functions (logic, system, FHIR extension helpers).
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
                singleton_bool(focus, "not()")?
                    .map(|b| Value::Bool(!b))
                    .into_iter()
                    .collect()
            }
            "iif" => {
                check_arity_range(name, args, 2, 3)?;
                // N1 §5.5.1: iif() on several items is an error.
                singleton(focus, "iif()")?;
                let cond = self.eval(&args[0], focus)?;
                let result = if criteria_bool(&cond, name)? == Some(true) {
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
                    if let Some(fields) = item.fields() {
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
                    // Per item, so many copies of one node cannot expand past the cap.
                    check_collection_len(out.len())?;
                }
                out
            }
            "hasValue" => {
                check_arity(name, args, 0)?;
                // R5: true for one primitive that has a value. A complex value
                // gives empty ("otherwise, the return value is empty"); no item,
                // several items or a primitive without a value give false, as the
                // R5 constraints (exp-2, per-1) and the official suite expect.
                match focus {
                    [item] if matches!(item.primitive_value(), Value::Object(_)) => vec![],
                    [item] => vec![Value::Bool(!matches!(item.primitive_value(), Value::Null))],
                    _ => vec![Value::Bool(false)],
                }
            }
            "getValue" => {
                check_arity(name, args, 0)?;
                focus
                    .iter()
                    .map(Value::primitive_value)
                    .filter(|v| !matches!(v, Value::Null | Value::Object(_)))
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
