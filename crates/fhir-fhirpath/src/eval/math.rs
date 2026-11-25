use crate::ast::Expr;
use crate::error::EvalError;
use crate::value::{Collection, Value};
use crate::{Decimal, Rounding};

use super::Evaluator;
use super::helpers::{check_arity, single_integer, singleton};

impl Evaluator<'_> {
    /// Handle mathematical functions.
    /// Returns `None` if the function name is not in this category.
    pub(super) fn call_math_function(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Option<Result<Collection, EvalError>> {
        self.try_math(name, args, focus, ctx).transpose()
    }

    fn try_math(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Result<Option<Collection>, EvalError> {
        let col: Collection = match name {
            "abs" => {
                check_arity(name, args, 0)?;
                match singleton(focus, name)? {
                    Some(Value::Integer(n)) => {
                        vec![Value::Integer(n.checked_abs().ok_or_else(|| {
                            EvalError::Type("integer overflow in abs()".into())
                        })?)]
                    }
                    Some(Value::Long(n)) => {
                        vec![Value::Long(n.checked_abs().ok_or_else(|| {
                            EvalError::Type("integer overflow in abs()".into())
                        })?)]
                    }
                    Some(Value::Decimal(d)) => {
                        vec![Value::Decimal(d.checked_abs().ok_or_else(overflow)?)]
                    }
                    // N1 §5.7.1: the absolute value of a quantity keeps its unit.
                    Some(v) => match super::helpers::object_quantity(v) {
                        Some(Value::Quantity(d, unit)) => {
                            vec![Value::Quantity(d.checked_abs().ok_or_else(overflow)?, unit)]
                        }
                        _ => {
                            return Err(EvalError::Type(format!(
                                "abs() requires number, got {}",
                                v.type_name()
                            )));
                        }
                    },
                    None => vec![],
                }
            }
            "ceiling" => {
                check_arity(name, args, 0)?;
                match singleton(focus, name)? {
                    Some(Value::Integer(n)) => vec![Value::Integer(*n)],
                    Some(Value::Decimal(d)) => {
                        vec![Value::Integer(
                            d.to_i64(Rounding::Ceiling).ok_or_else(overflow)?,
                        )]
                    }
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "ceiling() requires number, got {}",
                            v.type_name()
                        )));
                    }
                    None => vec![],
                }
            }
            "floor" => {
                check_arity(name, args, 0)?;
                match singleton(focus, name)? {
                    Some(Value::Integer(n)) => vec![Value::Integer(*n)],
                    Some(Value::Decimal(d)) => {
                        vec![Value::Integer(
                            d.to_i64(Rounding::Floor).ok_or_else(overflow)?,
                        )]
                    }
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "floor() requires number, got {}",
                            v.type_name()
                        )));
                    }
                    None => vec![],
                }
            }
            "round" => {
                if args.len() > 1 {
                    return Err(EvalError::Arity {
                        name: name.to_owned(),
                        expected: 1,
                        got: args.len(),
                    });
                }
                let precision = if args.is_empty() {
                    0i64
                } else {
                    single_integer(&self.eval(&args[0], ctx)?, "round precision")?
                };
                match singleton(focus, name)? {
                    Some(Value::Integer(n)) => vec![Value::Integer(*n)],
                    Some(Value::Decimal(d)) => {
                        let places = i32::try_from(precision).map_err(|_| overflow())?;
                        let rounded = if places >= d.scale() {
                            Some(*d)
                        } else {
                            d.rescale(places, Rounding::HalfAwayFromZero)
                        };
                        vec![Value::Decimal(rounded.ok_or_else(overflow)?)]
                    }
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "round() requires number, got {}",
                            v.type_name()
                        )));
                    }
                    None => vec![],
                }
            }
            "sqrt" => {
                check_arity(name, args, 0)?;
                match singleton(focus, name)? {
                    Some(Value::Integer(n)) => representable((*n as f64).sqrt()),
                    Some(Value::Decimal(d)) => representable(d.to_f64().sqrt()),
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "sqrt() requires number, got {}",
                            v.type_name()
                        )));
                    }
                    None => vec![],
                }
            }
            "exp" => {
                check_arity(name, args, 0)?;
                match singleton(focus, name)? {
                    Some(Value::Integer(n)) => representable((*n as f64).exp()),
                    Some(Value::Decimal(d)) => representable(d.to_f64().exp()),
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "exp() requires number, got {}",
                            v.type_name()
                        )));
                    }
                    None => vec![],
                }
            }
            "ln" => {
                check_arity(name, args, 0)?;
                match singleton(focus, name)? {
                    Some(Value::Integer(n)) => representable((*n as f64).ln()),
                    Some(Value::Decimal(d)) => representable(d.to_f64().ln()),
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "ln() requires number, got {}",
                            v.type_name()
                        )));
                    }
                    None => vec![],
                }
            }
            "log" => {
                check_arity(name, args, 1)?;
                // Several focus items are an error even when the base is empty.
                let item = singleton(focus, name)?;
                let base = match singleton(&self.eval(&args[0], ctx)?, name)? {
                    Some(v) => v
                        .as_decimal()
                        .ok_or_else(|| EvalError::Type("log() base must be a number".into()))?
                        .to_f64(),
                    None => return Ok(Some(vec![])),
                };
                match item {
                    Some(Value::Integer(n)) => representable((*n as f64).log(base)),
                    Some(Value::Decimal(d)) => representable(d.to_f64().log(base)),
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "log() requires number, got {}",
                            v.type_name()
                        )));
                    }
                    None => vec![],
                }
            }
            "power" => {
                check_arity(name, args, 1)?;
                // Several focus items are an error even when the exponent is empty.
                let item = singleton(focus, name)?;
                let exponent = self.eval(&args[0], ctx)?;
                let Some(exponent) = singleton(&exponent, name)? else {
                    return Ok(Some(vec![]));
                };
                let exp = exponent
                    .as_decimal()
                    .ok_or_else(|| EvalError::Type("power() exponent must be a number".into()))?
                    .to_f64();
                match item {
                    // N1 §5.7.8: used with Integers, the result is an Integer.
                    Some(Value::Integer(n)) => match exponent {
                        Value::Integer(e) if *e >= 0 => {
                            // Past u32::MAX only 0, 1 and -1 fit, so keep the parity.
                            let e = u32::try_from(*e).unwrap_or(u32::MAX - 1 + (*e % 2) as u32);
                            vec![Value::Integer(n.checked_pow(e).ok_or_else(|| {
                                EvalError::Type("integer overflow in power()".into())
                            })?)]
                        }
                        _ => representable((*n as f64).powf(exp)),
                    },
                    Some(Value::Decimal(d)) => representable(d.to_f64().powf(exp)),
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "power() requires number, got {}",
                            v.type_name()
                        )));
                    }
                    None => vec![],
                }
            }
            "truncate" => {
                check_arity(name, args, 0)?;
                match singleton(focus, name)? {
                    Some(Value::Integer(n)) => vec![Value::Integer(*n)],
                    Some(Value::Decimal(d)) => {
                        vec![Value::Integer(
                            d.to_i64(Rounding::TowardZero).ok_or_else(overflow)?,
                        )]
                    }
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "truncate() requires number, got {}",
                            v.type_name()
                        )));
                    }
                    None => vec![],
                }
            }

            _ => return Ok(None),
        };
        Ok(Some(col))
    }
}

/// N2 sqrt() and power(), and exp(), ln() and log(): a result that cannot be
/// represented (NaN or infinity) is empty.
/// shortcut: computed in f64, then kept as its shortest decimal text; exact
/// only for the arithmetic operators.
fn representable(result: f64) -> Collection {
    Decimal::from_f64(result)
        .map(Value::Decimal)
        .into_iter()
        .collect()
}

fn overflow() -> EvalError {
    EvalError::Type("decimal overflow".into())
}
