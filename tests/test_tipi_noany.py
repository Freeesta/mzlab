"""Level 3 of the tipi ratchet: `// @ts-noany` files are checked with noImplicitAny in stretto3*.json, never in the base."""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
sys.path.insert(0, str(ROOT / "tools" / "tipi"))
import controlla_tsc as ct  # noqa: E402
import fondi_main as fm  # noqa: E402
import genera_globali as gg  # noqa: E402


def test_a_strict3_error_fails_even_when_the_base_is_silent(monkeypatch, tmp_path, capsys):
    base = tmp_path / "base.json"
    base.write_text("{}")
    monkeypatch.setattr(ct, "BASE", str(base))
    monkeypatch.setattr(ct, "misura", lambda: {"stretto3:mzlab/web/x.js": 1})
    monkeypatch.setattr(sys, "argv", ["controlla_tsc.py"])
    assert ct.main() == 1 and "stretto3:mzlab/web/x.js: 1 errors" in capsys.readouterr().out


def test_strict3_files_are_generated_and_merged_by_the_tool():
    assert set(gg.strict_all()) >= {"stretto3.json", "stretto3-worker.json"}
    assert not [f for f in fm.GENERATI if f.startswith("tools/tipi/")]   # git-ignored: nothing to merge
    assert {"stretto3", "stretto3-worker"} <= set(ct.PROGETTI)


def test_a_noany_file_is_in_strict3_only():
    marked = [p for n in ("pagina", "moduli") for p in gg.read_list(n) if gg.NOANY_HEAD in "\n".join(gg.read(p).split("\n")[:3])]
    assert marked, "no file carries // @ts-noany"
    inc = {n: json.loads(gg.strict_all()[n])["include"] for n in ("stretto.json", "stretto2.json", "stretto3.json")}
    for p in marked:
        f = "../../" + p
        assert f in inc["stretto3.json"] and f not in inc["stretto.json"] and f not in inc["stretto2.json"]
    assert json.loads(gg.strict_all()["stretto3.json"])["compilerOptions"]["noImplicitAny"] is True


def test_generated_type_files_are_deterministic_and_not_committed():
    """The type files are rebuilt before every type check: two runs give identical bytes and git tracks none of them."""
    import subprocess
    a, b = gg.generati(), gg.generati()
    assert a == b and len(a) == 8 and all(v.endswith("\n") for v in a.values())
    tracked = subprocess.run(["git", "ls-files", "tools/tipi"], cwd=ROOT, capture_output=True, text=True).stdout.split()
    assert not [f for f in tracked if f.rsplit("/", 1)[-1] in a]
