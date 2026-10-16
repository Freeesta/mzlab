"""Tabui scripts that are not needed at the start live in TP_Mine/lazy/ (loaded through TPLAZY.load), never in TP_Mine/js/ (loaded at every start of Tabui)."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LAZY = ["reattivita.js", "viewer3d.js"]       # panel of reactivity of the parent and its 3D viewer (docs/agenti/reattivita.md)


def test_lazy_scripts_are_not_in_startup_folder():
    for n in LAZY:
        assert (ROOT / "TP_Mine" / "lazy" / n).is_file(), n
    startup = {f.name for f in (ROOT / "TP_Mine" / "js").glob("*.js")}
    assert not startup & set(LAZY) and not any(n.endswith(("reattivita.js", "viewer3d.js")) for n in startup), startup


def test_loader_and_builder_know_lazy():
    assert "TPLAZY" in (ROOT / "mzlab/web/tpmine-loader.js").read_text(encoding="utf-8")
    assert '"lazy"' in (ROOT / "tools/build_site.py").read_text(encoding="utf-8")
