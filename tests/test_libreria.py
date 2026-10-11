"""Part D: reading MSP / MGF libraries and the scores (cosine, spectral entropy), run by node on mzlab/web/libreria-worker.js."""
import json
import subprocess
from pathlib import Path

JS = Path(__file__).resolve().parent.parent / "mzlab" / "web" / "libreria-worker.js"


def run(code):
    full = f"const L=require({str(JS)!r});{code}"
    return json.loads(subprocess.run(["node", "-e", full], check=True, capture_output=True, encoding="utf-8").stdout)


MSP = """Name: Alfa
PrecursorMZ: 300.1234
Precursor_type: [M+H]+
Ion_mode: Positive
Collision_energy: 30
Instrument_type: HCD
Formula: C10H10O
InChIKey: AAAAAAAAAAAAAA-BBBBBBBBBB-N
Num Peaks: 3
100.5\t1000
150.25\t500
200.1\t250

Name: Beta, negative
PRECURSORMZ: 255.5
Precursor_type: [M-H]-
Num Peaks: 2
80.0 10
120.5 20

Name: Senza precursore
Num Peaks: 2
50 10
60 20

Name: Senza picchi
PrecursorMZ: 400
Num Peaks: 0

Name: Rotto
PrecursorMZ: abc
Num Peaks: 1
10 10

Name: Gamma NIST style
PrecursorMZ: 410.2
Num Peaks: 3
50 100; 60 200; 70 300;
"""
MGF = """BEGIN IONS
NAME=Delta
PEPMASS=500.25 12345
CHARGE=1+
IONMODE=Positive
100.0 10
200.0 20
END IONS
BEGIN IONS
NAME=Rotto
PEPMASS=600.0
100.0 10
BEGIN IONS
NAME=Epsilon
PEPMASS=700.5
CHARGE=1-
300.0 5
END IONS
BEGIN IONS
NAME=SenzaPicchi
PEPMASS=800.0
END IONS
"""


def parse(fmt, text, chunk=50):
    return run(f"""const out=[],p=new L.LibParser({fmt!r},s=>out.push({{prec:s.prec,pol:s.pol,mz:Array.from(s.mz),it:Array.from(s.it),meta:s.meta}}));
      const t={json.dumps(text)};for(let i=0;i<t.length;i+=({chunk}))p.push(t.slice(i,i+{chunk}));p.end();console.log(JSON.stringify({{out,stats:p.stats}}))""")


def test_msp_blocks_fields_and_discarded():
    r = parse("msp", MSP)
    names = [s["meta"][0] for s in r["out"]]
    assert names == ["Alfa", "Beta, negative", "Gamma NIST style"], names
    a, b, g = r["out"]
    assert a["prec"] == 300.1234 and a["pol"] == 1 and a["meta"][1:] == ["[M+H]+", "30", "HCD", "C10H10O", "AAAAAAAAAAAAAA-BBBBBBBBBB-N", "", "", ""]      # then accession, authors, license (empty here)
    assert a["mz"] == [100.5, 150.25, 200.10000610351562] or abs(a["mz"][2] - 200.1) < 1e-4       # float32
    assert a["it"][0] == 1 and abs(a["it"][2] - 0.25) < 1e-6                                       # normalised to the highest
    assert b["pol"] == -1 and g["pol"] == 0 and len(g["mz"]) == 3                                  # polarity from Ion_mode / adduct; none = 0; «50 100; 60 200;» pairs
    assert r["stats"] == {"read": 3, "dropped": 3, "noPrec": 2, "noPeaks": 1, "broken": 0}, r["stats"]


def test_msp_is_the_same_in_any_chunking():
    assert parse("msp", MSP, 7)["out"] == parse("msp", MSP, 100000)["out"]


def test_mgf_with_a_broken_block():
    r = parse("mgf", MGF)
    assert [s["meta"][0] for s in r["out"]] == ["Delta", "Epsilon"], r
    d, e = r["out"]
    assert d["prec"] == 500.25 and d["pol"] == 1 and e["pol"] == -1
    assert r["stats"]["read"] == 2 and r["stats"]["broken"] == 1 and r["stats"]["noPeaks"] == 1 and r["stats"]["dropped"] == 2, r["stats"]


