//! Unit conversion for FHIRPath Quantity values.
//!
//! The prefixes and units come from `data/ucum-essence.xml`, an unmodified
//! copy of the UCUM essence file (see `data/UCUM-NOTICE.txt` for its
//! copyright notice and licence). It is parsed once, on first use.
//!
//! Factors are exact ratios. Units the converter cannot express exactly
//! are unsupported and convert to nothing: special (non-linear) units such
//! as `Cel` or `[degF]`, arbitrary units such as `[IU]`, and units whose
//! factor does not fit an `i128` ratio (for example `[pi]` and the units
//! defined from it).

mod ratio;
mod term;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::sync::OnceLock;

use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};

pub(crate) use ratio::Ratio;
use term::{AtomLookup, Unit, parse_unit};

const ESSENCE: &str = include_str!("../data/ucum-essence.xml");

/// Order of the `dim` letters in `<base-unit>`, matching [`term::Dimension`].
const BASE_DIMENSIONS: [&str; 7] = ["L", "T", "M", "A", "C", "Q", "F"];

/// The factor that converts an amount in `from` into `to`
/// (`amount_in_to = amount_in_from * factor`).
///
/// `None` if either unit is malformed, unknown or unsupported, or the two
/// have different dimensions.
pub(crate) fn convert(from: &str, to: &str) -> Option<Ratio> {
    let (from, to) = (resolve(from)?, resolve(to)?);
    if from.dimension != to.dimension {
        return None;
    }
    from.factor.div(to.factor)
}

/// The unit of `left` times `right` (`power` 1) or `left` divided by
/// `right` (`power` -1), for N1 §6.6 `*` and `/` on Quantities. Powers of
/// the same atom merge and cancel (`cm` * `cm` is `cm2`, `m` / `m` is `1`);
/// units with parentheses or annotations are kept whole in parentheses.
/// `None` if either unit, or the result, is not a supported UCUM unit.
pub(crate) fn combine(left: &str, right: &str, power: i32) -> Option<String> {
    resolve(left)?;
    resolve(right)?;
    let unit = match (simple_terms(left), simple_terms(right)) {
        (Some(left_terms), Some(right_terms)) => {
            let mut terms: Vec<(&str, i32)> = Vec::new();
            let right_terms = right_terms
                .into_iter()
                .map(|(atom, exponent)| (atom, power, exponent));
            for (atom, sign, exponent) in left_terms
                .into_iter()
                .map(|(atom, exponent)| (atom, 1, exponent))
                .chain(right_terms)
            {
                let exponent = exponent.checked_mul(sign)?;
                match terms.iter_mut().find(|(existing, _)| *existing == atom) {
                    Some((_, total)) => *total = total.checked_add(exponent)?,
                    None => terms.push((atom, exponent)),
                }
            }
            render_terms(&terms)
        }
        _ => format!("({left}){}({right})", if power < 0 { '/' } else { '.' }),
    };
    resolve(&unit).map(|_| unit)
}

/// A unit made only of `.` and `/` separated atoms with exponents, as
/// `(atom, exponent)` pairs (`g/m2` is `[("g", 1), ("m", -2)]`); the number
/// `1` contributes nothing. `None` for any other form. Callers resolve the
/// unit first, which allows a single leading `/`.
fn simple_terms(unit: &str) -> Option<Vec<(&str, i32)>> {
    if unit.contains(['(', ')', '{', '}']) {
        return None;
    }
    // A leading `/` inverts the whole term, unlike an inner one.
    if let Some(inverted) = unit.strip_prefix('/') {
        return simple_terms(inverted)?
            .into_iter()
            .map(|(atom, exponent)| Some((atom, exponent.checked_neg()?)))
            .collect();
    }
    let mut terms = Vec::new();
    let mut sign = 1;
    let mut start = 0;
    let mut depth = 0u32;
    for (index, byte) in unit.bytes().chain(*b".").enumerate() {
        match byte {
            b'[' => depth += 1,
            b']' => depth = depth.checked_sub(1)?,
            b'.' | b'/' if depth == 0 => {
                let component = &unit[start..index];
                if component != "1" {
                    // `10*3` reads as atom `10*` to the power 3, which is what it means.
                    let (atom, exponent) = term::split_exponent(component)?;
                    if atom.bytes().all(|byte| byte.is_ascii_digit()) {
                        return None;
                    }
                    terms.push((atom, exponent.checked_mul(sign)?));
                }
                sign = if byte == b'/' { -1 } else { 1 };
                start = index + 1;
            }
            _ => {}
        }
    }
    Some(terms)
}

/// `[("g", 1), ("m", -2)]` as `g/m2`; no atoms left is `1`.
fn render_terms(terms: &[(&str, i32)]) -> String {
    let power = |(atom, exponent): &(&str, i32)| match exponent.unsigned_abs() {
        1 => (*atom).to_owned(),
        magnitude => format!("{atom}{magnitude}"),
    };
    let numerator: Vec<String> = terms.iter().filter(|term| term.1 > 0).map(power).collect();
    let mut text = if numerator.is_empty() {
        "1".to_owned()
    } else {
        numerator.join(".")
    };
    for term in terms.iter().filter(|term| term.1 < 0) {
        text.push('/');
        text.push_str(&power(term));
    }
    text
}

