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
}