def test_only_the_100_most_intense_peaks_are_kept():
    peaks = "\n".join(f"{100 + i} {1 + (i * 37) % 1000}" for i in range(250))
    r = parse("msp", f"Name: Big\nPrecursorMZ: 500\nNum Peaks: 250\n{peaks}\n")
    s = r["out"][0]
    assert len(s["mz"]) == 100 and s["mz"] == sorted(s["mz"]) and max(s["it"]) == 1


def test_format_detection():
    assert run("console.log(JSON.stringify([L.formatOf('a.MSP',''),L.formatOf('b.mgf',''),L.formatOf('c.lib',''),L.formatOf('d.mzVault',''),L.formatOf('x.txt','BEGIN IONS\\n'),L.formatOf('x','Name: a\\n'),L.formatOf('x','zzz')]))") == ["msp", "mgf", "unsupported", "unsupported", "mgf", "msp", "unknown"]


def spec(mz, it):
    return {"mz": mz, "it": it}


def scores(a, b, tol=0.01):
    return run(f"""const f=s=>L.topPeaks(s.mz,s.it);const A=f({json.dumps(a)}),B=f({json.dumps(b)});const r=L.score(A,B,{tol});console.log(JSON.stringify({{cos:r.cos,ent:r.ent,shared:r.shared}}))""")


def test_identical_spectra_score_one():
    s = spec([100, 150, 200, 250], [1000, 500, 250, 100])
    r = scores(s, s)
    assert abs(r["cos"] - 1) < 1e-9 and abs(r["ent"] - 1) < 1e-9 and r["shared"] == 4


def test_spectra_without_common_peaks_score_zero():
    r = scores(spec([100, 150, 200], [10, 5, 2]), spec([101, 151.5, 300], [10, 5, 2]))
    assert r["cos"] == 0 and abs(r["ent"]) < 1e-9 and r["shared"] == 0


def test_tolerance_decides_the_pairs_and_partial_overlap_is_between():
    a, b = spec([100, 150, 200], [10, 8, 6]), spec([100.004, 150.5, 200.008], [10, 8, 6])
    tight, loose = scores(a, b, 0.01), scores(a, b, 1.0)
    assert tight["shared"] == 2 and loose["shared"] == 3 and 0 < tight["cos"] < loose["cos"] <= 1 and 0 < tight["ent"] < 1


def test_entropy_matches_a_hand_computation_for_two_peaks():
    # two equal peaks in A, one of them also in B (B = that peak only): S_A = ln 2 < 3 -> weight 0.25 + 0.25 ln 2, B is a single peak (S = 0, weight 0.25, p = 1)
    import math
    r = scores(spec([100, 200], [1, 1]), spec([100], [1]))
    w = 0.25 + 0.25 * math.log(2); pa = [0.5 ** w, 0.5 ** w]; t = sum(pa); pa = [v / t for v in pa]       # = 0.5, 0.5
    m = [(pa[0] + 1) / 2, pa[1] / 2]
    H = lambda p: -sum(v * math.log(v) for v in p if v > 0)
    expected = 1 - (2 * H(m) - H(pa) - 0) / math.log(4)
    assert abs(r["ent"] - expected) < 1e-9, (r, expected)


def test_search_window_polarity_order_and_the_pack_round_trip():
    r = run("""
      const prec=[300.1234,300.1260,255.5,300.1235,500], pol=[1,1,-1,-1,0], off=[0,2,4,6,8,10];
      const peaks=[[100,200],[100,200],[100,200],[100,200],[100,200]].map((_, i)=>({mz:Float32Array.from([100,200+i*3]),it:Float32Array.from([1,0.5])}));
      const idx=L.buildIndex(prec,pol,off), meta=prec.map((p,i)=>['n'+i,'','','','','']);
      const q=L.queryOf([[100,1000],[200,500]],300.1234,1);
      const ppm=L.searchIndex(idx,q,{ppm:5,frag:0.01,pol:1},p=>peaks[p],meta);
      const wide=L.searchIndex(idx,q,{da:0.5,frag:0.01,pol:0},p=>peaks[p],meta);
      console.log(JSON.stringify({sorted:Array.from(idx.prec), ord:Array.from(idx.ord), ppm:ppm.map(x=>[x.name,Math.round(x.dprec*10)/10,x.shared]), wide:wide.map(x=>x.name)}));""")
    assert r["sorted"] == sorted([300.1234, 300.1260, 255.5, 300.1235, 500]) and r["ord"] == [2, 0, 3, 1, 4]
    assert r["ppm"][0][0] == "n0" and r["ppm"][0][1] == 0 and r["ppm"][0][2] == 2                      # 300.1260 is 8.7 ppm away; the negative one is another polarity
    assert [x[0] for x in r["ppm"]] == ["n0"] or len(r["ppm"]) == 1
    assert sorted(r["wide"]) == ["n0", "n1", "n3"]