/// Whether amounts in `unit_a` and `unit_b` can be converted into each other.
#[cfg(test)]
pub(crate) fn comparable(unit_a: &str, unit_b: &str) -> bool {
    convert(unit_a, unit_b).is_some()
}

fn resolve(unit: &str) -> Option<Unit> {
    let table = table();
    parse_unit(unit, &table.prefixes, &mut |code| {
        table.atoms.get(code).copied()
    })
}

struct Table {
    prefixes: HashMap<String, Ratio>,
    /// Atom code to (is metric, unit reduced to base units if supported).
    atoms: HashMap<String, (bool, Option<Unit>)>,
}

fn table() -> &'static Table {
    static TABLE: OnceLock<Table> = OnceLock::new();
    TABLE.get_or_init(load)
}

/// An atom as written in the essence file, before reduction to base units.
enum Definition {
    Base(usize),
    Derived { unit: String, value: Ratio },
    Unsupported,
}

fn load() -> Table {
    let (prefixes, raw) = read_essence();
    let mut resolved: HashMap<String, Option<Unit>> = HashMap::new();
    for code in raw.keys() {
        resolve_atom(code, &raw, &prefixes, &mut resolved);
    }
    let atoms = raw
        .iter()
        .map(|(code, (metric, _))| {
            (
                code.clone(),
                (*metric, resolved.get(code).copied().flatten()),
            )
        })
        .collect();
    Table { prefixes, atoms }
}

fn resolve_atom(
    code: &str,
    raw: &HashMap<String, (bool, Definition)>,
    prefixes: &HashMap<String, Ratio>,
    resolved: &mut HashMap<String, Option<Unit>>,
) -> Option<Unit> {
    if let Some(unit) = resolved.get(code) {
        return *unit;
    }
    // Placeholder while resolving: a definition cycle resolves to unsupported
    // instead of recursing forever; chains are bounded by the atom count.
    resolved.insert(code.to_owned(), None);
    let unit = match &raw.get(code)?.1 {
        Definition::Base(index) => {
            let mut dimension = [0; 7];
            dimension[*index] = 1;
            Some(Unit {
                factor: Ratio::ONE,
                dimension,
            })
        }
        Definition::Derived { unit, value } => {
            let mut lookup = |atom: &str| -> AtomLookup {
                let metric = raw.get(atom)?.0;
                Some((metric, resolve_atom(atom, raw, prefixes, resolved)))
            };
            parse_unit(unit, prefixes, &mut lookup).and_then(|base| {
                Some(Unit {
                    factor: base.factor.mul(*value)?,
                    ..base
                })
            })
        }
        Definition::Unsupported => None,
    };
    resolved.insert(code.to_owned(), unit);
    unit
}

/// Read prefixes and atom definitions from the embedded essence file.
fn read_essence() -> (HashMap<String, Ratio>, HashMap<String, (bool, Definition)>) {
    let mut prefixes = HashMap::new();
    let mut atoms = HashMap::new();
    // (element name, Code, isMetric, special or arbitrary) of the open entry.
    let mut entry: Option<(Vec<u8>, String, bool, bool)> = None;
    let mut reader = Reader::from_str(ESSENCE);
    loop {
        let element = match reader.read_event() {
            Ok(Event::Start(element) | Event::Empty(element)) => element,
            // The file is embedded and checksummed, so it always parses to the end.
            Ok(Event::Eof) | Err(_) => break,
            Ok(_) => continue,
        };
        match element.name().as_ref() {
            b"base-unit" => {
                let code = attribute(&element, "Code");
                let index = BASE_DIMENSIONS
                    .iter()
                    .position(|letter| Some(*letter) == attribute(&element, "dim").as_deref());
                if let (Some(code), Some(index)) = (code, index) {
                    atoms.insert(code, (true, Definition::Base(index)));
                }
            }
            name @ (b"prefix" | b"unit") => {
                entry = attribute(&element, "Code").map(|code| {
                    let flag = |name| attribute(&element, name).as_deref() == Some("yes");
                    let unsupported = flag("isSpecial") || flag("isArbitrary");
                    (name.to_vec(), code, flag("isMetric"), unsupported)
                });
            }
            b"value" => {
                let Some((kind, code, metric, unsupported)) = entry.take() else {
                    continue;
                };
                let value = attribute(&element, "value").and_then(|v| Ratio::parse_decimal(&v));
                if kind == b"prefix" {
                    if let Some(value) = value {
                        prefixes.insert(code, value);
                    }
                    continue;
                }
                let definition = match (value, attribute(&element, "Unit")) {
                    (Some(value), Some(unit)) if !unsupported => {
                        Definition::Derived { unit, value }
                    }
                    _ => Definition::Unsupported,
                };
                atoms.insert(code, (metric, definition));
            }
            _ => {}
        }
    }
    (prefixes, atoms)
}

fn attribute(element: &BytesStart<'_>, name: &str) -> Option<String> {
    let attribute = element.try_get_attribute(name).ok()??;
    Some(
        attribute
            .normalized_value(XmlVersion::Explicit1_0)
            .ok()?
            .into_owned(),
    )
}
