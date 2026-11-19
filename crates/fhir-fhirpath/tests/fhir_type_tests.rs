//! `is`, `as` and `ofType` on values that carry their FHIR type, answered
//! through FHIR inheritance from the real R5 `Registry` (`code` is a
//! `string`, `canonical` a `uri`), and the error without that knowledge.
use std::error::Error;
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_fhirpath::{
    Collection, Environment, EvalError, Value, evaluate_parsed_with_environment, parse,
    resource_to_typed_value,
};
use fhir_parser::{Resource, parse_json, parse_xml};

type TestResult = Result<(), Box<dyn Error>>;

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).expect("R5 definitions")
    })
}

fn is_subtype(child: &str, ancestor: &str) -> bool {
    registry().type_ancestors(child).any(|a| a == ancestor)
}

fn typed(resource: &Resource) -> Value {
    let registry = registry();
    resource_to_typed_value(resource, &|parent, name| {
        registry.child_element(parent, name)
    })
}

fn questionnaire() -> Result<Value, Box<dyn Error>> {
    let resource = parse_json(
        r#"{"resourceType":"Questionnaire","id":"q1","status":"draft",
        "url":"http://example.org/q","title":"http://example.org/q",
        "derivedFrom":["http://example.org/base"],
        "_version":{"extension":[{"url":"http://example.org/x","valueBoolean":true}]}}"#,
    )?;
    Ok(typed(&resource))
}

fn eval(expression: &str, with_hook: bool) -> Result<Collection, Box<dyn Error>> {
    eval_on(questionnaire()?, expression, with_hook)
}

fn eval_on(input: Value, expression: &str, with_hook: bool) -> Result<Collection, Box<dyn Error>> {
    let input = [input];
    let environment = Environment {
        is_subtype: with_hook.then_some(&is_subtype as &dyn Fn(&str, &str) -> bool),
        ..Environment::default()
    };
    Ok(evaluate_parsed_with_environment(
        &parse(expression)?,
        &input,
        &|base, suffix| registry().is_choice_variant(base, suffix),
        &environment,
    )?)
}

fn strings(result: &Collection) -> Vec<String> {
    result
        .iter()
        .map(|v| v.as_string().map(|s| s.to_string()).unwrap_or_default())
        .collect()
}

#[test]
fn of_type_canonical_finds_the_canonical_and_not_a_string() -> TestResult {
    let result = eval("Questionnaire.descendants().ofType(canonical)", true)?;
    assert_eq!(strings(&result), ["http://example.org/base"]);
    Ok(())
}

#[test]
fn of_type_uri_finds_the_uri_and_its_subtypes() -> TestResult {
    let result = eval("Questionnaire.descendants().ofType(uri)", true)?;
    // url (uri), derivedFrom (canonical) and the extension's url (uri).
    assert_eq!(
        strings(&result),
        [
            "http://example.org/q",
            "http://example.org/base",
            "http://example.org/x",
        ]
    );
    Ok(())
}

#[test]
fn a_code_is_a_string() -> TestResult {
    assert_eq!(
        eval("Questionnaire.status.is(string)", true)?,
        [Value::Bool(true)]
    );
    assert_eq!(
        eval("Questionnaire.status is code", true)?,
        [Value::Bool(true)]
    );
    assert_eq!(
        strings(&eval("Questionnaire.status.as(code)", true)?),
        ["draft"]
    );
    Ok(())
}

#[test]
fn a_string_is_not_a_code() -> TestResult {
    assert_eq!(
        eval("Questionnaire.title.is(code)", true)?,
        [Value::Bool(false)]
    );
    assert_eq!(eval("Questionnaire.title as code", true)?, []);
    assert_eq!(
        eval("Questionnaire.url.is(url)", true)?,
        [Value::Bool(false)]
    );
    Ok(())
}

#[test]
fn the_resource_id_carries_its_fhir_type() -> TestResult {
    assert_eq!(eval("Questionnaire.id.is(id)", true)?, [Value::Bool(true)]);
    Ok(())
}

#[test]
fn a_primitive_without_a_value_keeps_its_fhir_type() -> TestResult {
    assert_eq!(
        eval("Questionnaire.version.is(string)", true)?,
        [Value::Bool(true)]
    );
    assert_eq!(
        eval("Questionnaire.version.is(code)", true)?,
        [Value::Bool(false)]
    );
    Ok(())
}

