import json
from pathlib import Path

import fhir_linter
import pytest

EXTRA = Path(__file__).parent / "fixtures" / "extra"
PROFILE = EXTRA / "patient-with-birthdate.json"
PROFILED_PATIENT = json.dumps(
    {
        "resourceType": "Patient",
        "id": "p1",
        "meta": {"profile": [json.loads(PROFILE.read_text())["url"]]},
    }
)


def codes(diagnostics):
    return {(d.code, d.path) for d in diagnostics}


def test_unknown_profile_without_extra_definitions():
    found = codes(fhir_linter.validate(PROFILED_PATIENT))
    assert ("PROFILE_001", "Patient.meta.profile[0]") in found
    assert ("CARDINALITY_001", "Patient.birthDate") not in found


@pytest.mark.parametrize("path", [PROFILE, EXTRA, str(PROFILE)])
def test_validate_enforces_supplied_profile(path):
    found = codes(fhir_linter.validate(PROFILED_PATIENT, extra_definitions=[path]))
    assert ("CARDINALITY_001", "Patient.birthDate") in found
    assert ("PROFILE_001", "Patient.meta.profile[0]") not in found


def test_lint_accepts_supplied_profile():
    # Lint rules read core types from the base; supplied definitions never reach them.
    plain = fhir_linter.lint(PROFILED_PATIENT)
    extra = fhir_linter.lint(PROFILED_PATIENT, extra_definitions=[PROFILE])
    assert codes(extra) == codes(plain)


def test_empty_extra_definitions_change_nothing():
    plain = fhir_linter.validate(PROFILED_PATIENT)
    empty = fhir_linter.validate(PROFILED_PATIENT, extra_definitions=[])
    assert codes(plain) == codes(empty)


def test_bare_string_raises_type_error():
    with pytest.raises(TypeError):
        fhir_linter.validate(PROFILED_PATIENT, extra_definitions=str(PROFILE))


@pytest.mark.parametrize("function", [fhir_linter.validate, fhir_linter.lint])
def test_malformed_extra_definition_names_file_not_content(function, tmp_path):
    bad = tmp_path / "broken-profile.json"
    bad.write_text('{"resourceType": "SECRET-CONTENT", ')
    with pytest.raises(RuntimeError, match="cannot load extra definitions") as error:
        function(PROFILED_PATIENT, extra_definitions=[bad])
    assert "broken-profile.json" in str(error.value)
    assert "SECRET-CONTENT" not in str(error.value)


def test_conflicting_extra_definition_raises(tmp_path):
    conflict = tmp_path / "conflict.json"
    profile = json.loads(PROFILE.read_text())
    base_url = "http://hl7.org/fhir/StructureDefinition/Patient"
    conflict.write_text(json.dumps({**profile, "url": base_url}))
    with pytest.raises(RuntimeError, match="already defined") as error:
        fhir_linter.validate(PROFILED_PATIENT, extra_definitions=[conflict])
    assert "conflict.json" in str(error.value)


def test_lint_ignores_supplied_core_url_for_unknown_type(tmp_path):
    # An unknown type has no core definition; a supplied one adds no lint rules.
    unknown = json.dumps({"resourceType": "Zzz", "id": "z1"})
    profile = json.loads(PROFILE.read_text())
    zzz = tmp_path / "zzz.json"
    zzz.write_text(
        json.dumps(
            {
                **profile,
                "url": "http://hl7.org/fhir/StructureDefinition/Zzz",
                "type": "Zzz",
                "snapshot": {
                    "element": [
                        {"id": "Zzz", "path": "Zzz", "min": 0, "max": "*"},
                        {"id": "Zzz.text", "path": "Zzz.text", "min": 0, "max": "1"},
                    ]
                },
            }
        )
    )
    assert codes(fhir_linter.lint(unknown, extra_definitions=[zzz])) == codes(
        fhir_linter.lint(unknown)
    )


def test_missing_extra_definition_names_path(tmp_path):
    missing = tmp_path / "does-not-exist.json"
    with pytest.raises(RuntimeError, match="does-not-exist.json"):
        fhir_linter.validate(PROFILED_PATIENT, extra_definitions=[missing])
