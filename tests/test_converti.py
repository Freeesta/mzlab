"""tools/converti.py: .raw -> ThermoRawFileParser (command and skipping what is converted), .wiff / .d -> the MSConvert command in Docker."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import converti  # noqa: E402


def test_prova_stampa_i_comandi(tmp_path):
    (tmp_path / "a.raw").write_bytes(b"x"); (tmp_path / "b.wiff").write_bytes(b"x"); (tmp_path / "b.wiff.scan").write_bytes(b"x"); (tmp_path / "c.d").mkdir()
    rows = []
    r = converti.converti(tmp_path, tmp_path / "mzML", None, True, rows.append)
    t = "\n".join(rows)
    assert "-f 2" in t and "-z" in t and "a.raw" in t
    assert "docker run" in t and "b.wiff" in t and "c.d" in t and converti.PWIZ in t and "peakPicking vendor msLevel=1-" in t
    assert len(r["da_fare"]) == 3 and not r["errori"]
    assert not (tmp_path / "mzML").exists()                      # --prova writes nothing


def test_converte_con_un_programma_finto(tmp_path, monkeypatch):
    prog = tmp_path / "trfp_finto.py"                              # a fake ThermoRawFileParser (Python, so it also runs on Windows): writes the mzML named after the input
    prog.write_text("import sys, pathlib\na = sys.argv\ni = pathlib.Path(a[a.index('-i') + 1]); o = pathlib.Path(a[a.index('-o') + 1])\n(o / (i.stem + '.mzML')).write_text('<mzML/>')\n")
    monkeypatch.setattr(converti, "trova_trfp", lambda percorso=None: [sys.executable, str(prog)])
    (tmp_path / "a.raw").write_bytes(b"x"); (tmp_path / "b.raw").write_bytes(b"x"); (tmp_path / "mzML").mkdir(); (tmp_path / "mzML" / "b.mzML").write_text("old")
    rows = []
    r = converti.converti(tmp_path, tmp_path / "mzML", None, False, rows.append)
    assert r["fatti"] == ["a.raw"] and r["saltati"] == ["b.raw"] and not r["errori"]
    assert (tmp_path / "mzML" / "a.mzML").read_text().startswith("<mzML") and (tmp_path / "mzML" / "b.mzML").read_text() == "old"


def test_senza_programma_lo_dice(tmp_path, monkeypatch):
    monkeypatch.delenv("THERMORAWFILEPARSER", raising=False); monkeypatch.setenv("PATH", str(tmp_path))
    (tmp_path / "a.raw").write_bytes(b"x")
    rows = []
    r = converti.converti(tmp_path, tmp_path / "o", None, False, rows.append)
    assert r["errori"] == ["trfp"] and "ThermoRawFileParser non trovato" in "\n".join(rows)


def test_cartella_senza_file(tmp_path):
    rows = []
    converti.converti(tmp_path, tmp_path / "o", None, False, rows.append)
    assert "Nessun file" in rows[0]
