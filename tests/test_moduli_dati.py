"""tools/moduli/*.toml against the cap literals still in the build scripts and workflows: they must be equal, module by module (the build scripts read the data; the workflow lines still carry the engine and libraries caps)."""
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import moduli  # noqa: E402

WORKFLOWS = ("sito-rust", "pages", "release", "notte")


def test_dati_completi():
    assert moduli.errori(moduli.carica()) == []
    r = subprocess.run([sys.executable, str(ROOT / "tools/moduli.py"), "--controlla"], capture_output=True, text=True)
    assert r.returncode == 0, r.stdout


def test_nessun_tetto_negli_script():
    """The caps are in the data only: no `limit*` variable or `tetti` table is left in the build scripts."""
    for sh in ("build_feat_wasm.sh", "build_lettori_wasm.sh"):
        testo = (ROOT / "tools" / sh).read_text(encoding="utf-8")
        assert not re.search(r"^\s*(limit\w*=|tetti=)|\b\d{6,7}\b", testo, re.M), sh


def test_ogni_modulo_costruito_da_uno_script():
    """Every module of part a, b or lettori has a package that exists in the workspace; the manifest keys are unique across the data."""
    mod = moduli.carica()
    for n, r in mod.items():
        if r["parte"] != "core":
            assert (ROOT / "crates" / r["pacchetto"] / "Cargo.toml").is_file(), n
    chiavi = [k for r in mod.values() for k in r.get("chiavi", {})]
    assert len(chiavi) == len(set(chiavi)) == 27
    assert sorted(p for r in mod.values() for p in r.get("chiavi", {}).values()) == list(range(1, 28))


def test_tetti_come_i_workflow():
    mod = moduli.carica()
    for wf in WORKFLOWS:
        testo = (ROOT / f".github/workflows/{wf}.yml").read_text(encoding="utf-8")
        for nome, etichetta in (("motore", "engine"), ("librerie", "libraries")):
            cap = re.findall(rf"tools/wasm_opt\.sh \S+ (\d+) {etichetta} @motore", testo)
            assert cap and all(int(c) == mod[nome]["tetto"] for c in cap), f"{wf}: {nome} {cap} != {mod[nome]['tetto']}"
