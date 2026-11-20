use crate::ast::Expr;
use crate::error::EvalError;
use crate::value::{Collection, Value};
use crate::{Decimal, Rounding};

use super::Evaluator;
use super::helpers::{
    as_type, check_arity, check_arity_range, check_collection_len, check_known_content,
    check_type_name, collect_descendants, compare_values, criteria_bool, distinct, distinct_last,
    eval_type_name, is_distinct, is_type, min_max_order, optional_integer, primitive_values,
    singleton, try_sort_by, value_is_type,
};
use super::strict::Focus;
use super::value_set::ValueSet;
use std::cmp::Ordering;

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
                let items = Focus::new(focus);
                for index in 0..focus.len() {
                    let col = self.eval_item(&args[0], &items, index)?;
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
                let items = Focus::new(focus);
                for index in 0..focus.len() {
                    let col = self.eval_item(&args[0], &items, index)?;
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
                let items = Focus::new(focus);
                for (index, item) in focus.iter().enumerate() {
                    let col = self.eval_item(&args[0], &items, index)?;
                    if criteria_bool(&col, name)? == Some(true) {
                        out.push(item.clone());
                    }
                }
                out
            }
            "select" => {
                check_arity(name, args, 1)?;
                let mut out = Vec::new();
                let items = Focus::new(focus);
                for index in 0..focus.len() {
                    out.extend(self.eval_item(&args[0], &items, index)?);
                    check_collection_len(out.len())?;
                }
                out
            }
            "repeat" => {
                check_arity(name, args, 1)?;
                // Only projected items are returned, never the input items themselves.
                let mut seen = ValueSet::default();
                // The input, then each round's new items at the tail: strict navigation
                // types `$this` by all of them, as N1 types it by the input and the
                // projection (`repeat(answer | item)` reaches answers without `answer`).
                let mut items = focus.to_vec();
                let mut start = 0;
                while start < items.len() {
                    let end = items.len();
                    let mut next = Vec::new();
                    // shortcut: N1 does not say what $index is in repeat(); each
                    // round indexes its own items. Revisit if a later release defines it.
                    let round = Focus::new(&items[..end]);
                    for (index, item) in items[start..end].iter().enumerate() {
                        let projected = self.eval_member(&args[0], item, index, &round)?;
                        check_known_content(&projected)?;
                        for v in projected {
                            if seen.insert(v.clone()) {
                                next.push(v);
                            }
                        }
                        check_collection_len(seen.len())?;
                    }
                    items.extend(next);
                    start = end;
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
                let items = Focus::new(focus);
                for index in 0..focus.len() {
                    let step = Evaluator {
                        total: Some(&total),
                        variables: self.variables.clone(),
                        ..*self
                    };
                    total = step.eval_item(&args[0], &items, index)?;
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
                // Keep scanning after an undecided pair: a later type error still surfaces.
                let mut undecided = false;
                for v in &focus[1..] {
                    match min_max_order(v, &min)? {
                        Some(Ordering::Less) => min = v.clone(),
                        Some(_) => {}
                        None => undecided = true,
                    }
                }
                if undecided { vec![] } else { vec![min] }
            }
            "max" => {
                check_arity(name, args, 0)?;
                if focus.is_empty() {
                    return Ok(Some(vec![]));
                }
                let mut max = focus[0].clone();
                // Keep scanning after an undecided pair: a later type error still surfaces.
                let mut undecided = false;
                for v in &focus[1..] {
                    match min_max_order(v, &max)? {
                        Some(Ordering::Greater) => max = v.clone(),
                        Some(_) => {}
                        None => undecided = true,
                    }
                }
                if undecided { vec![] } else { vec![max] }
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
            "sort" => self.sort(focus, args)?,

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

    /// FHIRPath v3 `sort([keySelector [asc | desc], ...])`. No selector
    /// sorts the items themselves; an empty key is lower than any value.
    fn sort(&self, focus: &[Value], args: &[Expr]) -> Result<Collection, EvalError> {
        let selectors: Vec<(Option<&Expr>, bool, bool)> = if args.is_empty() {
            vec![(None, false, false)]
        } else {
            args.iter()
                .map(|arg| match arg {
                    Expr::Descending(key) => (key.as_ref(), true),
                    key => (key, false),
                })
                .map(|(key, descending)| {
                    let (key, negated) = negated_key(key, false);
                    (Some(key), negated, descending)
                })
                .collect()
        };
        let indices = self.sort_by_keys((0..focus.len()).collect(), focus, &selectors)?;
        Ok(indices.into_iter().map(|i| focus[i].clone()).collect())
    }

    /// Sorts `indices` by the first selector, then each run of ties by the
    /// rest, so a later key is only evaluated where earlier keys tied. Only
    /// one level's keys are held at a time: memory stays linear in the input.
    fn sort_by_keys(
        &self,
        indices: Vec<usize>,
        focus: &[Value],
        selectors: &[(Option<&Expr>, bool, bool)],
    ) -> Result<Vec<usize>, EvalError> {
        let Some((&(selector, negated, descending), rest)) = selectors.split_first() else {
            return Ok(indices);
        };
        let mut keyed = Vec::with_capacity(indices.len());
        let items = Focus::new(focus);
        for index in indices {
            let col = match selector {
                Some(expr) => self.eval_item(expr, &items, index)?,
                None => vec![focus[index].clone()],
            };
            keyed.push((index, singleton(&col, "sort() key")?.cloned()));
        }
        let compare = |a: &Option<Value>, b: &Option<Value>| -> Result<Ordering, EvalError> {
            let order = match (a, b) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Less,
                (Some(_), None) => Ordering::Greater,
                (Some(a), Some(b)) if negated => compare_values(b, a)?,
                (Some(a), Some(b)) => compare_values(a, b)?,
            };
            Ok(if descending { order.reverse() } else { order })
        };
        let keyed = try_sort_by(keyed, &mut |(_, a), (_, b)| compare(a, b))?;
        let mut runs: Vec<Vec<usize>> = Vec::new();
        for (position, (index, key)) in keyed.iter().enumerate() {
            let ties_previous = match position.checked_sub(1) {
                Some(previous) => compare(&keyed[previous].1, key)? == Ordering::Equal,
                None => false,
            };
            match runs.last_mut() {
                Some(run) if ties_previous => run.push(*index),
                _ => runs.push(vec![*index]),
            }
        }
        drop(keyed);
        let mut sorted = Vec::new();
        for run in runs {
            if run.len() > 1 {
                sorted.extend(self.sort_by_keys(run, focus, rest)?);
            } else {
                sorted.extend(run);
            }
        }
        Ok(sorted)
    }
}

/// A sort key without its `-` prefixes, and whether they negate it. Official
/// testSort8 and testSort10 negate keys of any orderable type, String
/// included; a negated empty key stays empty, so still lowest (testSort10).
fn negated_key(expr: &Expr, negated: bool) -> (&Expr, bool) {
    match expr {
        Expr::Neg(inner) => negated_key(inner, !negated),
        _ => (expr, negated),
    }
}

fn overflow() -> EvalError {
    EvalError::Type("numeric overflow".into())
}
