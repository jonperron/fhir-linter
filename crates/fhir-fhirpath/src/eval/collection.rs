use crate::ast::Expr;
use crate::error::EvalError;
use crate::value::{Collection, Value};
use crate::{Decimal, Rounding};

use super::Evaluator;
use super::helpers::{
    as_type, check_arity, check_arity_range, check_collection_len, check_known_content,
    check_type_name, collect_descendants, compare_values, criteria_bool, distinct, distinct_last,
    eval_type_name, is_distinct, is_type, optional_integer, primitive_values, value_is_type,
};
use super::value_set::ValueSet;

impl Evaluator<'_> {
    /// Handle collection, existence, filtering, aggregation, and navigation functions.
    /// Returns `None` if the function name is not in this category.
    pub(super) fn call_collection_function(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Option<Result<Collection, EvalError>> {
        self.try_collection(name, args, focus, ctx).transpose()
    }

    fn try_collection(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Result<Option<Collection>, EvalError> {
        // Aggregates read values: a primitive without one is not an input item.
        let values;
        let focus = match name {
            "allTrue" | "anyTrue" | "allFalse" | "anyFalse" | "sum" | "min" | "max" | "avg" => {
                values = primitive_values(focus);
                &*values
            }
            _ => focus,
        };
        let col: Collection = match name {
            // ── Existence ────────────────────────────────────────────────
            "empty" => {
                check_arity(name, args, 0)?;
                vec![Value::Bool(focus.is_empty())]
            }
            "exists" => {
                if args.is_empty() {
                    return Ok(Some(vec![Value::Bool(!focus.is_empty())]));
                }
                check_arity(name, args, 1)?;
                let mut result = false;
                for (index, item) in focus.iter().enumerate() {
                    let col = self.eval_item(&args[0], item, index)?;
                    if criteria_bool(&col, name)? == Some(true) {
                        result = true;
                        break;
                    }
                }
                vec![Value::Bool(result)]
            }
            "all" => {
                check_arity(name, args, 1)?;
                if focus.is_empty() {
                    return Ok(Some(vec![Value::Bool(true)]));
                }
                let mut result = true;
                for (index, item) in focus.iter().enumerate() {
                    let col = self.eval_item(&args[0], item, index)?;
                    if criteria_bool(&col, name)? != Some(true) {
                        result = false;
                        break;
                    }
                }
                vec![Value::Bool(result)]
            }
            "allTrue" | "anyTrue" | "allFalse" | "anyFalse" => {
                check_arity(name, args, 0)?;
                // N1 §5.1: the input must be a collection of Booleans.
                let bools = focus
                    .iter()
                    .map(|v| match v.primitive_value() {
                        Value::Bool(b) => Ok(*b),
                        v => Err(EvalError::Type(format!(
                            "{name}() requires Boolean items, got {}",
                            v.type_name()
                        ))),
                    })
                    .collect::<Result<Vec<bool>, _>>()?;
                let result = match name {
                    "allTrue" => bools.iter().all(|b| *b),
                    "anyTrue" => bools.iter().any(|b| *b),
                    "allFalse" => bools.iter().all(|b| !*b),
                    _ => bools.iter().any(|b| !*b),
                };
                vec![Value::Bool(result)]
            }

            // ── Filtering / subsetting ────────────────────────────────────
            "where" => {
                check_arity(name, args, 1)?;
                let mut out = Vec::new();
                for (index, item) in focus.iter().enumerate() {
                    let col = self.eval_item(&args[0], item, index)?;
                    if criteria_bool(&col, name)? == Some(true) {
                        out.push(item.clone());
                    }
                }
                out
            }
            "select" => {
                check_arity(name, args, 1)?;
                let mut out = Vec::new();
                for (index, item) in focus.iter().enumerate() {
                    out.extend(self.eval_item(&args[0], item, index)?);
                    check_collection_len(out.len())?;
                }
                out
            }
            "repeat" => {
                check_arity(name, args, 1)?;
                // Only projected items are returned, never the input items themselves.
                let mut seen = ValueSet::default();
                let mut frontier = focus.to_vec();
                loop {
                    let mut next = Vec::new();
                    // shortcut: N1 does not say what $index is in repeat(); each
                    // round indexes its own items. Revisit if a later release defines it.
                    for (index, item) in frontier.iter().enumerate() {
                        let projected = self.eval_item(&args[0], item, index)?;
                        check_known_content(&projected)?;
                        for v in projected {
                            if seen.insert(v.clone()) {
                                next.push(v);
                            }
                        }
                        check_collection_len(seen.len())?;
                    }
                    if next.is_empty() {
                        break;
                    }
                    frontier = next;
                }
                seen.into_values()
            }
            "ofType" => {
                check_arity(name, args, 1)?;
                let type_name = eval_type_name(&args[0])?;
                check_type_name(&type_name, self.environment.is_subtype)?;
                focus
                    .iter()
                    .filter_map(|v| {
                        match value_is_type(v, &type_name, self.environment.is_subtype) {
                            Ok(true) => Some(Ok(v.clone())),
                            Ok(false) => None,
                            Err(e) => Some(Err(e)),
                        }
                    })
                    .collect::<Result<_, _>>()?
            }
            "is" => {
                check_arity(name, args, 1)?;
                is_type(
                    focus,
                    &eval_type_name(&args[0])?,
                    self.environment.is_subtype,
                )?
            }
            "as" => {
                check_arity(name, args, 1)?;
                as_type(
                    focus,
                    &eval_type_name(&args[0])?,
                    self.environment.is_subtype,
                )?
            }

            // ── Counting / indexing ───────────────────────────────────────
            "count" => {
                check_arity(name, args, 0)?;
                vec![Value::Integer(focus.len() as i64)]
            }
            "first" => {
                check_arity(name, args, 0)?;
                focus.first().cloned().into_iter().collect()
            }
            "last" => {
                check_arity(name, args, 0)?;
                focus.last().cloned().into_iter().collect()
            }
            "tail" => {
                check_arity(name, args, 0)?;
                focus.iter().skip(1).cloned().collect()
            }
            "skip" | "take" => {
                check_arity(name, args, 1)?;
                // N1 §4.4.1: an empty count gives empty.
                let Some(n) = optional_integer(&self.eval(&args[0], ctx)?, name)? else {
                    return Ok(Some(vec![]));
                };
                // Clamp: on 32-bit targets an i64 count can exceed usize.
                let n = usize::try_from(n.max(0)).unwrap_or(usize::MAX);
                if name == "skip" {
                    focus.iter().skip(n).cloned().collect()
                } else {
                    focus.iter().take(n).cloned().collect()
                }
            }
            "single" => {
                check_arity(name, args, 0)?;
                if focus.len() > 1 {
                    return Err(EvalError::Type(format!(
                        "single() requires at most 1 item, got {}",
                        focus.len()
                    )));
                }
                focus.to_vec()
            }

            // ── Set operations ────────────────────────────────────────────
            "distinct" => {
                check_arity(name, args, 0)?;
                check_known_content(focus)?;
                // None: uncomparable date/times, so the result is empty (N1 `=`).
                distinct_last(focus).unwrap_or_default()
            }
            "isDistinct" => {
                check_arity(name, args, 0)?;
                check_known_content(focus)?;
                is_distinct(focus).map(Value::Bool).into_iter().collect()
            }
            "subsetOf" => {
                check_arity(name, args, 1)?;
                let other = self.eval(&args[0], ctx)?;
                check_known_content(focus)?;
                check_known_content(&other)?;
                let other: ValueSet = other.into_iter().collect();
                vec![Value::Bool(focus.iter().all(|v| other.contains(v)))]
            }
            "supersetOf" => {
                check_arity(name, args, 1)?;
                let other = self.eval(&args[0], ctx)?;
                check_known_content(focus)?;
                check_known_content(&other)?;
                let focus: ValueSet = focus.iter().cloned().collect();
                vec![Value::Bool(other.iter().all(|v| focus.contains(v)))]
            }
            "intersect" => {
                check_arity(name, args, 1)?;
                let other = self.eval(&args[0], ctx)?;
                check_known_content(focus)?;
                check_known_content(&other)?;
                let other: ValueSet = other.into_iter().collect();
                distinct(focus.iter().filter(|v| other.contains(v)).cloned())
            }
            "exclude" => {
                check_arity(name, args, 1)?;
                let other = self.eval(&args[0], ctx)?;
                check_known_content(focus)?;
                check_known_content(&other)?;
                let other: ValueSet = other.into_iter().collect();
                focus
                    .iter()
                    .filter(|v| !other.contains(v))
                    .cloned()
                    .collect()
            }
            // N2 §6.5.1: `union()` is the `|` operator.
            "union" => {
                check_arity(name, args, 1)?;
                let other = self.eval(&args[0], ctx)?;
                check_known_content(focus)?;
                check_known_content(&other)?;
                distinct(focus.iter().cloned().chain(other))
            }
            "combine" => {
                check_arity(name, args, 1)?;
                let other = self.eval(&args[0], ctx)?;
                let mut result = focus.to_vec();
                result.extend(other);
                result
            }
            "coalesce" => {
                for arg in args {
                    let result = self.eval(arg, focus)?;
                    if !result.is_empty() {
                        return Ok(Some(result));
                    }
                }
                vec![]
            }

            // ── Aggregation ───────────────────────────────────────────────
            "aggregate" => {
                check_arity_range(name, args, 1, 2)?;
                let mut total = if args.len() == 2 {
                    self.eval(&args[1], ctx)?
                } else {
                    vec![]
                };
                for (index, item) in focus.iter().enumerate() {
                    let step = Evaluator {
                        total: Some(&total),
                        variables: self.variables.clone(),
                        ..*self
                    };
                    total = step.eval_item(&args[0], item, index)?;
                }
                total
            }
            "sum" => {
                check_arity(name, args, 0)?;
                if focus.is_empty() {
                    return Ok(Some(vec![Value::Integer(0)]));
                }
                let mut sum = Decimal::from(0);
                let mut all_integer = true;
                for v in focus {
                    let item = match v.primitive_value() {
                        Value::Integer(n) => Decimal::from(*n),
                        Value::Decimal(d) => {
                            all_integer = false;
                            *d
                        }
                        other => {
                            return Err(EvalError::Type(format!(
                                "sum() requires numbers, got {}",
                                other.type_name()
                            )));
                        }
                    };
                    sum = sum.checked_add(item).ok_or_else(overflow)?;
                }
                if all_integer {
                    let total = sum.to_i64(Rounding::TowardZero).ok_or_else(overflow)?;
                    vec![Value::Integer(total)]
                } else {
                    vec![Value::Decimal(sum)]
                }
            }
            "min" => {
                check_arity(name, args, 0)?;
                if focus.is_empty() {
                    return Ok(Some(vec![]));
                }
                let mut min = focus[0].clone();
                for v in &focus[1..] {
                    if compare_values(v, &min)? == std::cmp::Ordering::Less {
                        min = v.clone();
                    }
                }
                vec![min]
            }
            "max" => {
                check_arity(name, args, 0)?;
                if focus.is_empty() {
                    return Ok(Some(vec![]));
                }
                let mut max = focus[0].clone();
                for v in &focus[1..] {
                    if compare_values(v, &max)? == std::cmp::Ordering::Greater {
                        max = v.clone();
                    }
                }
                vec![max]
            }
            "avg" => {
                check_arity(name, args, 0)?;
                if focus.is_empty() {
                    return Ok(Some(vec![]));
                }
                let count = Decimal::from(i64::try_from(focus.len()).map_err(|_| overflow())?);
                let mut sum = Decimal::from(0);
                for v in focus {
                    let item = match v.primitive_value() {
                        Value::Integer(n) | Value::Long(n) => Decimal::from(*n),
                        Value::Decimal(d) => *d,
                        other => {
                            return Err(EvalError::Type(format!(
                                "avg() requires numbers, got {}",
                                other.type_name()
                            )));
                        }
                    };
                    sum = sum.checked_add(item).ok_or_else(overflow)?;
                }
                vec![Value::Decimal(sum.checked_div(count).ok_or_else(overflow)?)]
            }
            "sort" => {
                check_arity_range(name, args, 0, 1)?;
                let mut items = focus.to_vec();
                if args.is_empty() {
                    let mut sort_error: Option<EvalError> = None;
                    items.sort_by(|a, b| {
                        compare_values(a, b).unwrap_or_else(|e| {
                            sort_error = Some(e);
                            std::cmp::Ordering::Equal
                        })
                    });
                    if let Some(e) = sort_error {
                        return Err(e);
                    }
                    items
                } else {
                    let mut keys: Vec<Value> = Vec::with_capacity(items.len());
                    for (index, item) in items.iter().enumerate() {
                        let key_col = self.eval_item(&args[0], item, index)?;
                        keys.push(key_col.into_iter().next().unwrap_or(Value::Null));
                    }
                    let mut indices: Vec<usize> = (0..items.len()).collect();
                    let mut sort_error: Option<EvalError> = None;
                    indices.sort_by(|&i, &j| {
                        compare_values(&keys[i], &keys[j]).unwrap_or_else(|e| {
                            sort_error = Some(e);
                            std::cmp::Ordering::Equal
                        })
                    });
                    if let Some(e) = sort_error {
                        return Err(e);
                    }
                    indices.into_iter().map(|i| items[i].clone()).collect()
                }
            }

            // ── Navigation ────────────────────────────────────────────────
            // Checked per item: many copies of one node would otherwise expand
            // past the cap before the result is checked.
            "children" => {
                check_arity(name, args, 0)?;
                check_known_content(focus)?;
                let mut out = Vec::new();
                for item in focus {
                    out.extend(item.child_elements().cloned());
                    check_collection_len(out.len())?;
                }
                out
            }
            "descendants" => {
                check_arity(name, args, 0)?;
                check_known_content(focus)?;
                let mut out = Vec::new();
                for item in focus {
                    collect_descendants(item, &mut out);
                    check_collection_len(out.len())?;
                }
                out
            }

            _ => return Ok(None),
        };
        Ok(Some(col))
    }
}

fn overflow() -> EvalError {
    EvalError::Type("numeric overflow".into())
}