#[test]
fn system_type_names_keep_the_system_rules_on_typed_values() -> TestResult {
    // N1 §10.1: `String` is no FHIR type, so it names System.String, which
    // FHIR data never is.
    assert_eq!(
        eval("Questionnaire.status.is(String)", true)?,
        [Value::Bool(false)]
    );
    assert_eq!(eval("'draft'.is(String)", true)?, [Value::Bool(true)]);
    // Every value, FHIR data included, is a System.Any.
    assert_eq!(
        eval("Questionnaire.status.is(Any)", true)?,
        [Value::Bool(true)]
    );
    assert_eq!(eval("'draft'.is(Any)", true)?, [Value::Bool(true)]);
    assert_eq!(eval("Questionnaire.status.ofType(integer)", true)?, []);
    // A capitalised name that is neither FHIR nor System stays an error.
    assert!(eval("Questionnaire.status.is(Foo)", true).is_err());
    Ok(())
}

#[test]
fn type_reflects_fhir_data_in_the_fhir_namespace() -> TestResult {
    for (expression, expected) in [
        ("Questionnaire.status.type().namespace", "FHIR"),
        ("Questionnaire.status.type().name", "code"),
        ("Questionnaire.type().namespace", "FHIR"),
        ("Questionnaire.type().name", "Questionnaire"),
        ("'Patient/1'.resolve().type().namespace", "FHIR"),
        ("'Patient/1'.resolve().type().name", "Patient"),
        ("'draft'.type().namespace", "System"),
        ("'draft'.type().name", "String"),
    ] {
        assert_eq!(
            strings(&eval(expression, true)?),
            [expected],
            "{expression}"
        );
    }
    Ok(())
}

#[test]
fn type_of_a_complex_datatype_is_its_fhir_type() -> TestResult {
    // N1 §10.2: FHIR data answers in the FHIR namespace, objects included.
    let name = eval("Questionnaire.version.extension.type().name", true)?;
    assert_eq!(strings(&name), ["Extension"]);
    let namespace = eval("Questionnaire.version.extension.type().namespace", true)?;
    assert_eq!(strings(&namespace), ["FHIR"]);
    Ok(())
}

#[test]
fn type_of_an_untyped_object_is_an_error() -> TestResult {
    // Without the model, a complex datatype object has no known type.
    let untyped = fhir_fhirpath::resource_to_value(&parse_json(
        r#"{"resourceType":"Patient","name":[{"family":"Doe"}]}"#,
    )?);
    assert!(eval_on(untyped, "Patient.name.type()", false).is_err());
    Ok(())
}

#[test]
fn type_of_an_untyped_primitive_is_an_error() -> TestResult {
    // Without the model, `_version` is a primitive whose FHIR type is unknown.
    let untyped = fhir_fhirpath::resource_to_value(&parse_json(
        r#"{"resourceType":"Questionnaire",
        "_version":{"extension":[{"url":"http://example.org/x","valueBoolean":true}]}}"#,
    )?);
    let exists = eval_on(untyped.clone(), "Questionnaire.version.exists()", false)?;
    assert_eq!(exists, [Value::Bool(true)]);
    assert!(eval_on(untyped, "Questionnaire.version.type()", false).is_err());
    Ok(())
}

#[test]
fn an_xml_primitive_with_extensions_keeps_its_fhir_type() -> TestResult {
    let resource = parse_xml(
        r#"<Questionnaire xmlns="http://hl7.org/fhir"><status value="draft">
        <extension url="http://example.org/x"><valueBoolean value="true"/></extension>
        </status></Questionnaire>"#,
    )?;
    let result = eval_on(typed(&resource), "Questionnaire.status.is(code)", true)?;
    assert_eq!(result, [Value::Bool(true)]);
    Ok(())
}

#[test]
fn a_literal_keeps_the_system_type_rules() -> TestResult {
    assert_eq!(eval("'draft'.is(String)", true)?, [Value::Bool(true)]);
    assert!(eval("'draft'.is(code)", true).is_err());
    Ok(())
}

