"""«What is it?»: the voice of the menu exists only for high-resolution MS2 spectra; identity and analogs together, level and evidence, stereoisomer groups,
the decision of the chemist (2b only there), the lines of the mirror (pairs of the golden file of Attributio), the thresholds."""
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
from pathlib import Path
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_ricerca_ms2.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
GOLD = json.loads((Path(__file__).resolve().parent.parent / "tests" / "golden" / "librerie" / "parita_attributio_ms2.json").read_text())
D = synth("ricms2")
r = Run(port=8981, wd="/tmp/wd_ricms2")
TRIO = "(()=>{const s1=E.panels.find(p=>p.type==='spec'&&p.duo!=null),s2=E.panels.find(p=>p.dda!=null);return {s1,s2}})()"
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1500})
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 6000, 2); pg.wait_for_timeout(2000)
        info = pg.evaluate("""(async()=>{const k=E.files.find(f=>f.kind==='ms2').k, d=await J('api/dda?k='+k), cnt={};d.tgt.forEach(t=>{if(t)cnt[t]=(cnt[t]||0)+1});
          const t=+Object.keys(cnt).sort((a,b)=>cnt[b]-cnt[a])[0], i=d.tgt.indexOf(t), s=await J('api/scan?k='+k+'&sid='+d.sid[i]);
          return {k, t, n:cnt[t], sid:d.sid[i], mz:s.mz, y:s.y}})()""")
        pk = [(m, y) for m, y in zip(info["mz"], info["y"]) if m < info["t"] - 1.5 and y > 0]
        def msp(name, prec, peaks, ik):
            return f"Name: {name}\nPrecursorMZ: {prec}\nPrecursor_type: [M+H]+\nIon_mode: Positive\nFormula: C10H10\nInChIKey: {ik}\nLicense: CC BY 4.0\nAuthors: Autore di prova\nAccession: ACC001\nNum Peaks: {len(peaks)}\n" + "\n".join(f"{m:.5f} {y:.1f}" for m, y in peaks) + "\n\n"
        text = (msp("Voce di prova", info["t"], pk, "AAAAAAAAAAAAAA-AAAAAAAAAA-N") + msp("Gemella di prova", info["t"], pk, "AAAAAAAAAAAAAA-BBBBBBBBBB-N")
                + msp("Analogo di prova", info["t"] - 15.9949, [(m if i < len(pk) // 2 else m - 15.9949, y) for i, (m, y) in enumerate(pk)], "CCCCCCCCCCCCCC-AAAAAAAAAA-N"))
        pg.evaluate("(async t=>{await LIB.call('add',{file:new File([t],'prova.msp')})})(%s)" % json.dumps(text)); pg.wait_for_timeout(600)
        def open_menu():
            pg.evaluate("(async()=>{const t=" + TRIO + ";await DDA.selectMs2(t.s2," + str(info["sid"]) + ")})()"); pg.wait_for_timeout(2500)
            pos = pg.evaluate("(()=>{const t=" + TRIO + ";t.s2.el.scrollIntoView({block:'center'});const r=t.s2.cv.getBoundingClientRect();return {x:r.left+r.width*0.5,y:r.top+r.height*0.3}})()")
            pg.mouse.click(pos["x"], pos["y"], button="right"); pg.wait_for_timeout(400)
            assert "Cos'è?" in pg.inner_text("#ctx"), pg.inner_text("#ctx")
        step("the MS2 panel of the DDA trio has the voice «Cos'è? (cerca nelle librerie)»", open_menu)
        def result():
            pg.click("#ctx >> text=Cos'è?"); pg.wait_for_selector("#libres #lr-body table", timeout=60000); pg.wait_for_timeout(1500)
            t = pg.inner_text("#libres")
            assert pg.evaluate("document.querySelector('#libres').open") and "Candidati" in t and "Analoghi" in t, t[:300]
            names = pg.locator("#libres tr.row[data-k=id]:not([hidden]) td:nth-child(2)").all_inner_texts()
            assert names and names[0].startswith("Voce di prova") or names[0].startswith("Gemella"), names
            assert "Analogo di prova" in pg.inner_text("#libres tr.row[data-k=an]"), t
            head = pg.inner_text("#libres #lr-body table tr >> nth=0")
            for col in ("Entropia", "Reverse", "Picchi in comune", "Intensità spiegata", "Addotto", "Polarità", "CE", "Strumento", "Licenza"):
                assert col in head, (col, head)
            assert "CC BY" in pg.inner_text("#libres tr.row[data-k=id] >> nth=0"), "license"
        step("identity and analogs together, with entropy, reverse, shared peaks, explained intensity, adduct, polarity, CE, instrument and license", result)
        def groups():
            b = pg.locator("#libres [data-grp]"); assert b.count() == 1, b.count()          # the twin shares the first InChIKey block
            assert "stesso primo blocco" in b.first.inner_text()
            assert pg.locator("#libres tr.mem[hidden]").count() == 1
            b.first.click(); pg.wait_for_timeout(300)
            assert pg.locator("#libres tr.mem:not([hidden])").count() == 1
        step("entries with the same first InChIKey block are grouped (stereoisomers), expandable", groups)
        def level():
            v = pg.inner_text("#libres #lr-verdict")
            assert "Candidato · L" in v and "Prove" in v and "Livello proposto" in v and "Livello deciso" in v, v
            assert "entropia" in v and "vs" in v and ("✓" in v or "✗" in v), v
            assert "è " not in pg.inner_text("#libres .vd .lvb"), "never «it is X»"
            n = pg.evaluate("LIB.assess([{name:'x',ent:.9,shared:8,dprec:1}],{nScans:1}).proposed"); assert n == "3a"
        step("a level badge «Candidato · LN», one evidence line per gate, proposed and decided level", level)
        def decision():
            prop = pg.evaluate("document.querySelector('#libres #lv-dec').closest('div').innerText")
            lv = pg.evaluate("(()=>{const m=document.querySelector('#libres .vd .lvb').textContent.match(/Candidato · L(\\w+)/);return m&&m[1]})()")
            assert lv in ("2a", "3a"), lv
            pg.select_option("#libres #lv-dec", "2b"); pg.wait_for_timeout(300)
            assert "L2b" in pg.inner_text("#libres .vd .lvb"), pg.inner_text("#libres .vd")
            assert "Livello proposto: L" + lv in pg.inner_text("#libres .vd").replace("\n", " "), "the proposal stays"
            pg.select_option("#libres #lv-dec", "1"); pg.wait_for_timeout(300)
            assert "standard" in pg.inner_text("#libres #lv-msg") and pg.input_value("#libres #lv-dec") == "2b"
            pg.select_option("#libres #lv-dec", ""); pg.wait_for_timeout(200)
        step("2b only by the decision of the chemist; 1 refused without a standard", decision)
        def thresholds():
            pg.click("#libres #lr-sog summary"); pg.fill("#libres [data-thr=entropia_min]", "0,999"); pg.dispatch_event("#libres [data-thr=entropia_min]", "change"); pg.wait_for_timeout(500)
            assert pg.evaluate("JSON.parse(localStorage.getItem('mzlab.lib.soglie')).mzlab.entropia_min") == 0.999
            assert "L3" in pg.inner_text("#libres .vd .lvb") or "L2a" not in pg.inner_text("#libres .vd .lvb")
            wf = pg.evaluate("LIVELLI.exportState()"); assert wf["soglie"]["entropia_min"] == 0.999 and wf["formato"] == "livelli/1"
            pg.click("#libres #lr-sog-reset"); pg.wait_for_timeout(300)
            assert pg.evaluate("LIVELLI.thresholds().entropia_min") == 0.75
        step("editable thresholds saved in mzlab.lib.soglie and exported for the workflow file", thresholds)
        def mirror_lines():
            lines = json.loads(pg.get_attribute("#libres #lr-cv", "data-lines")); assert len(lines) >= 4 and all(l[2] == 0 for l in lines), lines
            pg.locator("#libres tr.row[data-k=an]").first.click(); pg.wait_for_timeout(400)
            ln = json.loads(pg.get_attribute("#libres #lr-cv", "data-lines")); assert any(l[2] == 1 for l in ln), ln
            assert "Perdite neutre" in pg.inner_text("#libres #lr-detail")
        step("the mirror joins the paired peaks (shifted ones too) and lists the shared neutral losses", mirror_lines)
        def attribution():
            pg.locator("#libres tr.row[data-k=id]").first.click(); pg.wait_for_timeout(300)
            assert "Autore di prova" in pg.inner_text("#libres #lr-detail") and "ACC001" in pg.inner_text("#libres #lr-detail"), pg.inner_text("#libres #lr-detail")
        step("authors, accession and license under the mirror", attribution)
        def golden_pairs():
            ok = 0
            for tipo in ("stesso", "analogo", "chimerico"):
                cs = [c for c in GOLD["casi"] if c["tipo"] == tipo and c["match_tol0.02"]["pairs"]]
                if tipo == "analogo": cs = [c for c in cs if c["match_tol0.02"]["n_shifted"] > 0]
                assert cs, tipo; c = cs[0]
                got = pg.evaluate("""(c) => { const cv = document.createElement('canvas'); cv.style.width = '900px'; cv.style.height = '320px'; document.body.appendChild(cv);
                  const nz = a => { const m = Math.max(...a); return a.map(v => v / m); };
                  LIB.mirror(cv, {mz: c.a.mz, it: nz(c.a.int), hr: true}, {mz: c.b.mz, it: nz(c.b.int), prec: c.b.prec, name: 'x', shared: c.match['matched'], pairs: c.match.pairs});
                  const out = JSON.parse(cv.dataset.lines); cv.remove(); return out; }""", {"a": c["a"], "b": c["b"], "match": c["match_tol0.02"]})
                assert sorted(map(tuple, got)) == sorted(tuple(x) for x in c["match_tol0.02"]["pairs"]), (c["id"], got[:5])
                ok += 1
            assert ok == 3
        step("3 cases of the golden file of Attributio: the lines of the mirror join exactly those pairs (type 0 and 1)", golden_pairs)
        def cosvis():
            q = {"it": [1, .5, .2], "mz": [100, 200, 300]}
            v = pg.evaluate("(q) => LIB.cosVis({it: q.it}, {mz: q.mz, it: [1, .5, .2], pairs: [[0,0],[1,1],[2,2]]}, 150)", q)
            assert 0 < v < 1 and abs(v - ((.5 * 1) ** 0 and (0.5 + 0.2) / ((1.7) ** .5 * (0.7) ** .5))) < 1e-9, v
            assert pg.evaluate("LIB.cosVis({it:[1]}, {mz:[100], it:[1], pairs:[[0,0]]}, 50)") is None          # nothing cut: no extra proof
        step("cosine on the visible peaks (reference peaks below the scan window are cut), a proof that does not change the level", cosvis)
    r.close()
    with sync_playwright() as p:
        r = Run(port=8982, wd="/tmp/wd_ricms2lr"); pg = r.page(p); pg.set_viewport_size({"width": 1400, "height": 1000})
        pg.set_input_files("#pick", [mz("B_MS2-t15")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("setTab('ms2',true)"); pg.wait_for_timeout(2500)
        def lr_none():
            SPEC = "E.panels.find(p => p.type === 'spec' && p.level === 2)"
            assert pg.evaluate(f"LIB.isHr({SPEC})") is False
            c = pg.evaluate(f"(() => {{ const p = {SPEC}; p.el.scrollIntoView({{block:'center'}}); const q = p.cv.getBoundingClientRect(); return {{x: q.left + q.width * 0.5, y: q.top + q.height * 0.3}}; }})()")
            pg.mouse.click(c["x"], c["y"], button="right"); pg.wait_for_timeout(400)
            t = pg.inner_text("#ctx"); assert "Cos'è?" not in t, t
        step("low resolution: the voice «Cos'è?» does not exist (the program gives no answers)", lr_none)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
