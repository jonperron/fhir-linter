/// Errors produced by the FHIRPath engine.
#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum ParseError {
    #[error("syntax error: {0}")]
    Syntax(String),
}

/// Errors produced during evaluation.
#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum EvalError {
    #[error("type error: {0}")]
    Type(String),
    #[error("undefined function: {0}")]
    UndefinedFunction(String),
    #[error("arity error: function '{name}' expects {expected} argument(s), got {got}")]
    Arity {
        name: String,
        expected: usize,
        got: usize,
    },
    /// No longer produced: division by zero gives empty (N1 §6.6). Kept so
    /// that matches on the public enum keep compiling.
    #[error("division by zero")]
    DivisionByZero,
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),
    /// An intermediate collection would exceed the evaluator's size limit,
    /// which the field holds (not the collection's actual size).
    #[error("collection larger than {0} items")]
    CollectionTooLarge(usize),
    /// A String result would exceed the evaluator's length limit, in
    /// characters, which the field holds (not the string's actual length).
    #[error("string longer than {0} characters")]
    StringTooLong(usize),
    /// One evaluation built more characters of String values, in total, than
    /// the evaluator's budget, which the field holds.
    #[error("evaluation built more than {0} characters of strings")]
    StringBudgetExceeded(usize),
}
