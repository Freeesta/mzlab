"""tools/moduli/*.toml against the cap literals still in the build scripts and workflows: they must be equal, module by module (PR 1 of the modules data)."""
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import moduli  # noqa: E402

# module -> `limit*` variable of tools/build_feat_wasm.sh (the standard `limit` for the rest)
VAR = {"hr": "limit_hr", "untargeted": "limit_untargeted", "interp": "limit_interp", "table": "limit_table", "chem": "limit_chem",
       "msngrafo": "limit_msngrafo", "msnregole": "limit_msnregole", "rotte": "limit_rotte", "llm": "limit_llm",
       "attributio-all": "limit_attributio", "finder-lr": "limit_lr"}
WORKFLOWS = ("sito-rust", "pages", "release", "notte")


def test_dati_completi():
    assert moduli.errori(moduli.carica()) == []
    r = subprocess.run([sys.executable, str(ROOT / "tools/moduli.py"), "--controlla"], capture_output=True, text=True)
    assert r.returncode == 0, r.stdout


def test_tetti_come_gli_script():
    mod = moduli.carica()
    feat = (ROOT / "tools/build_feat_wasm.sh").read_text(encoding="utf-8")
    lett = (ROOT / "tools/build_lettori_wasm.sh").read_text(encoding="utf-8")
    var = dict(re.findall(r"^(limit\w*)=(\d+)", feat, re.M))
    tab = {f: int(v) for f, v in re.findall(r"^(?:tetti=\")?(\w+) (\d+)\"?$", re.search(r'^tetti="(.*?)"', lett, re.M | re.S).group(0), re.M)}
    confrontati = 0
    for n, r in mod.items():
        if r["parte"] in ("a", "b"):
            atteso = int(var[VAR.get(n, "limit")])
        elif r["parte"] == "lettori":
            atteso = tab[n.removeprefix("lettori-")]
        else:
            continue
        assert r["tetto"] == atteso, f"{n}: data {r['tetto']} != script {atteso}"
        confrontati += 1
    assert confrontati == 29
    assert len(tab) == 9


def test_tetti_come_i_workflow():
    mod = moduli.carica()
    for wf in WORKFLOWS:
        testo = (ROOT / f".github/workflows/{wf}.yml").read_text(encoding="utf-8")
        for nome, etichetta in (("motore", "engine"), ("librerie", "libraries")):
            cap = re.findall(rf"tools/wasm_opt\.sh \S+ (\d+) {etichetta} @motore", testo)
            assert cap and all(int(c) == mod[nome]["tetto"] for c in cap), f"{wf}: {nome} {cap} != {mod[nome]['tetto']}"
