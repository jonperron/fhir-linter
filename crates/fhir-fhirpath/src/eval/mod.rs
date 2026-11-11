/// FHIRPath expression evaluator (FHIRPath release 2, as used by FHIR R5).
///
/// The evaluator walks an [`Expr`] AST and produces a [`Collection`].
/// Implemented functions are grouped by category; any other function name
/// yields [`EvalError::UndefinedFunction`].
use std::sync::Arc;

use crate::ast::{Expr, TemporalKind};
use crate::error::EvalError;
use crate::value::{Collection, Value};

mod base64;
mod collection;
mod helpers;
mod math;
mod misc;
mod regex;
mod string_fns;
mod type_conv;

use helpers::{
    bool_col, contains_value, is_type, numeric_op, three_valued_and, three_valued_or,
    value_is_type, values_equal, values_equivalent,
};

/// Evaluate a FHIRPath expression against a context value.
///
/// `context` is the *input collection* (the current `$this` node).
/// Returns a collection of results. `is_choice_variant(base, suffix)` tells
/// whether `base` is a choice element with a type for the capitalised `suffix`,
/// so that `value` finds `valueQuantity`.
pub fn eval(
    expr: &Expr,
    context: &[Value],
    is_choice_variant: &dyn Fn(&str, &str) -> bool,
) -> Result<Collection, EvalError> {
    Evaluator { is_choice_variant }.eval(expr, context)
}

pub(super) struct Evaluator<'a> {
    is_choice_variant: &'a dyn Fn(&str, &str) -> bool,
}

