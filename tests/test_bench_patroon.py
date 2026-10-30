"""patRoon in the benchmark: an invented patRoon CSV -> `bench/1` (hits, levels) -> identification endpoints with known values; the runner skips without Docker."""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT)); sys.path.insert(0, str(ROOT / "tools"))
from bench import FORMATO  # noqa: E402
from bench import adatta as A  # noqa: E402
from bench import corri_patroon as P  # noqa: E402
from bench import metriche as M  # noqa: E402
from bench import rapporto as R  # noqa: E402
from bench import versioni  # noqa: E402


def comp(i, mz, rt, ruolo):
    return {"id": i, "mz": mz, "rt": rt, "ruolo": ruolo, "fascia": None, "atteso": None, "area_attesa": [100.0, 100.0] if ruolo == "progenitore" else None}


VERITA = [comp("par", 300.1000, 5.0, "progenitore"), comp("a", 316.1000, 4.0, "tp1"), comp("b", 330.2000, 6.0, "tp1"), comp("e1", 500.5000, None, "esca")]
STRUTTURE = {"par": "Parent", "a": "ProdA", "b": "ProdB"}


def scrivi(tmp_path):
    (tmp_path / "fg.csv").write_text(
        "group,ret,mz,S1,S2\n"
        "M300_R300,300,300.1001,90,110\n"                 # parent, one hit with the true name
        "M316_R240,240,316.1002,50,70\n"                  # a: two hits, the true one is second
        "M330_R360,360,330.2001,10,10\n"                  # b: found, no hit
        "M500_R120,120,500.5001,5,5\n"                    # on the decoy, with a hit
        "M700_R100,100,700.7000,1,1\n")
    (tmp_path / "su.csv").write_text(
        "group,name,estIDLevel,score\n"
        "M300_R300,Parent,3a,0.9\n"
        "M316_R240,Other,3b,0.8\n"
        "M316_R240,ProdA,3a,0.5\n"
        "M500_R120,Decoy,5,0.4\n")
    return tmp_path / "fg.csv", tmp_path / "su.csv"


def test_patroon_csv_to_bench1_with_hits(tmp_path):
    fg, su = scrivi(tmp_path)
    t = A.da_csv("patroon", fg, ["S1", "S2"], "T", "openms", sospetti=su)
    assert t["formato"] == FORMATO and len(t["righe"]) == 5
    r = {x["gruppo"]: x for x in t["righe"]}
    assert r["M316_R240"]["rt"] == 4.0 and r["M316_R240"]["candidato"] is True and r["M316_R240"]["livello"] == "3b" and r["M316_R240"]["punteggio"] == 0.8
    assert r["M330_R360"]["candidato"] is False and r["M330_R360"]["livello"] is None
    hits = {x["riga"]: x["hits"] for x in t["identificazioni"]}
    assert [h["nome"] for h in hits[r["M316_R240"]["id"]]] == ["Other", "ProdA"] and hits[r["M316_R240"]["id"]][1]["rango"] == 2
    assert A.a_json(t) == A.a_json(A.da_csv("patroon", fg, ["S1", "S2"], "T", "openms", sospetti=su))
    assert "identificazioni" not in A.da_csv("patroon", fg, ["S1", "S2"], "T", "openms")        # without screening the bytes are the usual ones


def test_identification_endpoints_known_values(tmp_path):
    fg, su = scrivi(tmp_path)
    t = A.da_csv("patroon", fg, ["S1", "S2"], "T", "openms", sospetti=su)
    m = M.identificazione(VERITA, t, STRUTTURE)
    assert m["standard"] == ["par", "a", "b"] and m["colpo"] == [1, 1, 0] and m["rango"] == [1, 2, None] and m["livello"] == ["3a", "3a", None]
    assert m["richiamo_colpi"] == 2 / 3 and m["mrr"] == (1 + 0.5 + 0) / 3
    assert m["esche"] == ["e1"] and m["falso"] == [1] and m["falsi"] == 1.0 and m["livelli"] == {"3a": 2, "none": 1}
    e = M.endpoint_nts(VERITA, t, STRUTTURE)
    assert e["P1"]["richiamo"] == 1.0 and e["P1"]["n"] == 3 and e["ID"]["n"] == 3
    r = R.identificazione_affiancata(VERITA, [t], STRUTTURE)
    assert r["patroon/openms"]["livelli"] == {"3a": 2, "none": 1} and "| patroon/openms | 3 |" in R.identificazione_markdown(r)


def test_runner_skips_without_docker_and_refuses_unpinned(tmp_path, monkeypatch):
    mz = tmp_path / "mzml"
    mz.mkdir()
    (mz / "S1.mzML").write_text("<mzML/>")
    cmd = [sys.executable, str(P.__file__), str(mz), str(tmp_path / "out")]
    r = subprocess.run(cmd, capture_output=True, text=True, env={"PATH": str(tmp_path)})                # no docker on the path
    assert r.returncode == 0 and "SKIP" in r.stdout
    assert versioni.PATROON["digest"] is None or versioni.PATROON["digest"].startswith("sha256:")
    c = P.comando("img@sha256:x", mz, tmp_path, tmp_path / "p.json", None, tmp_path / "s.csv", ["S1"])
    assert c[:3] == ["docker", "run", "--rm"] and f"{mz.resolve()}:/mzml:ro" in c and "/mzml/S1.mzML" in c and c[c.index("/bench/run.R") + 3] == "-"
