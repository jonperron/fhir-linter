import os
from pathlib import Path

import pytest

REPOSITORY_ROOT = Path(__file__).resolve().parents[3]


@pytest.fixture(autouse=True)
def definitions_dir(monkeypatch):
    path = os.environ.get("FHIR_DEFINITIONS_DIR") or REPOSITORY_ROOT / "definitions/r5"
    monkeypatch.setenv("FHIR_DEFINITIONS_DIR", str(path))
    return str(path)
