"""tools/moduli/*.toml: complete, no cap literals in scripts or workflows, cap rule against a base, `--misura` writer."""
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import moduli  # noqa: E402

WORKFLOWS = ("sito-rust", "pages", "release", "notte")


def test_data_complete():
    assert moduli.errori(moduli.loadScript()) == []
    r = subprocess.run([sys.executable, str(ROOT / "tools/moduli.py"), "--controlla"], capture_output=True, text=True)
    assert r.returncode == 0, r.stdout


def test_no_cap_in_the_scripts():
    """The caps are in the data only: no `limit*` variable or `tetti` table is left in the build scripts."""
    for sh in ("build_feat_wasm.sh", "build_lettori_wasm.sh"):
        testo = (ROOT / "tools" / sh).read_text(encoding="utf-8")
        assert not re.search(r"^\s*(limit\w*=|tetti=)|\b\d{6,7}\b", testo, re.M), sh


def test_every_module_is_built_by_a_script():
    """Every module of part a, b or lettori has a package that exists in the workspace; the manifest keys are unique across the data."""
    mod = moduli.loadScript()
    for n, r in mod.items():
        if r["parte"] != "core":
            assert (ROOT / "crates" / r["pacchetto"] / "Cargo.toml").is_file(), n
    chiavi = [k for r in mod.values() for k in r.get("chiavi", {})]
    assert len(chiavi) == len(set(chiavi)) == 28
    assert sorted(p for r in mod.values() for p in r.get("chiavi", {}).values()) == list(range(1, 29))


def test_workflow_caps_come_from_the_data():
    """The engine and libraries steps of the workflows (and the lab copy of pages.yml) take their cap from the data: no literal, the module name exists."""
    mod = moduli.loadScript()
    for wf in [f".github/workflows/{w}.yml" for w in WORKFLOWS] + ["tools/lab/qqq-pages.yml"]:
        testo = (ROOT / wf).read_text(encoding="utf-8")
        for nome, etichetta in (("motore", "engine"), ("librerie", "libraries")):
            assert nome in mod
            assert len(re.findall(rf'tools/wasm_opt\.sh \S+ "\$\(python3 tools/moduli\.py --tetto {nome}\)" {etichetta} @motore', testo)) == 1, (wf, nome)
        assert not re.search(r"tools/wasm_opt\.sh site/static\S+ \d+ ", testo), wf


def _con(tetto):
    return {"m": {"tetto": tetto}}


def test_ten_percent_rule():
    base = _con(1000)
    assert moduli.cambi(_con(1000), base) == []
    assert moduli.cambi(_con(900), base) == ["m: 1000 -> 900 (-10.0 %)"]       # lower is allowed
    assert not moduli.cambi(_con(1100), base)[0].startswith("FAIL")             # +10 % exactly
    assert moduli.cambi(_con(1101), base)[0].startswith("FAIL")                 # above 1.10 x


def test_new_and_removed_modules_show():
    base = _con(1000)
    nuovo = moduli.cambi({**_con(1000), "nuovo": {"tetto": 5}}, base)
    assert nuovo == ["NEW nuovo: cap 5 — a new .wasm module is an owner decision"]
    assert moduli.cambi({}, base) == ["REMOVED m"]
    assert not any(x.startswith("FAIL") for x in nuovo)                         # shown, never a failure


def test_measure_equal_to_the_cap_fails():
    r = {"m": {"tetto": 100, "misura": 100}}
    assert any("misura 100" in e for e in moduli.errori(r))                     # same rule as tools/wasm_opt.sh: `>=` fails
    assert not any("misura" in e for e in moduli.errori({"m": {"tetto": 100, "misura": 99}}))


def test_base_sconosciuta_fallisce():
    r = subprocess.run([sys.executable, str(ROOT / "tools/moduli.py"), "--controlla", "--base", "no-such-ref-0000"], capture_output=True, text=True)
    assert r.returncode != 0


def test_base_head_no_change():
    r = subprocess.run([sys.executable, str(ROOT / "tools/moduli.py"), "--controlla", "--base", "HEAD"], capture_output=True, text=True)
    assert r.returncode == 0 and "0 caps changed" in r.stdout, r.stdout + r.stderr


