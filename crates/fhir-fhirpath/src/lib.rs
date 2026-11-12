#![forbid(unsafe_code)]
//! FHIRPath evaluator for `constraint.expression` invariants in FHIR R5
//! StructureDefinitions.
//!
//! FHIR R5 uses FHIRPath release 2 (2.0.0). This crate implements a subset
//! of it, plus a few later additions; functions it does not implement
//! (for example `resolve()` or `memberOf()`) return
//! [`EvalError::UndefinedFunction`] rather than a guessed result.

mod ast;
mod error;
mod eval;
mod parser;
mod value;

pub use ast::Expr;
pub use error::{EvalError, ParseError};
pub use value::{
    ChildElement, Collection, Value, from_parser_value, node_to_typed_value,
    resource_to_typed_value, resource_to_value,
};

/// Parse and evaluate a FHIRPath expression against a context collection.
///
/// # Errors
///
/// Returns [`ParseError`] if the expression is syntactically invalid, or
/// [`EvalError`] if evaluation fails (type errors, undefined functions, etc.).
pub fn evaluate(expression: &str, context: &[Value]) -> Result<Collection, EvalError> {
    let expr = parser::parse(expression)?;
    evaluate_parsed(&expr, context)
}

/// Evaluate an already parsed expression (see [`parse`]) against a context collection.
///
/// Without type knowledge, choice elements (`value` for `valueQuantity`) are not
/// navigable; use [`evaluate_parsed_with_choices`] for FHIR content.
///
/// # Errors
///
/// Returns [`EvalError`] if evaluation fails.
pub fn evaluate_parsed(expr: &Expr, context: &[Value]) -> Result<Collection, EvalError> {
    eval::eval(expr, context, &|_, _| false)
}

/// Evaluate a parsed expression, navigating choice elements: when `value` is
/// absent, the fields `value` + suffix are returned for every capitalised
/// suffix for which `is_choice_variant("value", suffix)` is true (for example
/// `Quantity` or `DateTime`).
///
/// # Errors
///
/// Returns [`EvalError`] if evaluation fails.
pub fn evaluate_parsed_with_choices(
    expr: &Expr,
    context: &[Value],
    is_choice_variant: &dyn Fn(&str, &str) -> bool,
) -> Result<Collection, EvalError> {
    eval::eval(expr, context, is_choice_variant)
}

/// Parse a FHIRPath expression into an AST without evaluating it.
pub fn parse(expression: &str) -> Result<Expr, ParseError> {
    parser::parse(expression)
}
