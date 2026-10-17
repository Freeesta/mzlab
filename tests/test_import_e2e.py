"""The e2e scripts import only modules that exist (a deleted helper must not stay imported)."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import controlla_import  # noqa: E402


def test_import_degli_e2e_risolti():
    assert controlla_import.controlla() == []


def test_il_controllo_trova_un_modulo_sparito(tmp_path, monkeypatch):
    src = tmp_path / "e2e_finto.py"
    src.write_text("import os\nimport modulo_che_non_esiste_xyz\n", encoding="utf-8")
    assert controlla_import.imports(src) == [("os", 1), ("modulo_che_non_esiste_xyz", 2)]
    assert "modulo_che_non_esiste_xyz" not in controlla_import.local_names()