#[test]
fn without_the_hook_a_fhir_string_subtype_is_an_error() -> TestResult {
    for expression in [
        "Questionnaire.status.is(code)",
        "Questionnaire.title.is(code)",
        "Questionnaire.descendants().ofType(canonical)",
    ] {
        let error = eval(expression, false).err();
        let error = error.as_deref().and_then(|e| e.downcast_ref::<EvalError>());
        assert!(
            matches!(error, Some(EvalError::Type(_))),
            "{expression}: {error:?}"
        );
    }
    Ok(())
}

#[test]
fn an_unknown_type_name_is_an_error() -> TestResult {
    // testFHIRPathAsFunction23/24: `string1` is neither a FHIR nor a System type.
    for expression in [
        "Questionnaire.status.as(string1)",
        "Questionnaire.status.ofType(string1)",
        "Questionnaire.status.is(string1)",
        "Questionnaire.status is string1",
        "Questionnaire as Questionnaire1",
        // The name is checked even when the input is empty.
        "{}.as(string1)",
        "{}.ofType(string1)",
        "{} is string1",
    ] {
        let error = eval(expression, true).err();
        let error = error.as_deref().and_then(|e| e.downcast_ref::<EvalError>());
        assert!(
            matches!(error, Some(EvalError::Type(_))),
            "{expression}: {error:?}"
        );
    }
    // Known types still answer: a resource type and a FHIR complex type.
    assert_eq!(
        eval("Questionnaire.is(Questionnaire)", true)?,
        [Value::Bool(true)]
    );
    assert_eq!(eval("Questionnaire.status.ofType(HumanName)", true)?, []);
    assert!(eval("Questionnaire.status.is(Any)", true).is_ok());
    Ok(())
}

/// fhirpath.html, type specifiers: `System.X` names a FHIRPath type, `FHIR.X`
/// a FHIR type; the function form takes the same qualified names as the
/// operator form, and a FHIR value is not a System value (nor the reverse).
#[test]
fn qualified_type_names_check_their_namespace() -> TestResult {
    for (expression, expected) in [
        ("Questionnaire.status.is(FHIR.code)", true),
        ("Questionnaire.status is FHIR.string", true),
        ("Questionnaire.status.is(FHIR.`code`)", true),
        ("Questionnaire.status.is(System.String)", false),
        ("Questionnaire.is(FHIR.Questionnaire)", true),
        ("Questionnaire is FHIR.Questionnaire", true),
        ("Questionnaire.is(System.Questionnaire)", false),
        ("'a'.is(System.String)", true),
        ("'a' is System.String", true),
        ("'a'.is(FHIR.string)", false),
        ("'a'.is(System.string)", false),
        ("1.is(System.Integer)", true),
        ("1.is(System.Quantity)", false),
        ("true is System.Boolean", true),
        ("Questionnaire.status.is(System.Any)", true),
        ("1 is System.Any", true),
        // Known FHIR types that the value does not have.
        ("Questionnaire.status.is(FHIR.uri)", false),
        ("Questionnaire.is(FHIR.Patient)", false),
        // A resource is each of its ancestor types (Registry inheritance).
        ("Questionnaire.is(FHIR.DomainResource)", true),
        ("Questionnaire is FHIR.Resource", true),
        ("Questionnaire.is(DomainResource)", true),
    ] {
        assert_eq!(
            eval(expression, true)?,
            [Value::Bool(expected)],
            "{expression}"
        );
    }
    assert_eq!(
        strings(&eval("Questionnaire.ofType(FHIR.Questionnaire).id", true)?),
        ["q1"]
    );
    assert_eq!(
        strings(&eval("Questionnaire.status.as(FHIR.code)", true)?),
        ["draft"]
    );
    assert_eq!(eval("Questionnaire.status.as(System.String)", true)?, []);
    assert_eq!(eval("Questionnaire.status.as(FHIR.uri)", true)?, []);
    assert_eq!(eval("Questionnaire.ofType(FHIR.Patient)", true)?, []);
    // JSON booleans and numbers keep their FHIR type too, without extensions.
    let questionnaire = typed(&parse_json(
        r#"{"resourceType":"Questionnaire","status":"draft","experimental":true,
        "item":[{"linkId":"1","type":"string","maxLength":5}]}"#,
    )?);
    for (expression, expected) in [
        ("Questionnaire.experimental.is(FHIR.boolean)", true),
        ("Questionnaire.experimental.is(System.Boolean)", false),
        ("Questionnaire.item.maxLength.is(FHIR.integer)", true),
        ("Questionnaire.item.maxLength.is(System.Integer)", false),
    ] {
        assert_eq!(
            eval_on(questionnaire.clone(), expression, true)?,
            [Value::Bool(expected)],
            "{expression}"
        );
    }
    // An unknown FHIR type is an error, like an unknown unqualified name.
    let error = eval("Questionnaire.is(FHIR.NoSuchType)", true).err();
    let error = error.as_deref().and_then(|e| e.downcast_ref::<EvalError>());
    assert!(matches!(error, Some(EvalError::Type(_))), "{error:?}");
    Ok(())
}

