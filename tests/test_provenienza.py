"""Provenance of the exported tables (provenienza/1): every export of the program that leaves a table passes its provenance
(`table:` in the options of dlx()/dl()/PROV.tsv()), and the Rust engine is the only place that writes the line (mzlab-core::provenance,
unit tests there; the files really downloaded are proved by tests_e2e/e2e_provenienza.py)."""
import re
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
# the tabular exports of the program: file -> number of calls that must carry the provenance
DLX = {"banco.js": 1, "pannelli.js": 1, "bianco.js": 1, "pca.js": 1, "kmd.js": 1, "pfas.js": 1, "profilo.js": 1, "origine.js": 1, "picchi.js": 1,
       "composizione.js": 2, "libreria.js": 3, "riferimento.js": 1, "mappa-tabella.js": 1}
DL = {"riferimento.js": 1, "liste.js": 1, "campioni.js": 1}                       # CSV / TSV files
CLIP = {"banco.js": 1, "mappa-tabella.js": 1, "tdati.js": 1, "libreria.js": 2, "campioni.js": 1, "profilo.js": 1}      # tables copied to the clipboard


def call(src, i):
    """text of the call whose '(' is at src[i] (strings and regexes of these call sites do not unbalance the parentheses)"""
    d = 0
    for j in range(i, len(src)):
        d += (src[j] == "(") - (src[j] == ")")
        if d == 0:
            return src[i:j + 1]
    raise AssertionError("unbalanced")


def calls(name, head):
    src = (WEB / name).read_text(encoding="utf-8")
    return [call(src, m.end() - 1) for m in re.finditer(head + r"\(", src) if not src[:m.start()].endswith("function ")]


def test_every_dlx_carries_the_provenance():
    for name, n in DLX.items():
        c = calls(name, r"\bdlx")
        assert len(c) == n and all(re.search(r"\{ (p(: \w+)?, )?table: \"[\w-]+\"", x) for x in c), (name, [x[-80:] for x in c])
    # no export outside the list
    for f in WEB.glob("*.js"):
        if f.name not in DLX and f.name not in ("xlsx.js", "provenienza.js"):
            assert not re.search(r"\bdlx\(", f.read_text(encoding="utf-8")), f.name


def test_every_tabular_dl_carries_the_provenance():
    for name, n in DL.items():
        c = [x for x in calls(name, r"\bdl") if "tab-separated" in x or "text/csv" in x]
        assert len(c) == n and all("table:" in x for x in c), (name, c)


def test_clipboard_copies_use_the_same_helper():
    for name, n in CLIP.items():
        assert (WEB / name).read_text(encoding="utf-8").count("PROV.tsv(") >= n, name


def test_one_helper_and_the_engine_writes_the_line():
    assert "api/provenance" in (WEB / "provenienza.js").read_text(encoding="utf-8")
    rust = (WEB.parent.parent / "crates" / "mzlab-core" / "src" / "provenance.rs").read_text(encoding="utf-8")
    assert '"provenienza/1"' in rust and "# mzLab {version}" in rust
    # the schema name lives in the engine and in the helper only: a second copy would be a second schema
    for f in WEB.glob("*.js"):
        if f.name not in ("provenienza.js", "xlsx.js", "explore.js", "lr-map.js"):
            assert "provenienza/1" not in f.read_text(encoding="utf-8"), f.name


def test_own_imports_skip_the_provenance_line():
    for name in ("liste.js", "campioni.js"):
        assert 'startsWith("#")' in (WEB / name).read_text(encoding="utf-8"), name
