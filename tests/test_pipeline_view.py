"""View of the result of the pipeline of mzFinder (TP_Mine/js/33_pipeline_view.js) with node: the `.mzfeat` written here (MZFEAT01: magic, meta, arrays aligned to 64 bytes, section `evid`)
is read back, the identities get their cells, the filter «discarded» keeps the reason codes, the table goes to the model UNT (program «mzlab»). Synthetic molecules only."""
import json
import shutil
import struct
import subprocess
from pathlib import Path

import numpy as np
import pytest

ROOT = Path(__file__).resolve().parent.parent
JS = ROOT / "TP_Mine" / "js"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")
DT = {"f64": np.float64, "f32": np.float32, "u32": np.uint32, "u16": np.uint16, "u8": np.uint8}


def feat_bytes(arrays, rows, files, evid):
    data, directory = b"", []
    for name, dt, a in arrays + [("evid", "u8", np.frombuffer(evid, np.uint8))]:
        data += b"\0" * (-len(data) % 64)
        directory.append({"name": name, "dtype": dt, "count": int(len(a)), "offset": len(data)})
        data += np.asarray(a, DT[dt]).tobytes()
    meta = json.dumps({"format": "table", "rows": rows, "files": files, "arrays": directory}).encode()
    head = b"MZFEAT01" + struct.pack("<Q", len(meta)) + meta
    return head + b"\0" * (-len(head) % 64) + data


def proof(tipo, valore, soglia, punti, segno):
    return {"tipo": tipo, "valore": valore, "soglia": soglia, "punti": punti, "segno": segno, "fonte": "test", "versione": 1, "parametri": {}}


def series():
    mz, rt = [200.1, 250.2, 300.3], [1.0, 2.0, 3.0]
    state = [0, 2, 3, 0, 0, 0, 3, 3, 1]                        # row * 3 + file
    area = [10.0, 5.0, 0, 7.0, 8.0, 9.0, 0, 0, 4.0]
    ids = [{"id": 1, "massa_neutra": 199.1, "formula": "C9H9", "membri": [{"mz": 200.1, "rt": 1.0, "ruolo": "M"}], "contributi": [proof("massa", 1.0, 2.0, 2, "a favore"), proof("MS2", 0.5, 0.7, 3, "a favore")],
            "totale": 5, "contro": 0, "n": 1, "probabilita": 1.0, "motivi": [], "ipotesi": 3, "livello_max": "2a", "archi": []},
           {"id": 2, "massa_neutra": 249.2, "formula": "C8H9", "membri": [{"mz": 250.2, "rt": 2.0, "ruolo": "M"}], "contributi": [proof("cinetica", 1.0, 0.5, 0, "contro")],
            "totale": 0, "contro": 1, "n": 2, "probabilita": 0.5, "motivi": [{"codice": "EV_AGAINST_OPEN", "numeri": {}}], "ipotesi": None, "livello_max": "4", "archi": []},
           {"id": 3, "massa_neutra": 299.3, "formula": "C7H9", "membri": [{"mz": 300.3, "rt": 3.0, "ruolo": "M"}], "contributi": [], "totale": 0, "contro": 0, "n": 1, "probabilita": 1.0, "motivi": [], "ipotesi": None, "livello_max": "4", "archi": []}]
    ev = json.dumps({"formato": "evid/1", "identita": ids}).encode()
    return feat_bytes([("mz", "f64", mz), ("rt", "f64", rt), ("area", "f64", area), ("state", "u8", state), ("member_start", "u32", [0, 1, 2, 3])], 3, 3, ev)


def run_node(tmp_path, body):
    p = tmp_path / "s.mzfeat"
    p.write_bytes(series())
    js = f"""globalThis.UNT = require({json.dumps(str(JS / '25_untarget.js'))}); const V = require({json.dumps(str(JS / '33_pipeline_view.js'))});
const f = V.readFeat(require('fs').readFileSync({json.dumps(str(p))})); {body}"""
    out = subprocess.run(["node", "-e", js], capture_output=True, text=True, timeout=60)
    assert out.returncode == 0, out.stderr
    return json.loads(out.stdout)


def test_read_and_cells(tmp_path):
    r = run_node(tmp_path, "const ids = V.identities(f); console.log(JSON.stringify({rows: f.rows, files: f.files, n: ids.length, cells: ids[0].cells, row: ids.map(x => x.row), chips: ids[0].chips.map(c => c.type + c.points)}));")
    assert (r["rows"], r["files"], r["n"]) == (3, 3, 3) and r["row"] == [0, 1, 2]
    assert [c["state"] for c in r["cells"]] == [0, 2, 3] and r["cells"][0]["area"] == 10 and r["chips"] == ["massa2", "MS23"]


def test_discarded_filter_with_reason(tmp_path):
    r = run_node(tmp_path, """const ids = V.identities(f), keep = V.filter(ids, {}), gone = V.filter(ids, {discarded: true}), q = V.filter(ids, {text: 'c9'});
console.log(JSON.stringify({keep: keep.map(x => x.id), gone: gone.map(x => [x.id, x.reasons]), q: q.map(x => x.id)}));""")
    assert r["keep"] == [1] and r["gone"] == [[2, ["EV_AGAINST_OPEN"]], [3, ["EV_NO_PROOF"]]] and r["q"] == [1]


def test_table_enters_unt(tmp_path):
    r = run_node(tmp_path, "const t = V.toUnt(f, ['a', 'b', 'c'], 1); console.log(JSON.stringify({p: t.program, n: t.features.length, a: t.features[0].areas, d: t.features[0].detected, k: t.features[2].key, plain: V.plainRows(f).length}));")
    assert r["p"] == "mzlab" and r["n"] == 3 and r["a"] == {"a": 10.0, "b": 5.0} and r["d"] == ["a"] and r["k"] == "POS:3" and r["plain"] == 3


def test_not_a_mzfeat(tmp_path):
    p = subprocess.run(["node", "-e", f"const V = require({json.dumps(str(JS / '33_pipeline_view.js'))}); try {{ V.readFeat(new Uint8Array(40)); }} catch (e) {{ console.log(e.key); }}"], capture_output=True, text=True)
    assert p.stdout.strip() == "error.badMzfeat"
