"""tools/ci_pytest_lenti.py: the slow-file list comes from the mean time of the last runs, slowest first, with a threshold."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import ci_pytest_lenti as c  # noqa: E402


def run(ts, **file):
    return {"tipo": "test_durate", "python": "3.11", "ts_utc": ts, "fasi": {f: [s, v, 0] for f, (s, v) in file.items()}}


def test_ordine_per_media_e_soglia():
    ev = [run("2026-10-22T01:00:00Z", **{"tests/old.py": (0, 99)}),
          run("2026-10-22T02:00:00Z", **{"tests/a.py": (1, 9), "tests/b.py": (0, 30), "tests/c.py": (0, 2)}),
          run("2026-10-22T03:00:00Z", **{"tests/a.py": (1, 11), "tests/b.py": (0, 50), "tests/c.py": (0, 3)}),
          {"tipo": "test_durate", "python": "3.14", "ts_utc": "2026-10-22T03:00:00Z", "fasi": {"tests/x.py": [0, 500, 0]}}]
    assert c.lenti(ev, ultimi=2, minimo=5) == ["tests/b.py", "tests/a.py"]
    assert c.lenti(ev, ultimi=3, minimo=5)[:2] == ["tests/old.py", "tests/b.py"]     # the mean also counts the runs where the file did not appear


def test_senza_dati_lista_vuota():
    assert c.lenti([{"tipo": "run"}]) == []


def test_gli_shard_della_stessa_corsa_si_uniscono():
    def riga(run, parte, fasi):
        return {"tipo": "test_durate", "python": "3.11", "parte": parte, "run_id": run, "ts_utc": f"2026-10-22T0{run}:00:00Z", "fasi": fasi}

    ev = [riga(1, "rest", {"tests/a.py": [0, 6, 0]}), riga(1, "solo311", {"tests/b.py": [0, 9, 0]}),
          riga(2, "rest", {"tests/a.py": [0, 4, 0]}), riga(2, "solo311", {"tests/b.py": [0, 7, 0]})]
    assert c.lenti(ev, ultimi=8, minimo=5.0) == ["tests/b.py", "tests/a.py"]
