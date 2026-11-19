//! JSON parser limits: nesting depth, BOM and duplicate keys.

use fhir_parser::parse_json;

#[test]
fn deep_nesting_is_an_error_not_a_stack_overflow() {
    let src = format!(
        "{{\"resourceType\":\"X\",\"a\":{}{}}}",
        "[".repeat(100_000),
        "]".repeat(100_000)
    );
    assert!(parse_json(&src).is_err());
}

#[test]
fn leading_utf8_bom_is_skipped_and_offsets_count_it() {
    let src = "\u{FEFF}{\"resourceType\":\"Patient\",\"active\":true}";
    let resource = parse_json(src).unwrap();
    assert_eq!(&*resource.resource_type, "Patient");
    let active = &resource.fields["active"];
    // Spans stay byte offsets into the input as given, BOM included.
    assert_eq!(active.span.offset as usize, src.find("true").unwrap());
    // Only a leading BOM is skipped.
    assert!(parse_json("{\"resourceType\":\"Patient\"}\u{FEFF}").is_err());
    assert!(parse_json("\u{FEFF}\u{FEFF}{\"resourceType\":\"Patient\"}").is_err());
}

#[test]
fn duplicate_key_is_rejected() {
    assert!(parse_json(r#"{"resourceType":"X","id":"a","id":"b"}"#).is_err());
}