def test_archive_index_round_trip():
    r = run("""
      const prec=[300.5,100.25,200.75], idx=L.buildIndex(prec,[1,-1,0],[0,3,5,9]);
      const u=L.unpackIndex(L.packIndex(idx,9));
      console.log(JSON.stringify({n:u.n,tp:u.totalPeaks,prec:Array.from(u.prec),ord:Array.from(u.ord),pol:Array.from(u.pol),off:Array.from(u.off)}));""")
    assert r == {"n": 3, "tp": 9, "prec": [100.25, 200.75, 300.5], "ord": [1, 2, 0], "pol": [1, -1, 0], "off": [0, 3, 5, 9]}


def test_modified_cosine_matches_at_shift_zero_and_at_the_precursor_difference():
    out = run("""
const lib={prec:300.1,mz:Float32Array.from([60.5,100.1,150.2,200.3,250.4]),it:Float32Array.from([100,400,900,300,200])};
const q  ={prec:316.1,mz:Float32Array.from([60.5,100.1,166.2,216.3,266.4]),it:Float32Array.from([100,400,900,300,200])};   // +15.9949-ish: the last three peaks carry the modification
const r=L.modCosine(q,lib,0.01);
const plain=L.score({mz:q.mz,it:q.it},{mz:lib.mz,it:lib.it},0.01);
console.log(JSON.stringify({cos:r.cos,matched:r.matched,shifted:r.shifted,delta:r.delta,plain:plain.cos}));""")
    assert out["matched"] == 5 and out["shifted"] == 3 and out["cos"] > 0.99 and out["plain"] < 0.5 and abs(out["delta"] - 16.0) < 1e-6


def test_analog_search_keeps_modified_neighbours_and_drops_unrelated_ones():
    out = run("""
const mk=(prec,pk)=>({prec,mz:Float32Array.from(pk.map(p=>p[0])),it:Float32Array.from(pk.map(p=>p[1]))});
const specs=[mk(300.1,[[60.5,100],[100.1,400],[150.2,900],[200.3,300],[250.4,200]]),            // 0: the analogue (query = this + 16)
             mk(310.0,[[55.0,100],[95.0,300],[120.0,800],[180.0,300],[222.0,100]]),             // 1: unrelated
             mk(316.1,[[60.5,100],[100.1,400],[166.2,900],[216.3,300],[266.4,200]]),            // 2: the identity (same precursor): not an analogue
             mk(900.0,[[60.5,100],[100.1,400],[150.2,900],[200.3,300],[250.4,200]])];           // 3: far away
const idx=L.buildIndex(specs.map(s=>s.prec),[1,1,1,1],[0,5,10,15,20]);
const q=mk(316.1,[[60.5,100],[100.1,400],[166.2,900],[216.3,300],[266.4,200]]);
const r=L.analogIndex(idx,q,{maxDelta:200,frag:0.01,pol:1},pos=>specs[pos],null);
console.log(JSON.stringify(r.map(x=>({pos:x.pos,mcos:x.mcos,delta:x.delta,shifted:x.shifted}))));""")
    assert [r["pos"] for r in out] == [0] and out[0]["mcos"] > 0.99 and abs(out[0]["delta"] - 16.0) < 1e-6 and out[0]["shifted"] == 3


def test_index_positions_point_to_the_right_spectra_for_a_precursor_search():
    r = run("""
      const prec=[], pol=[], off=[0], peaks=[]; let tot=0;
      for (let i=0;i<50;i++){ prec.push(100+i*7.3); pol.push(1); peaks.push({mz:Float32Array.from([i+1,i+2]),it:Float32Array.from([1,0.5])}); tot+=2; off.push(tot); }
      const u=L.unpackIndex(L.packIndex(L.buildIndex(prec,pol,off),tot)), want=prec[37], k=L.lowerBound(u.prec,want-0.001);
      console.log(JSON.stringify({pos:u.ord[k], off:[u.off[u.ord[k]],u.off[u.ord[k]+1]]}));""")
    assert r == {"pos": 37, "off": [74, 76]}