fn observation() -> Result<Value, Box<dyn Error>> {
    Ok(typed(&parse_json(
        r#"{"resourceType":"Observation","status":"final","code":{"text":"weight"},
        "valueQuantity":{"value":185,"unit":"lbs","system":"http://unitsofmeasure.org",
          "code":"[lb_av]"},
        "extension":[{"url":"http://example.org/age","valueAge":{"value":40,"unit":"yr",
          "system":"http://unitsofmeasure.org","code":"a"}}],
        "component":[{"code":{"text":"x"},"valueString":"y"}],
        "contained":[{"resourceType":"Patient","id":"p","name":[{"family":"Doe"}]}]}"#,
    )?))
}

#[test]
fn a_complex_datatype_is_its_fhir_type_and_its_ancestors() -> TestResult {
    // N1 §6.3: `is` / `as` / `ofType` match the type or a subtype of it;
    // R5 fhirpath.html §2.1.9.1.2: `Observation.value.ofType(Quantity)`.
    for (expression, expected) in [
        ("Observation.value is Quantity", true),
        ("Observation.value.is(Quantity)", true),
        ("Observation.value is Element", true),
        ("Observation.value is Age", false),
        ("Observation.value is Period", false),
        ("Observation.extension.value is Age", true),
        ("Observation.extension.value is Quantity", true),
        ("Observation.code is CodeableConcept", true),
        ("Observation.component.is(BackboneElement)", true),
        ("Observation.contained.name.is(HumanName)", true),
        ("Observation.contained.is(Patient)", true),
        ("Observation.contained.is(DomainResource)", true),
    ] {
        assert_eq!(
            eval_on(observation()?, expression, true)?,
            [Value::Bool(expected)],
            "{expression}"
        );
    }
    Ok(())
}

#[test]
fn as_and_of_type_keep_a_matching_complex_datatype() -> TestResult {
    for (expression, expected) in [
        ("Observation.value.as(Quantity).unit", vec!["lbs"]),
        ("(Observation.value as Quantity).unit", vec!["lbs"]),
        ("Observation.value.ofType(Quantity).unit", vec!["lbs"]),
        ("Observation.value.as(Period).start", vec![]),
        (
            "Observation.contained.name.ofType(HumanName).family",
            vec!["Doe"],
        ),
    ] {
        assert_eq!(
            strings(&eval_on(observation()?, expression, true)?),
            expected,
            "{expression}"
        );
    }
    Ok(())
}

#[test]
fn a_typed_quantity_still_compares_as_a_quantity() -> TestResult {
    let result = eval_on(observation()?, "Observation.value = 185 '[lb_av]'", true)?;
    assert_eq!(result, [Value::Bool(true)]);
    Ok(())
}

#[test]
fn an_expression_may_start_with_a_supertype_of_the_context() -> TestResult {
    // N1 §3: a leading type name resolves to the context's type or a supertype.
    let patient = typed(&parse_json(
        r#"{"resourceType":"Patient","id":"p1","text":{"status":"generated",
        "div":"<div xmlns=\"http://www.w3.org/1999/xhtml\">x</div>"}}"#,
    )?);
    for (expression, expected) in [
        ("DomainResource.text.status", vec!["generated"]),
        ("Resource.id", vec!["p1"]),
        ("Observation.id", vec![]),
    ] {
        assert_eq!(
            strings(&eval_on(patient.clone(), expression, true)?),
            expected,
            "{expression}"
        );
    }
    Ok(())
}
