"""Old and new names of the golden/guard variables both trigger the rewrite path; `--tutto` is an alias of `--mostra-ok`."""
import importlib
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tests_e2e"))


@pytest.mark.parametrize("name", ["MZLAB_UPDATE_GOLDEN", "MZLAB_REGEN_GOLDEN"])
def test_golden_ref_reads_both_names(monkeypatch, name):
    for n in ("MZLAB_UPDATE_GOLDEN", "MZLAB_REGEN_GOLDEN"):
        monkeypatch.delenv(n, raising=False)
    monkeypatch.setenv(name, "1")
    import golden_ref
    assert importlib.reload(golden_ref).REGEN is True
    monkeypatch.delenv(name)
    assert importlib.reload(golden_ref).REGEN is False


def test_guards_reader_text():
    src = (ROOT / "tests" / "test_guardie_memoria.py").read_text(encoding="utf-8")
    assert 'os.environ.get("MZLAB_UPDATE_GUARDS") or os.environ.get("MZLAB_GUARDIE_CREA")' in src


@pytest.mark.parametrize("f,new,old", [
    ("crates/mzlab-feat/tests/famalign.rs", "MZLAB_UPDATE_GOLDEN", "MZLAB_GOLDEN_WRITE"),
    ("crates/mzlab-feat/tests/network_golden.rs", "MZLAB_UPDATE_GOLDEN", "MZLAB_WRITE_GOLDEN"),
    ("crates/mzlab-feat/tests/rete.rs", "MZLAB_UPDATE_FIXTURE", "MZLAB_WRITE_FIXTURE"),
    ("crates/mzlab-desktop/examples/serve.rs", "MZLAB_APP_DIR", "MZLAB_DATA"),
])
def test_rust_readers_check_new_then_old(f, new, old):
    # rustfmt may wrap the chain over several lines: compare without whitespace
    src = "".join((ROOT / f).read_text(encoding="utf-8").split())
    assert "".join(f'var_os("{new}").or_else(|| std::env::var_os("{old}"))'.split()) in src


def test_mostra_ok_alias():
    out = subprocess.run([sys.executable, str(ROOT / "tools" / "verifica.py"), "--help"], capture_output=True, text=True, cwd=ROOT).stdout
    assert "--mostra-ok, --tutto" in out