impl Evaluator<'_> {
    pub(super) fn eval(&self, expr: &Expr, ctx: &[Value]) -> Result<Collection, EvalError> {
        match expr {
            // ── Literals ────────────────────────────────────────────────────
            Expr::Null => Ok(vec![]),
            Expr::Bool(b) => Ok(vec![Value::Bool(*b)]),
            Expr::Integer(n) => Ok(vec![Value::Integer(*n)]),
            Expr::Long(n) => Ok(vec![Value::Long(*n)]),
            Expr::Decimal(d) => Ok(vec![Value::Decimal(*d)]),
            Expr::String(s) => Ok(vec![Value::String(Arc::from(s.as_str()))]),
            Expr::Temporal(kind, s) => {
                let v = Arc::from(s.as_str());
                Ok(vec![match kind {
                    TemporalKind::Date => Value::Date(v),
                    TemporalKind::DateTime => Value::DateTime(v),
                    TemporalKind::Time => Value::Time(v),
                }])
            }
            Expr::Quantity(n, u) => Ok(vec![Value::Quantity(*n, Arc::from(u.as_str()))]),

            // ── Context / root ──────────────────────────────────────────────
            Expr::DollarThis => Ok(ctx.to_vec()),
            Expr::DollarIndex => Err(EvalError::UndefinedFunction(
                "$index is not supported outside of iteration context".to_owned(),
            )),

            // ── Identifier / path step ─────────────────────────────────────
            Expr::Ident(name) => self.eval_ident(name, ctx, true),

            // ── Navigation ─────────────────────────────────────────────────
            Expr::Dot(base, step) => {
                let base_col = self.eval(base, ctx)?;
                match &**step {
                    // A type name may only lead the expression, not follow a dot.
                    Expr::Ident(name) => self.eval_ident(name, &base_col, false),
                    _ => self.eval(step, &base_col),
                }
            }

            Expr::Index(base, index_expr) => {
                let base_col = self.eval(base, ctx)?;
                let idx_col = self.eval(index_expr, ctx)?;
                let idx = helpers::single_integer(&idx_col, "index")?;
                // FHIRPath §5.5: an index past the end (or negative) yields empty.
                Ok(usize::try_from(idx)
                    .ok()
                    .and_then(|i| base_col.get(i))
                    .cloned()
                    .into_iter()
                    .collect())
            }

            // ── Function calls ─────────────────────────────────────────────
            Expr::FuncCall(name, args) => self.call_function(name, args, ctx, ctx),
            Expr::Method(base, name, args) => {
                let base_col = self.eval(base, ctx)?;
                self.call_function(name, args, &base_col, ctx)
            }

            // ── Arithmetic ─────────────────────────────────────────────────
            Expr::Add(l, r) => self.eval_arith(l, r, ctx, ArithOp::Add),
            Expr::Sub(l, r) => self.eval_arith(l, r, ctx, ArithOp::Sub),
            Expr::Mul(l, r) => self.eval_arith(l, r, ctx, ArithOp::Mul),
            Expr::Div(l, r) => self.eval_arith(l, r, ctx, ArithOp::Div),
            Expr::DivInt(l, r) => self.eval_arith(l, r, ctx, ArithOp::DivInt),
            Expr::Mod(l, r) => self.eval_arith(l, r, ctx, ArithOp::Mod),
            Expr::Neg(e) => {
                let col = self.eval(e, ctx)?;
                if col.is_empty() {
                    return Ok(vec![]);
                }
                match &col[0] {
                    Value::Integer(n) => n
                        .checked_neg()
                        .map(|r| vec![Value::Integer(r)])
                        .ok_or_else(|| EvalError::Type("integer overflow in negation".into())),
                    Value::Long(n) => n
                        .checked_neg()
                        .map(|r| vec![Value::Long(r)])
                        .ok_or_else(|| EvalError::Type("integer overflow in negation".into())),
                    Value::Decimal(d) => Ok(vec![Value::Decimal(-d)]),
                    v => Err(EvalError::Type(format!("cannot negate {}", v.type_name()))),
                }
            }
            Expr::Concat(l, r) => {
                let ls = self.eval(l, ctx)?;
                let rs = self.eval(r, ctx)?;
                let left = helpers::string_or_empty(&ls);
                let right = helpers::string_or_empty(&rs);
                Ok(vec![Value::String(Arc::from(
                    format!("{left}{right}").as_str(),
                ))])
            }

            // ── Comparisons ─────────────────────────────────────────────────
            Expr::Eq(l, r) => self.eval_cmp(l, r, ctx, CmpOp::Eq),
            Expr::Neq(l, r) => self.eval_cmp(l, r, ctx, CmpOp::Neq),
            Expr::Lt(l, r) => self.eval_cmp(l, r, ctx, CmpOp::Lt),
            Expr::Lte(l, r) => self.eval_cmp(l, r, ctx, CmpOp::Lte),
            Expr::Gt(l, r) => self.eval_cmp(l, r, ctx, CmpOp::Gt),
            Expr::Gte(l, r) => self.eval_cmp(l, r, ctx, CmpOp::Gte),
            Expr::Equiv(l, r) => self.eval_cmp(l, r, ctx, CmpOp::Equiv),
            Expr::NotEquiv(l, r) => self.eval_cmp(l, r, ctx, CmpOp::NotEquiv),

            // ── Boolean operators ─────────────────────────────────────────
            Expr::And(l, r) => {
                let lv = self.eval_bool(l, ctx)?;
                let rv = self.eval_bool(r, ctx)?;
                Ok(bool_col(three_valued_and(lv, rv)))
            }
            Expr::Or(l, r) => {
                let lv = self.eval_bool(l, ctx)?;
                let rv = self.eval_bool(r, ctx)?;
                Ok(bool_col(three_valued_or(lv, rv)))
            }
            Expr::Xor(l, r) => {
                let lv = self.eval_bool(l, ctx)?;
                let rv = self.eval_bool(r, ctx)?;
                let result = match (lv, rv) {
                    (Some(a), Some(b)) => Some(a ^ b),
                    _ => None,
                };
                Ok(bool_col(result))
            }
            Expr::Implies(l, r) => {
                let lv = self.eval_bool(l, ctx)?;
                let rv = self.eval_bool(r, ctx)?;
                let result = match lv {
                    Some(false) => Some(true),
                    Some(true) => rv,
                    None => match rv {
                        Some(true) => Some(true),
                        _ => None,
                    },
                };
                Ok(bool_col(result))
            }

            // ── Type operators ────────────────────────────────────────────
            Expr::Is(base, type_name) => is_type(&self.eval(base, ctx)?, type_name),
            Expr::As(base, type_name) => {
                let col = self.eval(base, ctx)?;
                Ok(col
                    .into_iter()
                    .filter(|v| value_is_type(v, type_name))
                    .collect())
            }

            // ── Set operators ─────────────────────────────────────────────
            Expr::Union(l, r) => {
                let mut ls = self.eval(l, ctx)?;
                let rs = self.eval(r, ctx)?;
                for v in rs {
                    if !contains_value(&ls, &v) {
                        ls.push(v);
                    }
                }
                Ok(ls)
            }
            Expr::In(lhs, rhs) => {
                let lc = self.eval(lhs, ctx)?;
                let rc = self.eval(rhs, ctx)?;
                if lc.is_empty() {
                    return Ok(vec![]);
                }
                let result = lc.iter().all(|v| contains_value(&rc, v));
                Ok(vec![Value::Bool(result)])
            }
            Expr::Contains(lhs, rhs) => {
                let lc = self.eval(lhs, ctx)?;
                let rc = self.eval(rhs, ctx)?;
                if rc.is_empty() {
                    return Ok(vec![]);
                }
                let result = rc.iter().all(|v| contains_value(&lc, v));
                Ok(vec![Value::Bool(result)])
            }
        }
    }

    // ── Path navigation ─────────────────────────────────────────────────────

    fn eval_ident(
        &self,
        name: &str,
        ctx: &[Value],
        allow_type_name: bool,
    ) -> Result<Collection, EvalError> {
        let mut result = Vec::new();
        for item in ctx {
            if let Value::Object(fields) = item {
                if let Some(children) = fields.get(name) {
                    result.extend_from_slice(children);
                } else if let Some(variants) = self.choice_variants(fields, name) {
                    // R5 §2.1.9.1.1: a choice element is named without its `[x]` suffix.
                    result.extend(variants);
                } else if allow_type_name
                    && matches!(
                        fields.get("resourceType").map(Vec::as_slice),
                        Some([Value::String(resource_type)]) if &**resource_type == name
                    )
                {
                    // FHIRPath: an expression may start with the context's type name.
                    result.push(item.clone());
                }
            }
        }
        Ok(result)
    }

    /// Values of the choice variants of `name` (`valueQuantity` for `value`), or `None`.
    fn choice_variants(
        &self,
        fields: &indexmap::IndexMap<Arc<str>, Collection>,
        name: &str,
    ) -> Option<Collection> {
        let variants: Collection = fields
            .iter()
            .filter(|(key, _)| {
                key.strip_prefix(name).is_some_and(|suffix| {
                    suffix.starts_with(|c: char| c.is_ascii_uppercase())
                        && (self.is_choice_variant)(name, suffix)
                })
            })
            .flat_map(|(_, values)| values.iter().cloned())
            .collect();
        (!variants.is_empty()).then_some(variants)
    }

    // ── Function dispatch ───────────────────────────────────────────────────

    fn call_function(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Result<Collection, EvalError> {
        if let Some(r) = self.call_collection_function(name, args, focus, ctx) {
            return r;
        }
        if let Some(r) = self.call_string_function(name, args, focus, ctx) {
            return r;
        }
        if let Some(r) = self.call_math_function(name, args, focus, ctx) {
            return r;
        }
        if let Some(r) = self.call_type_function(name, args, focus) {
            return r;
        }
        if let Some(r) = self.call_misc_function(name, args, focus, ctx) {
            return r;
        }
        Err(EvalError::UndefinedFunction(name.to_owned()))
    }

    // ── Arithmetic helpers ───────────────────────────────────────────────────

    fn eval_arith(
        &self,
        l: &Expr,
        r: &Expr,
        ctx: &[Value],
        op: ArithOp,
    ) -> Result<Collection, EvalError> {
        let lc = self.eval(l, ctx)?;
        let rc = self.eval(r, ctx)?;
        if lc.is_empty() || rc.is_empty() {
            return Ok(vec![]);
        }
        let lv = &lc[0];
        let rv = &rc[0];
        match op {
            ArithOp::Add | ArithOp::Sub | ArithOp::Mul | ArithOp::Div | ArithOp::Mod => {
                numeric_op(lv, rv, op)
            }
            ArithOp::DivInt => {
                let a = lv
                    .as_decimal()
                    .ok_or_else(|| EvalError::Type("div requires numbers".into()))?;
                let b = rv
                    .as_decimal()
                    .ok_or_else(|| EvalError::Type("div requires numbers".into()))?;
                if b == 0.0 {
                    return Err(EvalError::DivisionByZero);
                }
                Ok(vec![Value::Integer((a / b).trunc() as i64)])
            }
        }
    }

    // ── Comparison helpers ───────────────────────────────────────────────────

    fn eval_cmp(
        &self,
        l: &Expr,
        r: &Expr,
        ctx: &[Value],
        op: CmpOp,
    ) -> Result<Collection, EvalError> {
        let lc = self.eval(l, ctx)?;
        let rc = self.eval(r, ctx)?;
        if lc.is_empty() || rc.is_empty() {
            return Ok(vec![]);
        }
        let result = match op {
            CmpOp::Eq => values_equal(&lc[0], &rc[0]),
            CmpOp::Neq => values_equal(&lc[0], &rc[0]).map(|b| !b),
            CmpOp::Equiv => Some(values_equivalent(&lc[0], &rc[0])),
            CmpOp::NotEquiv => Some(!values_equivalent(&lc[0], &rc[0])),
            _ => {
                let ord = helpers::compare_values(&lc[0], &rc[0])?;
                let b = match op {
                    CmpOp::Lt => ord == std::cmp::Ordering::Less,
                    CmpOp::Lte => ord != std::cmp::Ordering::Greater,
                    CmpOp::Gt => ord == std::cmp::Ordering::Greater,
                    CmpOp::Gte => ord != std::cmp::Ordering::Less,
                    _ => unreachable!(),
                };
                Some(b)
            }
        };
        Ok(bool_col(result))
    }

    /// Evaluate an expression as a three-valued boolean (`None` = empty collection).
    fn eval_bool(&self, expr: &Expr, ctx: &[Value]) -> Result<Option<bool>, EvalError> {
        let col = self.eval(expr, ctx)?;
        Ok(helpers::collection_to_bool(&col))
    }
}

// ── Helper types ─────────────────────────────────────────────────────────────

pub(super) enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
    DivInt,
    Mod,
}

pub(super) enum CmpOp {
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    Equiv,
    NotEquiv,
}
