//! FHIRPath expression parser.
//!
//! Converts raw text into an [`Expr`] AST via the Pest grammar.
use pest::Parser as _;
use pest::iterators::Pair;
use pest_derive::Parser;

use crate::ast::{Expr, TemporalKind};
use crate::error::ParseError;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct FhirPathParser;

/// Parse a FHIRPath expression string into an [`Expr`] AST.
pub fn parse(input: &str) -> Result<Expr, ParseError> {
    check_limits(input)?;
    let pairs = FhirPathParser::parse(Rule::expression, input)
        .map_err(|e| ParseError::Syntax(e.to_string()))?;

    let top = pairs
        .into_iter()
        .next()
        .ok_or_else(|| ParseError::Syntax("empty input".into()))?;

    build_expr(
        top.into_inner()
            .next()
            .ok_or_else(|| ParseError::Syntax("empty expression".into()))?,
    )
}

/// Maximum accepted expression length in bytes.
// Real FHIR constraints are < 400 bytes; 1 KiB keeps chains within a 2 MB debug stack.
const MAX_INPUT_LEN: usize = 1024;
/// Maximum nesting of `(` and `[`; deeper input would overflow the stack.
const MAX_NESTING: usize = 16;

/// Reject hostile input before pest/AST recursion can overflow the stack.
fn check_limits(input: &str) -> Result<(), ParseError> {
    if input.len() > MAX_INPUT_LEN {
        return Err(ParseError::Syntax(format!(
            "expression longer than {MAX_INPUT_LEN} bytes"
        )));
    }
    let mut depth = 0usize;
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            // Quoted spans: skip to the closing delimiter, honouring backslash escapes.
            '\'' | '`' => {
                while let Some(d) = chars.next() {
                    if d == '\\' {
                        chars.next();
                    } else if d == c {
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'/') => {
                for d in chars.by_ref() {
                    if d == '\n' {
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut prev = ' ';
                for d in chars.by_ref() {
                    if prev == '*' && d == '/' {
                        break;
                    }
                    prev = d;
                }
            }
            '(' | '[' => {
                depth += 1;
                if depth > MAX_NESTING {
                    return Err(ParseError::Syntax(format!(
                        "nesting deeper than {MAX_NESTING}"
                    )));
                }
            }
            ')' | ']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}

fn build_expr(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    match pair.as_rule() {
        Rule::implies_expr => build_implies(pair),
        Rule::or_expr => build_or(pair),
        Rule::and_expr => build_and(pair),
        Rule::membership_expr => build_membership(pair),
        Rule::ineq_expr => build_ineq(pair),
        Rule::eq_expr => build_eq(pair),
        Rule::type_expr => build_type(pair),
        Rule::union_expr => build_union(pair),
        Rule::add_expr => build_add(pair),
        Rule::mul_expr => build_mul(pair),
        Rule::unary_expr => build_unary(pair),
        Rule::postfix_expr => build_postfix(pair),
        Rule::expression_inner => build_expr(only_child(pair)?),
        Rule::paren_expr => build_expr(only_child(pair)?),
        // Primary atoms
        Rule::null_lit => Ok(Expr::Null),
        Rule::bool_lit => Ok(Expr::Bool(pair.as_str() == "true")),
        Rule::integer_lit => parse_integer(pair),
        Rule::long_lit => parse_long(pair),
        Rule::decimal_lit => parse_decimal(pair),
        Rule::string_lit => Ok(Expr::String(unescape_string(pair.as_str()))),
        Rule::date_lit => Ok(Expr::Temporal(TemporalKind::Date, trim_at(pair.as_str()))),
        Rule::datetime_lit => Ok(Expr::Temporal(
            TemporalKind::DateTime,
            trim_at(pair.as_str()),
        )),
        Rule::time_lit => Ok(Expr::Temporal(TemporalKind::Time, trim_at_t(pair.as_str()))),
        Rule::quantity_lit => parse_quantity(pair),
        Rule::ident => Ok(Expr::Ident(ident_name(&pair))),
        Rule::func_call => build_func_call(pair),
        Rule::dollar_this => Ok(Expr::DollarThis),
        Rule::dollar_index => Ok(Expr::DollarIndex),
        other => Err(ParseError::Syntax(format!("unexpected rule: {other:?}"))),
    }
}

// ── Binary operator helpers ────────────────────────────────────────────────

fn build_implies(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut iter = pair.into_inner();
    let lhs = build_expr(iter.next().ok_or_else(missing)?)?;
    // Skip implies_kw if present, then parse the rhs
    match iter.next() {
        None => Ok(lhs),
        Some(_kw) => {
            let rhs = build_expr(iter.next().ok_or_else(missing)?)?;
            Ok(Expr::Implies(Box::new(lhs), Box::new(rhs)))
        }
    }
}

fn build_or(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut iter = pair.into_inner();
    let mut lhs = build_expr(iter.next().ok_or_else(missing)?)?;
    // alternating: (or_kw | xor_kw, and_expr)*
    while let Some(op_pair) = iter.next() {
        let is_xor = op_pair.as_rule() == Rule::xor_kw;
        let rhs = build_expr(iter.next().ok_or_else(missing)?)?;
        lhs = if is_xor {
            Expr::Xor(Box::new(lhs), Box::new(rhs))
        } else {
            Expr::Or(Box::new(lhs), Box::new(rhs))
        };
    }
    Ok(lhs)
}

fn build_and(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut iter = pair.into_inner();
    let mut lhs = build_expr(iter.next().ok_or_else(missing)?)?;
    // alternating: (and_kw, membership_expr)*
    while let Some(_kw) = iter.next() {
        let rhs = build_expr(iter.next().ok_or_else(missing)?)?;
        lhs = Expr::And(Box::new(lhs), Box::new(rhs));
    }
    Ok(lhs)
}

fn build_membership(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut iter = pair.into_inner();
    let lhs = build_expr(iter.next().ok_or_else(missing)?)?;
    match iter.next() {
        None => Ok(lhs),
        Some(op_pair) => {
            let is_contains = op_pair.as_rule() == Rule::contains_kw;
            let rhs = build_expr(iter.next().ok_or_else(missing)?)?;
            if is_contains {
                Ok(Expr::Contains(Box::new(lhs), Box::new(rhs)))
            } else {
                Ok(Expr::In(Box::new(lhs), Box::new(rhs)))
            }
        }
    }
}

fn build_ineq(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut iter = pair.into_inner();
    let lhs = build_expr(iter.next().ok_or_else(missing)?)?;
    match iter.next() {
        None => Ok(lhs),
        Some(op_pair) => {
            let op = op_pair.as_rule();
            let rhs = build_expr(iter.next().ok_or_else(missing)?)?;
            Ok(match op {
                Rule::lt => Expr::Lt(Box::new(lhs), Box::new(rhs)),
                Rule::lte => Expr::Lte(Box::new(lhs), Box::new(rhs)),
                Rule::gt => Expr::Gt(Box::new(lhs), Box::new(rhs)),
                Rule::gte => Expr::Gte(Box::new(lhs), Box::new(rhs)),
                other => return Err(ParseError::Syntax(format!("unexpected ineq op: {other:?}"))),
            })
        }
    }
}

fn build_eq(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut iter = pair.into_inner();
    let lhs = build_expr(iter.next().ok_or_else(missing)?)?;
    match iter.next() {
        None => Ok(lhs),
        Some(op_pair) => {
            let op = op_pair.as_rule();
            let rhs = build_expr(iter.next().ok_or_else(missing)?)?;
            Ok(match op {
                Rule::eq => Expr::Eq(Box::new(lhs), Box::new(rhs)),
                Rule::neq => Expr::Neq(Box::new(lhs), Box::new(rhs)),
                Rule::tilde_eq => Expr::Equiv(Box::new(lhs), Box::new(rhs)),
                Rule::tilde_neq => Expr::NotEquiv(Box::new(lhs), Box::new(rhs)),
                other => return Err(ParseError::Syntax(format!("unexpected eq op: {other:?}"))),
            })
        }
    }
}

fn build_type(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut iter = pair.into_inner();
    let lhs = build_expr(iter.next().ok_or_else(missing)?)?;
    match iter.next() {
        None => Ok(lhs),
        Some(op_pair) => {
            let is_as = op_pair.as_rule() == Rule::as_kw;
            let type_spec = iter
                .next()
                .ok_or_else(missing)?
                .into_inner()
                .map(|p| ident_name(&p))
                .collect::<Vec<_>>()
                .join(".");
            if is_as {
                Ok(Expr::As(Box::new(lhs), type_spec))
            } else {
                Ok(Expr::Is(Box::new(lhs), type_spec))
            }
        }
    }
}

fn build_union(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut iter = pair.into_inner();
    let mut lhs = build_expr(iter.next().ok_or_else(missing)?)?;
    // alternating: (pipe, add_expr)*
    while let Some(_pipe) = iter.next() {
        let rhs = build_expr(iter.next().ok_or_else(missing)?)?;
        lhs = Expr::Union(Box::new(lhs), Box::new(rhs));
    }
    Ok(lhs)
}

fn build_add(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut iter = pair.into_inner();
    let mut lhs = build_expr(iter.next().ok_or_else(missing)?)?;
    let children: Vec<Pair<Rule>> = iter.collect();
    let mut i = 0;
    while i < children.len() {
        let op = children[i].as_rule();
        i += 1;
        let rhs = build_expr(children[i].clone())?;
        i += 1;
        lhs = match op {
            Rule::plus => Expr::Add(Box::new(lhs), Box::new(rhs)),
            Rule::minus => Expr::Sub(Box::new(lhs), Box::new(rhs)),
            Rule::amp => Expr::Concat(Box::new(lhs), Box::new(rhs)),
            other => return Err(ParseError::Syntax(format!("unexpected add op: {other:?}"))),
        };
    }
    Ok(lhs)
}

fn build_mul(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut iter = pair.into_inner();
    let mut lhs = build_expr(iter.next().ok_or_else(missing)?)?;
    let children: Vec<Pair<Rule>> = iter.collect();
    let mut i = 0;
    while i < children.len() {
        let op = children[i].as_rule();
        i += 1;
        let rhs = build_expr(children[i].clone())?;
        i += 1;
        lhs = match op {
            Rule::star => Expr::Mul(Box::new(lhs), Box::new(rhs)),
            Rule::slash => Expr::Div(Box::new(lhs), Box::new(rhs)),
            Rule::div_kw => Expr::DivInt(Box::new(lhs), Box::new(rhs)),
            Rule::mod_kw => Expr::Mod(Box::new(lhs), Box::new(rhs)),
            other => return Err(ParseError::Syntax(format!("unexpected mul op: {other:?}"))),
        };
    }
    Ok(lhs)
}

fn build_unary(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut iter = pair.into_inner();
    let first = iter.next().ok_or_else(missing)?;
    match first.as_rule() {
        Rule::postfix_expr => build_postfix(first),
        Rule::unary_minus => {
            let operand = build_expr(iter.next().ok_or_else(missing)?)?;
            Ok(Expr::Neg(Box::new(operand)))
        }
        Rule::unary_plus => {
            // unary + is a no-op
            build_expr(iter.next().ok_or_else(missing)?)
        }
        other => Err(ParseError::Syntax(format!(
            "unexpected unary child: {other:?}"
        ))),
    }
}

fn build_postfix(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut iter = pair.into_inner();
    let mut base = build_expr(iter.next().ok_or_else(missing)?)?;
    for step in iter {
        match step.as_rule() {
            Rule::dot_step => {
                let inner = step.into_inner().next().ok_or_else(missing)?;
                match inner.as_rule() {
                    Rule::func_call => {
                        let (name, args) = parse_func_call_inner(inner)?;
                        base = Expr::Method(Box::new(base), name, args);
                    }
                    Rule::ident => {
                        base = Expr::Dot(Box::new(base), Box::new(Expr::Ident(ident_name(&inner))));
                    }
                    other => {
                        return Err(ParseError::Syntax(format!(
                            "unexpected dot step: {other:?}"
                        )));
                    }
                }
            }
            Rule::index_step => {
                let inner = step.into_inner().next().ok_or_else(missing)?;
                let index_expr = build_expr(inner)?;
                base = Expr::Index(Box::new(base), Box::new(index_expr));
            }
            other => {
                return Err(ParseError::Syntax(format!(
                    "unexpected postfix step: {other:?}"
                )));
            }
        }
    }
    Ok(base)
}

fn build_func_call(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let (name, args) = parse_func_call_inner(pair)?;
    Ok(Expr::FuncCall(name, args))
}

fn parse_func_call_inner(pair: Pair<Rule>) -> Result<(String, Vec<Expr>), ParseError> {
    let mut iter = pair.into_inner();
    let name = ident_name(&iter.next().ok_or_else(missing)?);
    let args = iter.map(build_expr).collect::<Result<Vec<_>, _>>()?;
    Ok((name, args))
}

// ── Literal parsers ────────────────────────────────────────────────────────

fn parse_integer(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let s = pair.into_inner().next().ok_or_else(missing)?.as_str();
    s.parse::<i64>()
        .map(Expr::Integer)
        .map_err(|_| ParseError::Syntax(format!("invalid integer: {s}")))
}

fn parse_long(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let raw = pair.as_str();
    let s = raw.trim_end_matches('L');
    s.parse::<i64>()
        .map(Expr::Long)
        .map_err(|_| ParseError::Syntax(format!("invalid long literal: {raw}")))
}

fn parse_decimal(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let s = pair.into_inner().next().ok_or_else(missing)?.as_str();
    s.parse::<f64>()
        .map(Expr::Decimal)
        .map_err(|_| ParseError::Syntax(format!("invalid decimal: {s}")))
}

fn parse_quantity(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let text = pair.as_str();
    // Format: <number> '<unit>'
    // Split on first whitespace
    let (num_str, rest) = text
        .split_once(' ')
        .ok_or_else(|| ParseError::Syntax(format!("invalid quantity: {text}")))?;
    let value = num_str
        .parse::<f64>()
        .map_err(|_| ParseError::Syntax(format!("invalid quantity number: {num_str}")))?;
    let unit = strip_quotes(rest.trim()).to_owned();
    Ok(Expr::Quantity(value, unit))
}

/// Remove the leading `@` from a date/dateTime literal.
fn trim_at(s: &str) -> String {
    s.trim_start_matches('@').to_owned()
}

/// Remove the leading `@T` from a time literal.
fn trim_at_t(s: &str) -> String {
    s.trim_start_matches("@T").to_owned()
}

/// Remove exactly one leading and one trailing `'` (escaped quotes inside stay intact).
fn strip_quotes(raw: &str) -> &str {
    raw.strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
        .unwrap_or(raw)
}

/// Unescape a FHIRPath string literal (strips surrounding `'`).
fn unescape_string(raw: &str) -> String {
    unescape(strip_quotes(raw))
}

/// Name of an `ident` pair; a backtick-delimited identifier is unescaped.
fn ident_name(pair: &Pair<Rule>) -> String {
    let raw = pair.as_str();
    raw.strip_prefix('`')
        .and_then(|s| s.strip_suffix('`'))
        .map_or_else(|| raw.to_owned(), unescape)
}

/// Resolve the FHIRPath `ESC` sequences shared by strings and delimited identifiers.
fn unescape(inner: &str) -> String {
    let mut result = String::with_capacity(inner.len());
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some(c @ ('\'' | '`' | '"' | '/')) => result.push(c),
                Some('f') => result.push('\u{c}'),
                Some('\\') => result.push('\\'),
                Some('n') => result.push('\n'),
                Some('r') => result.push('\r'),
                Some('t') => result.push('\t'),
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).collect();
                    if let Ok(n) = u32::from_str_radix(&hex, 16) {
                        if let Some(c) = char::from_u32(n) {
                            result.push(c);
                        }
                    }
                }
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }
    result
}

// ── Utilities ──────────────────────────────────────────────────────────────

fn only_child(pair: Pair<Rule>) -> Result<Pair<Rule>, ParseError> {
    pair.into_inner()
        .next()
        .ok_or_else(|| ParseError::Syntax("expected child pair".into()))
}

fn missing() -> ParseError {
    ParseError::Syntax("missing expected pair".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_with_escaped_quote_at_end() {
        assert_eq!(parse(r"'it\''").unwrap(), Expr::String("it'".into()));
        assert_eq!(parse(r"'\''").unwrap(), Expr::String("'".into()));
    }

    #[test]
    fn quantity_unit_keeps_escaped_quote() {
        assert_eq!(
            parse(r"1 'a\''").unwrap(),
            Expr::Quantity(1.0, r"a\'".into())
        );
    }

    #[test]
    fn identifiers_starting_with_bool_words() {
        assert_eq!(parse("trueValue").unwrap(), Expr::Ident("trueValue".into()));
        assert!(matches!(parse("falsePositive.x").unwrap(), Expr::Dot(..)));
        assert_eq!(parse("true").unwrap(), Expr::Bool(true));
    }

    #[test]
    fn deep_nesting_is_rejected_not_overflowed() {
        let deep = format!("{}1{}", "(".repeat(5000), ")".repeat(5000));
        assert!(parse(&deep).is_err());
        let ok = format!("{}1{}", "(".repeat(10), ")".repeat(10));
        assert!(parse(&ok).is_ok());
    }

    #[test]
    fn parens_inside_strings_do_not_count() {
        let s = format!("'{}'", "(".repeat(500));
        assert!(parse(&s).is_ok());
    }

    #[test]
    fn apostrophes_in_comments_do_not_hide_nesting() {
        let deep = format!("// it's\n{}1{}", "(".repeat(500), ")".repeat(500));
        assert!(parse(&deep).is_err());
        let block = format!("/* it's */ {}1{}", "(".repeat(500), ")".repeat(500));
        assert!(parse(&block).is_err());
        assert!(parse("1 /* ((((( */").is_ok());
    }

    #[test]
    fn repeated_unary_operators_are_syntax_errors() {
        assert!(parse(&format!("{}1", "- ".repeat(5000))).is_err());
    }

    #[test]
    fn oversized_input_is_rejected() {
        assert!(parse(&"1+".repeat(MAX_INPUT_LEN)).is_err());
    }

    /// Parse, evaluate and drop the worst accepted inputs on a 2 MB stack.
    #[test]
    fn worst_case_accepted_inputs_fit_in_small_stack() {
        let fit = |s: String| s[..MAX_INPUT_LEN.min(s.len())].to_string();
        let nest = MAX_NESTING;
        let cases = [
            fit("a.".repeat(MAX_INPUT_LEN) + "a"),
            fit("1+".repeat(MAX_INPUT_LEN) + "1"),
            fit("a and ".repeat(MAX_INPUT_LEN) + "a"),
            fit("a|".repeat(MAX_INPUT_LEN) + "a"),
            format!("{}1{}", "(".repeat(nest), ")".repeat(nest)),
            format!("{}1{}", "[".repeat(nest), "]".repeat(nest)),
            format!("{}a{}", "a.where(".repeat(nest), ")".repeat(nest)),
            format!("{}1{}", "-(".repeat(nest), ")".repeat(nest)),
        ];
        for c in cases {
            std::thread::Builder::new()
                .stack_size(2 << 20)
                .spawn(move || {
                    if let Ok(e) = parse(&c) {
                        let _ = crate::eval::eval(&e, &[], &|_, _| false);
                    }
                })
                .unwrap()
                .join()
                .expect("stack overflow or panic");
        }
    }

    #[test]
    fn is_binds_tighter_than_union() {
        match parse("a | b is X").unwrap() {
            Expr::Union(_, rhs) => assert!(matches!(*rhs, Expr::Is(..))),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn delimited_identifier_parses_to_plain_identifier() {
        let div = || Box::new(Expr::Ident("div".into()));
        assert_eq!(
            parse("text.`div`").unwrap(),
            Expr::Dot(Box::new(Expr::Ident("text".into())), div()),
        );
        assert_eq!(parse("`div`").unwrap(), *div());
        assert_eq!(parse(r"`a\`b`").unwrap(), Expr::Ident("a`b".into()));
        assert_eq!(parse("`a b\\u0041`").unwrap(), Expr::Ident("a bA".into()));
        for (delimited, plain) in [
            ("1 is `System`.`Integer`", "1 is System.Integer"),
            ("a.`exists`()", "a.exists()"),
            ("`today`()", "today()"),
        ] {
            assert_eq!(
                parse(delimited).unwrap(),
                parse(plain).unwrap(),
                "{delimited}"
            );
        }
        assert!(parse("`unterminated").is_err());
    }

    #[test]
    fn spec_escapes_are_accepted_in_strings() {
        assert_eq!(
            parse(r#"'\"\/\f\`'"#).unwrap(),
            Expr::String("\"/\u{c}`".into())
        );
    }

    #[test]
    fn comparison_of_union_and_equality_still_parse() {
        assert!(matches!(parse("a | b = c").unwrap(), Expr::Eq(..)));
        assert!(matches!(parse("a is X = true").unwrap(), Expr::Eq(..)));
    }
}
