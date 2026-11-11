import json
from concurrent.futures import ThreadPoolExecutor

import fhir_linter
import pytest

BARE_PATIENT = json.dumps({"resourceType": "Patient", "id": "p1"})


def test_validate_valid_resource_has_no_errors():
    diagnostics = fhir_linter.validate(BARE_PATIENT)
    assert [d for d in diagnostics if d.severity == "error"] == []


def test_validate_unknown_resource_type():
    diagnostics = fhir_linter.validate('{"resourceType": "Nope"}')
    assert [d.code for d in diagnostics] == ["STRUCTURE_000"]
    assert diagnostics[0].severity == "error"
    assert diagnostics[0].rule is None


def test_validate_reports_unknown_element():
    diagnostics = fhir_linter.validate('{"resourceType": "Patient", "bogus": 1}')
    assert any(d.path == "Patient.bogus" for d in diagnostics)


def test_lint_bare_patient_triggers_all_builtin_rules():
    diagnostics = fhir_linter.lint(BARE_PATIENT)
    assert sorted(d.rule for d in diagnostics) == [
        "missing-identifier",
        "missing-meta-profile",
        "narrative-absent",
    ]
    assert {d.severity for d in diagnostics} == {"warning"}


def test_lint_config_overrides_severity_and_disables_rules():
    config = """
[rules.narrative-absent]
severity = "error"

[rules.missing-identifier]
enabled = false
"""
    diagnostics = {d.rule: d for d in fhir_linter.lint(BARE_PATIENT, config)}
    assert "missing-identifier" not in diagnostics
    assert diagnostics["narrative-absent"].severity == "error"


@pytest.mark.parametrize(
    ("configured", "expected"),
    [("fatal", "fatal"), ("hint", "information")],
)
def test_lint_config_severity_outputs_issue_severity_code(configured, expected):
    config = f'[rules.narrative-absent]\nseverity = "{configured}"'
    diagnostics = {d.rule: d for d in fhir_linter.lint(BARE_PATIENT, config)}
    assert diagnostics["narrative-absent"].severity == expected


@pytest.mark.parametrize("function", [fhir_linter.validate, fhir_linter.lint])
def test_malformed_json_raises_value_error(function):
    with pytest.raises(ValueError, match="parse error"):
        function("{not json")


def test_lint_unknown_rule_raises_value_error():
    with pytest.raises(ValueError, match="unknown lint rule"):
        fhir_linter.lint(BARE_PATIENT, "[rules.bogus]\nenabled = false")


def test_lint_invalid_toml_raises_value_error():
    with pytest.raises(ValueError, match="invalid lint config"):
        fhir_linter.lint(BARE_PATIENT, "[rules")


def test_missing_definitions_raises_value_error(monkeypatch):
    monkeypatch.delenv("FHIR_DEFINITIONS_DIR")
    with pytest.raises(ValueError, match="no FHIR definitions"):
        fhir_linter.validate(BARE_PATIENT)


def test_unreadable_definitions_dir_raises_runtime_error(tmp_path):
    with pytest.raises(RuntimeError, match="cannot load definitions"):
        fhir_linter.validate(BARE_PATIENT, definitions_dir=str(tmp_path))


def test_equivalent_definitions_paths_give_same_result(definitions_dir, monkeypatch):
    monkeypatch.chdir(definitions_dir)
    relative = fhir_linter.validate(BARE_PATIENT, definitions_dir=".")
    absolute = fhir_linter.validate(BARE_PATIENT, definitions_dir=definitions_dir)
    assert [d.code for d in relative] == [d.code for d in absolute]


def test_concurrent_calls_from_threads():
    with ThreadPoolExecutor(max_workers=4) as pool:
        results = list(pool.map(lambda _: fhir_linter.lint(BARE_PATIENT), range(8)))
    assert all(len(diagnostics) == 3 for diagnostics in results)
