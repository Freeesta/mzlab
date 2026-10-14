"""The readers module (crates/mzlab-lettori-wasm) stays a module of its own: its size limits do not go up, and the engine (mzlab-wasm) does not name it."""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TETTO = 1048576


def test_tetti_nello_script():
    s = (ROOT / "tools/build_lettori_wasm.sh").read_text(encoding="utf-8")
    assert re.search(r"^limit=1048576\b", s, re.M)
    assert re.search(r"^# limit_tdf=655360\b", s, re.M)
    assert not re.search(r"^limit(_tdf)?=(?!1048576\b|655360\b)", s, re.M)


def test_il_motore_non_nomina_il_modulo():
    t = (ROOT / "crates/mzlab-wasm/Cargo.toml").read_text(encoding="utf-8")
    assert "mzlab-lettori" not in t and "estesi" not in t


def test_costruito_dentro_il_tetto():
    w = ROOT / "site/static/wasm-lettori/mzlab_lettori_wasm_bg.wasm"
    if w.exists():
        assert w.stat().st_size < TETTO


def test_costruzione_chiamata_da_build_feat():
    assert "tools/build_lettori_wasm.sh" in (ROOT / "tools/build_feat_wasm.sh").read_text(encoding="utf-8")
