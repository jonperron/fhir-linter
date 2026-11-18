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
