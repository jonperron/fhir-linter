use super::{Ratio, comparable, convert};

fn ratio(numer: i128, denom: i128) -> Option<Ratio> {
    Ratio::new(numer, denom)
}

#[test]
fn embedded_essence_is_unmodified() {
    let data = include_bytes!("../../data/ucum-essence.xml");
    // FNV-1a 64 of the upstream file; any edit to the copy changes it.
    let hash = data.iter().fold(0xcbf2_9ce4_8422_2325_u64, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    });
    assert_eq!(data.len(), 82_332);
    assert_eq!(hash, 0x2034_0979_0829_726f);
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/ucum/ucum-essence.xml"
    );
    if let Ok(upstream) = std::fs::read(fixture) {
        assert_eq!(upstream.as_slice(), data.as_slice());
    }
}

#[test]
fn metric_prefixes() {
    assert_eq!(convert("g", "mg"), ratio(1000, 1));
    assert_eq!(convert("mg", "g"), ratio(1, 1000));
    assert_eq!(convert("kg", "g"), ratio(1000, 1));
    assert_eq!(convert("L", "mL"), ratio(1000, 1));
    assert_eq!(convert("dL", "L"), ratio(1, 10));
}

#[test]
fn customary_and_time_units() {
    assert_eq!(convert("[in_i]", "cm"), ratio(127, 50));
    assert_eq!(convert("cm", "[in_i]"), ratio(50, 127));
    assert_eq!(convert("wk", "d"), ratio(7, 1));
    assert_eq!(convert("min", "s"), ratio(60, 1));
    assert_eq!(convert("h", "min"), ratio(60, 1));
    assert_eq!(convert("[lb_av]", "g"), ratio(45_359_237, 100_000));
}

#[test]
fn mm_hg_is_linear() {
    // 1 mm[Hg] = 133.3220 Pa
    assert_eq!(convert("mm[Hg]", "Pa"), ratio(1_333_220, 10_000));
    assert!(comparable("mm[Hg]", "kPa"));
}

#[test]
fn unit_terms() {
    assert_eq!(convert("m/s", "km/h"), ratio(18, 5));
    assert_eq!(convert("cm2", "m2"), ratio(1, 10_000));
    assert_eq!(convert("m.s-1", "m/s"), ratio(1, 1));
    assert_eq!(convert("10*3/uL", "10*9/L"), ratio(1, 1));
    assert_eq!(convert("/min", "/s"), ratio(1, 60));
    assert_eq!(convert("{beats}/min", "/h"), ratio(60, 1));
    assert_eq!(convert("kg/(m.s2)", "Pa"), ratio(1, 1));
    assert_eq!(convert("N", "kg.m/s2"), ratio(1, 1));
    assert_eq!(convert("%", "1"), ratio(1, 100));
    // a leading `/` inverts the whole term (UCUM main-term grammar)
    assert_eq!(convert("/s.m", "/(s.m)"), ratio(1, 1));
    assert!(comparable("m", "[ft_i]"));
}

#[test]
fn incompatible_dimensions() {
    assert_eq!(convert("g", "m"), None);
    assert_eq!(convert("kg", "L"), None);
    assert!(!comparable("s", "m"));
}

#[test]
fn unknown_units() {
    assert_eq!(convert("xyz", "g"), None);
    assert_eq!(convert("g", "kkg"), None);
    // prefixes apply to metric units only
    assert_eq!(convert("k[in_i]", "m"), None);
    assert!(!comparable("xyz", "xyz"));
}

#[test]
fn unsupported_units() {
    // special (non-linear) and arbitrary units
    assert_eq!(convert("Cel", "K"), None);
    assert_eq!(convert("[degF]", "Cel"), None);
    assert_eq!(convert("[IU]", "[IU]"), None);
}

#[test]
fn malformed_units() {
    for unit in [
        "",
        "m//s",
        "m.",
        ".m",
        "(m",
        "m)",
        "()",
        "m{",
        "{a",
        "[in_i",
        "m s",
        "m^",
        "g2x",
        "m+",
        "mé",
        "m99999999999",
        "/",
        "0.m",
        "m/0",
    ] {
        assert_eq!(convert(unit, "m"), None, "{unit}");
    }
    let deep = format!("{}m{}", "(".repeat(10_000), ")".repeat(10_000));
    assert_eq!(convert(&deep, "m"), None);
    assert_eq!(convert("g999999999", "g999999999"), ratio(1, 1));
}

#[test]
fn atoms_resolve_except_unsupported() {
    // 310 atoms; the 75 others are special, arbitrary, or need factors beyond
    // i128 ([pi], [h] and the units defined from them).
    let table = super::table();
    assert_eq!(table.prefixes.len(), 24);
    assert_eq!(table.atoms.len(), 310);
    let supported = table
        .atoms
        .values()
        .filter(|(_, unit)| unit.is_some())
        .count();
    assert_eq!(supported, 235);
}

#[test]
fn ratio_arithmetic_overflow_and_zero_return_none() {
    assert_eq!(Ratio::new(1, 0), None);
    assert_eq!(Ratio::new(2, 4), ratio(1, 2));
    assert_eq!(Ratio::new(1, -2), ratio(-1, 2));
    let big = Ratio::new(i128::MAX, 1);
    assert!(big.is_some_and(|big| big.mul(big).is_none()));
    assert_eq!(Ratio::new(0, 1).and_then(Ratio::recip), None);
    assert_eq!(Ratio::new(10, 1).and_then(|ten| ten.pow(100)), None);
    assert_eq!(Ratio::new(10, 1).and_then(|ten| ten.pow(-2)), ratio(1, 100));
    assert_eq!(Ratio::parse_decimal("254e-2"), ratio(127, 50));
    assert_eq!(Ratio::parse_decimal("1e400"), None);
    assert_eq!(Ratio::parse_decimal("abc"), None);
}

#[test]
fn convert_factor_overflow_returns_none() {
    assert_eq!(convert("10*30.10*30", "1"), None);
}
