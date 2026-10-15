"""Golden data of the Attributio port: present and consistent with livelli_attesi.csv."""
import csv
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ORO = ROOT / "tests/golden/attributio/sintetica"
ATTESI = ROOT / "tests/data/attributio/campagna_sintetica/livelli_attesi.csv"
STADI = ["campagna", "letture", "rilevazione", "livelli", "identita", "proposte", "decisioni", "abbinamenti", "spiegazioni", "frammenti"]


def test_stadi_presenti_e_piccoli():
    for s in STADI:
        p = ORO / f"{s}.json"
        assert p.exists(), p
        assert p.stat().st_size < 100_000
        json.loads(p.read_text(encoding="utf-8"))
    assert sum(p.stat().st_size for p in ORO.glob("*.json")) < 300_000


def test_livelli_coincidono_con_attesi():
    got = json.loads((ORO / "livelli.json").read_text(encoding="utf-8"))["features"]
    with open(ATTESI, encoding="utf-8", newline="") as fh:
        att = list(csv.DictReader(fh, delimiter=";"))
    assert len(got) == len(att) == 15
    for g, a in zip(got, att):
        assert (g["_pol"], str(g["_id"])) == (a["_pol"], a["_id"])
        for k in ("level", "level_sub", "level_basis", "name"):
            assert ("" if g[k] is None else str(g[k])) == a[k], (a["_pol"], a["_id"], k)