def _finto(tmp_path, monkeypatch, byte):
    """A copy of the data and a fake built site where every module is `byte` bytes."""
    dati = tmp_path / "moduli"
    dati.mkdir(parents=True)
    for f in moduli.DIR.glob("*.toml"):
        (dati / f.name).write_bytes(f.read_bytes())
    monkeypatch.setattr(moduli, "DIR", dati)
    static = tmp_path / "static"
    for r in moduli.loadScript().values():
        w = static / r["uscita"] / f"{r['pacchetto'].replace('-', '_')}_bg.wasm"
        w.parent.mkdir(parents=True, exist_ok=True)
        w.write_bytes(b"\0" * byte)
    return dati, static


def test_measure_writes_and_a_repeat_changes_nothing(tmp_path, monkeypatch):
    dati, static = _finto(tmp_path, monkeypatch, 1234)
    assert moduli.misure(moduli.loadScript(), static) == []
    prima = {f.name: f.read_text() for f in dati.glob("*.toml")}
    assert all(r["misura"] == 1234 for r in moduli.loadScript().values())
    assert all(t.count("misura = ") == 1 for t in prima.values())
    assert moduli.misure(moduli.loadScript(), static) == []
    assert {f.name: f.read_text() for f in dati.glob("*.toml")} == prima       # the second run gives no diff
    assert moduli.errori(moduli.loadScript()) == []


def test_measure_does_not_write_if_missing_or_over(tmp_path, monkeypatch):
    dati, static = _finto(tmp_path, monkeypatch, 1234)
    prima = {f.name: f.read_text() for f in dati.glob("*.toml")}
    next(static.rglob("*.wasm")).unlink()
    assert moduli.misure(moduli.loadScript(), static)
    assert {f.name: f.read_text() for f in dati.glob("*.toml")} == prima       # nothing half-written
    dati2, static2 = _finto(tmp_path / "b", monkeypatch, 10 ** 8)
    assert any("not below the cap" in e for e in moduli.misure(moduli.loadScript(), static2))


def test_every_module_has_its_measure():
    assert [n for n, r in moduli.loadScript().items() if "misura" not in r] == []


def _mod(cap, exception=None):
    r = {"tetto": cap}
    if exception is not None:
        r["exception"] = exception
    return {"chem": r}


EXC = {"cap": 655360, "date": "2026-11-20", "reason": "owner decision 20 Nov 2026, sci-form 2.0.0"}


def test_exception_lets_exactly_its_cap_through():
    """(a) chem at 655 360 B passes with the entry; the line is printed with its reason."""
    out = moduli.cambi(_mod(655360, EXC), _mod(569344))
    assert len(out) == 1 and not out[0].startswith("FAIL") and "EXCEPTION 2026-11-20" in out[0]


def test_same_jump_without_the_entry_fails():
    """(b) the same jump without the entry, or with an entry for another cap, still fails."""
    assert moduli.cambi(_mod(655360), _mod(569344))[0].startswith("FAIL")
    assert moduli.cambi(_mod(655360, dict(EXC, cap=640000)), _mod(569344))[0].startswith("FAIL")
    assert moduli.cambi(_mod(660000, EXC), _mod(569344))[0].startswith("FAIL")


def test_other_module_over_the_rule_still_fails():
    """(c) another module over 1.10 x fails; the chem entry gives it nothing."""
    mod = {"chem": {"tetto": 655360, "exception": EXC}, "motore": {"tetto": 2_600_000}}
    base = {"chem": {"tetto": 569344}, "motore": {"tetto": 2_300_000}}
    out = moduli.cambi(mod, base)
    assert [x.startswith("FAIL") for x in out if "motore" in x] == [True]
    assert not any(x.startswith("FAIL") for x in out if x.startswith("chem"))


def test_exception_shape_is_checked():
    rec = {"uscita": "u", "pacchetto": "p", "wasm_opt": "@motore", "parte": "a", "tetto": 655360, "nota": "n"}
    assert moduli.errori({"chem": dict(rec, exception=EXC)}) == []
    assert moduli.errori({"chem": dict(rec, exception=dict(EXC, cap=1))})
    assert moduli.errori({"chem": dict(rec, exception={"cap": 655360})})
