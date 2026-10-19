"""Private validation of the identification and of the analogue search (WP-H8/H9): the MassBank library of the data repository against the DDA MS2 of the
photocatalysis series (the files and the library are NOT in this repository: the test is skipped without them). The compound is never named here: the
expected library entry is read from the data (the highest-scoring identity of the parent)."""
import json, subprocess, sys
from pathlib import Path
import pytest

ROOT = Path(__file__).resolve().parents[1]
JS = ROOT / "mzlab" / "web" / "libreria-worker.js"
sys.path.insert(0, str(ROOT / "tools"))
from dati_percorsi import percorso  # noqa: E402


def _dati():
    """The MassBank library file and the HRMS folder of the data repository, or None."""
    lib, hrms = percorso("dati/librerie/massbank_eu_qft_pos.msp"), percorso("dati/HRMS")
    if lib and lib.is_file() and hrms and list(hrms.glob("*/*_TiO2_t010min.mzML")):
        return {"lib": lib, "hrms": hrms}
    return None


pytestmark = pytest.mark.skipif(_dati() is None, reason="MZLAB_DATI / QQQ_DATI with librerie/ and HRMS/ not available")

TARGETS = {"parent": (317.1642, 5.55, 5.85), "315": (315.1485, 5.15, 5.45), "291": (291.1485, 4.5, 4.75), "331": (331.1435, 4.6, 4.85),
           "319": (319.1435, 5.3, 5.5), "247": (247.1223, 2.6, 2.85)}
EXPECT_ANALOG = {"315": (-2.0159, 0.81), "291": (-26.0154, 0.75), "331": (13.9796, 0.84), "319": (1.9791, 0.72), "247": (-70.0419, 0.88)}      # (delta precursor, modified cosine) of the specification


def _queries(hrms):
    import sys
    sys.path.insert(0, str(ROOT))
    from mzlab.reader.mzml import Run
    out, allq = {}, []
    for fn in ("t002min", "t010min", "t020min", "t045min"):                  # the TP show up at different times: every target is looked for in the four files
        f = next(hrms.glob(f"*/*_TiO2_{fn}.mzML")); r = Run(str(f))
        for k, (mz, lo, hi) in TARGETS.items():
            for s in r.scans:
                if s.level == 2 and lo <= s.rt <= hi and abs((s.iso[0] + s.iso[1]) / 2 - mz) <= 0.005:
                    a, b = r.read(s.index)
                    if len(a):
                        out.setdefault(k, []).append({"rt": float(s.rt), "prec": float((s.iso[0] + s.iso[1]) / 2), "peaks": [[float(x), float(y)] for x, y in zip(a, b)]})
        if fn == "t010min":
            for s in r.scans:
                if s.level == 2:
                    a, b = r.read(s.index)
                    allq.append({"rt": float(s.rt), "prec": float((s.iso[0] + s.iso[1]) / 2), "peaks": [[float(x), float(y)] for x, y in zip(a, b)]})
    return out, allq


NODE = r"""
const fs=require('fs'), L=require(process.argv[1]);
const lib=fs.readFileSync(process.argv[2],'utf8'), Q=JSON.parse(fs.readFileSync(process.argv[3],'utf8'));
const specs=[], meta=[], prec=[], pol=[], off=[0]; let tot=0;
const P=new L.LibParser('msp', s=>{specs.push({mz:s.mz,it:s.it}); meta.push(s.meta); prec.push(s.prec); pol.push(s.pol); tot+=s.mz.length; off.push(tot)});
const t0=Date.now(); P.push(lib); P.end(); const idx=L.buildIndex(prec,pol,off);
const out={n:specs.length, read_ms:Date.now()-t0, ident:{}, analog:{}};
for (const [k,lst] of Object.entries(Q.queries)) {
  const rows=lst.map(q=>{const qq=L.queryOf(q.peaks,q.prec,1); const r=L.searchIndex(idx,qq,{ppm:10,frag:0.01,pol:1},pos=>specs[pos],meta).sort((a,b)=>b.ent-a.ent)[0]; return r?{name:r.name,ent:r.ent,cos:r.cos,shared:r.shared,dprec:r.dprec}:null});
  out.ident[k]=rows;
  const q=lst[Math.floor(lst.length/2)], qq=L.queryOf(q.peaks,q.prec,1), t=Date.now();
  const a=L.analogIndex(idx,qq,{maxDelta:200,frag:0.01,pol:1},pos=>specs[pos],meta);
  out.analog[k]={ms:Date.now()-t, top:a.slice(0,3).map(x=>({name:x.name,mcos:x.mcos,delta:x.delta,matched:x.matched}))};
}
const t1=Date.now(); let strong=0; const names={};
for (const q of Q.all) { const qq=L.queryOf(q.peaks,q.prec,1); if(!qq) continue; const r=L.searchIndex(idx,qq,{ppm:10,frag:0.01,pol:1},pos=>specs[pos],meta).sort((a,b)=>b.ent-a.ent)[0]; if(r&&r.ent>=0.75&&r.shared>=6){strong++; names[r.name]=(names[r.name]||0)+1} }
out.all={ms:Date.now()-t1, n:Q.all.length, strong, names};
console.log(JSON.stringify(out));
"""


@pytest.fixture(scope="module")
def result(tmp_path_factory):
    d = _dati(); q, allq = _queries(d["hrms"])
    tmp = tmp_path_factory.mktemp("ident"); qf = tmp / "q.json"; qf.write_text(json.dumps({"queries": q, "all": allq}))
    p = subprocess.run(["node", "-e", NODE, str(JS), str(d["lib"]), str(qf)], capture_output=True, encoding="utf-8", check=True)
    return json.loads(p.stdout)


def test_the_parent_is_identified_strongly(result):
    rows = result["ident"]["parent"]
    assert rows and all(r and r["ent"] >= 0.75 and r["shared"] >= 6 and abs(r["dprec"]) <= 5 for r in rows), rows
    assert len({r["name"] for r in rows}) == 1


def test_the_products_have_no_strong_identity(result):
    for k in ("315", "291", "331", "319", "247"):
        rows = result["ident"].get(k) or []
        assert rows, k
        assert not any(r and r["ent"] >= 0.75 and r["shared"] >= 6 for r in rows), (k, rows)


def test_the_analogues_find_the_parent_with_the_expected_shifts(result):
    parent = result["ident"]["parent"][0]["name"]
    for k, (dm, mc) in EXPECT_ANALOG.items():
        top = result["analog"][k]["top"]
        assert top and top[0]["name"] == parent, (k, top)
        assert abs(top[0]["delta"] - dm) <= 0.002 and abs(top[0]["mcos"] - mc) <= 0.05, (k, top[0], dm, mc)
        assert result["analog"][k]["ms"] <= 200 * 3, result["analog"][k]            # node, files already in memory (the browser target is 200 ms)


def test_a_whole_file_is_searched_in_a_second_and_the_parent_is_the_most_frequent_hit(result):
    a = result["all"]
    assert a["ms"] <= 3000, a                                                       # the browser target is 1 s for 3842 MS2 against 17290 spectra
    parent = result["ident"]["parent"][0]["name"]
    assert a["names"] and max(a["names"], key=a["names"].get) == parent, a["names"]
