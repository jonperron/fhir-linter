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
mod temporal;
mod type_conv;

use helpers::{
    bool_col, collections_equal, collections_equivalent, contains_value, is_type, numeric_op,
    singleton, three_valued_and, three_valued_implies, three_valued_or, value_is_type,
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
                let Some(v) = singleton(&col, "negation")? else {
                    return Ok(vec![]);
                };
                match v {
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
                singleton(&ls, "&")?;
                singleton(&rs, "&")?;
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
            Expr::And(l, r) => self.eval_logical(l, r, ctx, [false, false], three_valued_and),
            Expr::Or(l, r) => self.eval_logical(l, r, ctx, [true, true], three_valued_or),
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
                self.eval_logical(l, r, ctx, [false, true], three_valued_implies)
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
                let ls = self.eval(l, ctx)?;
                let rs = self.eval(r, ctx)?;
                Ok(helpers::distinct(ls.into_iter().chain(rs)))
            }
            Expr::In(lhs, rhs) => {
                let lc = self.eval(lhs, ctx)?;
                let rc = self.eval(rhs, ctx)?;
                Ok(singleton(&lc, "in")?
                    .map(|v| Value::Bool(contains_value(&rc, v)))
                    .into_iter()
                    .collect())
            }
            Expr::Contains(lhs, rhs) => {
                let lc = self.eval(lhs, ctx)?;
                let rc = self.eval(rhs, ctx)?;
                Ok(singleton(&rc, "contains")?
                    .map(|v| Value::Bool(contains_value(&lc, v)))
                    .into_iter()
                    .collect())
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
        let (Some(lv), Some(rv)) = (singleton(&lc, "arithmetic")?, singleton(&rc, "arithmetic")?)
        else {
            return Ok(vec![]);
        };
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
                    return Ok(vec![]);
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
        // N1 §6.1.3: `~` is never empty, even on empty operands.
        match op {
            CmpOp::Equiv => return Ok(bool_col(Some(collections_equivalent(&lc, &rc)))),
            CmpOp::NotEquiv => return Ok(bool_col(Some(!collections_equivalent(&lc, &rc)))),
            _ => {}
        }
        if lc.is_empty() || rc.is_empty() {
            return Ok(vec![]);
        }
        let result = match op {
            CmpOp::Eq => collections_equal(&lc, &rc),
            CmpOp::Neq => collections_equal(&lc, &rc).map(|b| !b),
            _ => {
                let (Some(a), Some(b)) =
                    (singleton(&lc, "comparison")?, singleton(&rc, "comparison")?)
                else {
                    return Ok(vec![]);
                };
                let Some(ord) = helpers::operator_order(a, b)? else {
                    return Ok(vec![]);
                };
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

    /// `and`, `or` and `implies`: when one side has the value in `deciding`
    /// (left, right), the truth table fixes the result, so an error on the
    /// other side is ignored (N1 §6.5). The right side is skipped when the
    /// left decides.
    fn eval_logical(
        &self,
        l: &Expr,
        r: &Expr,
        ctx: &[Value],
        deciding: [bool; 2],
        combine: fn(Option<bool>, Option<bool>) -> Option<bool>,
    ) -> Result<Collection, EvalError> {
        let left = self.eval_bool(l, ctx);
        if let Ok(Some(b)) = left
            && b == deciding[0]
        {
            return Ok(bool_col(combine(Some(b), None)));
        }
        let right = self.eval_bool(r, ctx);
        if let Ok(Some(b)) = right
            && b == deciding[1]
        {
            return Ok(bool_col(combine(left.unwrap_or(None), Some(b))));
        }
        Ok(bool_col(combine(left?, right?)))
    }

    /// Evaluate an expression as a three-valued boolean (`None` = empty collection).
    fn eval_bool(&self, expr: &Expr, ctx: &[Value]) -> Result<Option<bool>, EvalError> {
        helpers::singleton_bool(&self.eval(expr, ctx)?, "boolean operator")
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
