//! Unit dimensions and the parser for unit expressions such as `kg/(m.s2)`.

use std::collections::HashMap;

use super::ratio::Ratio;

/// Exponents of the seven base units: length, time, mass, plane angle,
/// temperature, electric charge, luminous intensity.
pub(crate) type Dimension = [i32; 7];

/// A unit reduced to base units: `1 unit = factor * base units^dimension`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Unit {
    pub(crate) factor: Ratio,
    pub(crate) dimension: Dimension,
}

impl Unit {
    pub(crate) const ONE: Unit = Unit {
        factor: Ratio::ONE,
        dimension: [0; 7],
    };

    pub(crate) fn mul(self, other: Unit) -> Option<Unit> {
        let mut dimension = self.dimension;
        for (exponent, added) in dimension.iter_mut().zip(other.dimension) {
            *exponent = exponent.checked_add(added)?;
        }
        Some(Unit {
            factor: self.factor.mul(other.factor)?,
            dimension,
        })
    }

    pub(crate) fn pow(self, power: i32) -> Option<Unit> {
        let mut dimension = self.dimension;
        for exponent in &mut dimension {
            *exponent = exponent.checked_mul(power)?;
        }
        Some(Unit {
            factor: self.factor.pow(power)?,
            dimension,
        })
    }
}

/// What an atom lookup found: `None` if the code is not an atom, else
/// whether it is metric and its resolved unit (`None` if unsupported).
pub(crate) type AtomLookup = Option<(bool, Option<Unit>)>;

/// Bound on parenthesis nesting, so hostile input cannot overflow the stack.
const MAX_NESTING: usize = 32;

/// Parse a unit expression; `None` if it is malformed, unknown or unsupported.
pub(crate) fn parse_unit(
    text: &str,
    prefixes: &HashMap<String, Ratio>,
    lookup: &mut dyn FnMut(&str) -> AtomLookup,
) -> Option<Unit> {
    // Unit codes are printable ASCII; a space is only allowed inside an annotation.
    let mut in_annotation = false;
    for byte in text.bytes() {
        match byte {
            b'{' => in_annotation = true,
            b'}' => in_annotation = false,
            b' ' if in_annotation => {}
            33..=126 => {}
            _ => return None,
        }
    }
    let mut parser = Parser {
        text: text.as_bytes(),
        position: 0,
        nesting: 0,
        prefixes,
        lookup,
    };
    let unit = if parser.eat(b'/') {
        parser.term()?.pow(-1)?
    } else {
        parser.term()?
    };
    (parser.position == parser.text.len()).then_some(unit)
}

struct Parser<'a> {
    text: &'a [u8],
    position: usize,
    nesting: usize,
    prefixes: &'a HashMap<String, Ratio>,
    lookup: &'a mut dyn FnMut(&str) -> AtomLookup,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.text.get(self.position).copied()
    }

    fn eat(&mut self, byte: u8) -> bool {
        let found = self.peek() == Some(byte);
        if found {
            self.position += 1;
        }
        found
    }

    fn term(&mut self) -> Option<Unit> {
        let mut unit = self.component()?;
        loop {
            if self.eat(b'.') {
                unit = unit.mul(self.component()?)?;
            } else if self.eat(b'/') {
                unit = unit.mul(self.component()?.pow(-1)?)?;
            } else {
                return Some(unit);
            }
        }
    }

    fn component(&mut self) -> Option<Unit> {
        if self.eat(b'(') {
            self.nesting += 1;
            if self.nesting > MAX_NESTING {
                return None;
            }
            let unit = self.term()?;
            self.nesting -= 1;
            return self.eat(b')').then_some(unit);
        }
        if self.peek() == Some(b'{') {
            self.annotation()?;
            return Some(Unit::ONE);
        }
        let symbol = self.symbol()?;
        // A zero factor would make the unit uninvertible: not a usable unit.
        if symbol.bytes().all(|byte| byte.is_ascii_digit()) {
            return Some(Unit {
                factor: Ratio::new(symbol.parse().ok()?, 1).filter(|factor| factor.numer() != 0)?,
                dimension: [0; 7],
            });
        }
        let (atom, exponent) = split_exponent(symbol)?;
        if self.peek() == Some(b'{') {
            self.annotation()?;
        }
        self.simple_unit(atom)?.pow(exponent)
    }

    /// Read up to the next operator, parenthesis or annotation; square
    /// brackets are part of the symbol (`[in_i]`, `mm[Hg]`).
    fn symbol(&mut self) -> Option<&'a str> {
        let text: &'a [u8] = self.text;
        let start = self.position;
        while let Some(byte) = self.peek() {
            match byte {
                b'.' | b'/' | b'(' | b')' | b'{' | b'}' => break,
                b'[' => {
                    let close = self.text[self.position..].iter().position(|&b| b == b']')?;
                    self.position += close + 1;
                }
                _ => self.position += 1,
            }
        }
        let symbol = std::str::from_utf8(&text[start..self.position]).ok()?;
        (!symbol.is_empty()).then_some(symbol)
    }

    /// Skip a `{...}` annotation; it does not change the unit.
    fn annotation(&mut self) -> Option<()> {
        let close = self.text[self.position..].iter().position(|&b| b == b'}')?;
        let body = &self.text[self.position + 1..self.position + close];
        if body.contains(&b'{') {
            return None;
        }
        self.position += close + 1;
        Some(())
    }

    /// An atom, or a prefix followed by a metric atom (`mg`, `kPa`, `mm[Hg]`).
    fn simple_unit(&mut self, atom: &str) -> Option<Unit> {
        if let Some((_, unit)) = (self.lookup)(atom) {
            return unit;
        }
        for split in 1..atom.len().min(3) {
            let Some(prefix) = self.prefixes.get(&atom[..split]).copied() else {
                continue;
            };
            if let Some((true, unit)) = (self.lookup)(&atom[split..]) {
                let unit = unit?;
                return Some(Unit {
                    factor: unit.factor.mul(prefix)?,
                    ..unit
                });
            }
        }
        None
    }
}

/// Split a trailing signed integer exponent off a symbol: `m2`, `s-1`, `10*3`.
pub(super) fn split_exponent(symbol: &str) -> Option<(&str, i32)> {
    let digits_start = symbol.trim_end_matches(|c: char| c.is_ascii_digit()).len();
    if digits_start == symbol.len() {
        return Some((symbol, 1));
    }
    let atom_end = match symbol.as_bytes().get(digits_start.wrapping_sub(1)) {
        Some(b'+' | b'-') => digits_start - 1,
        _ => digits_start,
    };
    if atom_end == 0 {
        return None;
    }
    Some((&symbol[..atom_end], symbol[atom_end..].parse().ok()?))
}
