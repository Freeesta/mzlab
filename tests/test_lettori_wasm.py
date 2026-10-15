"""The readers modules (crates/mzlab-lettori-wasm, one .wasm per family) stay modules of their own: their size limits do not go up, the engine (mzlab-wasm) does not name them,
every family has its feature, and a module that is built stays under its limit."""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TETTI = {"aperti": 393216, "andi": 262144, "thermo": 614400, "agilent": 393216, "sciex": 327680, "waters": 327680, "shimadzu": 327680, "bruker": 524288}


def tabella():
    s = (ROOT / "tools/build_lettori_wasm.sh").read_text(encoding="utf-8")
    blocco = re.search(r'^tetti="(.*?)"', s, re.M | re.S).group(1)
    return {f: int(n) for f, n in (r.split() for r in blocco.strip().splitlines())}


def test_tetti_nello_script():
    t = tabella()
    assert set(t) == set(TETTI)
    for f, n in t.items():
        assert n <= TETTI[f], (f, n)


def test_il_motore_non_nomina_i_moduli():
    t = (ROOT / "crates/mzlab-wasm/Cargo.toml").read_text(encoding="utf-8")
    assert "mzlab-lettori" not in t and "estesi" not in t


def test_ogni_famiglia_ha_la_sua_feature():
    for crate in ("mzlab-lettori", "mzlab-lettori-wasm"):
        c = (ROOT / "crates" / crate / "Cargo.toml").read_text(encoding="utf-8")
        for f in TETTI:
            assert re.search(rf"^{f} = ", c, re.M), (crate, f)


def test_costruiti_dentro_il_tetto():
    for f, n in TETTI.items():
        w = ROOT / "site/static/wasm-lettori" / f / "mzlab_lettori_wasm_bg.wasm"
        if w.exists():
            assert w.stat().st_size < n, (f, w.stat().st_size)


def test_costruzione_chiamata_da_build_feat():
    assert "tools/build_lettori_wasm.sh" in (ROOT / "tools/build_feat_wasm.sh").read_text(encoding="utf-8")
