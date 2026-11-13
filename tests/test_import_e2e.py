"""The e2e scripts import only modules that exist (a deleted helper must not stay imported)."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import check_import  # noqa: E402


def test_e2e_imports_resolved():
    assert check_import.controlla() == []


def test_the_check_finds_a_missing_module(tmp_path, monkeypatch):
    src = tmp_path / "e2e_finto.py"
    src.write_text("import os\nimport modulo_che_non_esiste_xyz\n", encoding="utf-8")
    assert check_import.imports(src) == [("os", 1), ("modulo_che_non_esiste_xyz", 2)]
    assert "modulo_che_non_esiste_xyz" not in check_import.local_names()
