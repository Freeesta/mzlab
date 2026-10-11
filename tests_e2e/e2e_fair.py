"""Open-data exports from an HR MS2 spectrum: right-click menu → MassBank record (dialog with checks, download) and SIRIUS 6 .ms; nothing in low resolution."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_fair.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\\n")[0][:240]))
D = synth("fair")
S2 = "E.panels.find(p=>p.dda!=null)"
TRIO = "(()=>{const c=E.panels.find(p=>p.type==='chrom'&&p.tab==='full'),s1=E.panels.find(p=>p.type==='spec'&&p.link===c.id);return {c,s1}})()"
r = Run(wd="/tmp/wdfair")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1400})
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2); ready(pg)
        pg.evaluate("(async()=>{const t=" + TRIO + ";const D=await DDA.get(t.s1.k);await DDA.selectMs2(" + S2 + ",D.sid[3])})()"); pg.wait_for_timeout(2500)
        def menu():
            pos = pg.evaluate("(()=>{const s=" + S2 + ";s.el.scrollIntoView({block:'center'});const r=s.cv.getBoundingClientRect();return {x:r.left+r.width/2,y:r.top+r.height/2}})()")
            pg.mouse.click(pos["x"], pos["y"], button="right"); pg.wait_for_timeout(400)
            t = pg.inner_text("#ctx"); assert "Record MassBank" in t and "SIRIUS 6" in t, t
        step("HR MS2: the menu offers the MassBank record and the SIRIUS file", menu)
        def refuse():
            pg.click("#ctx >> text=Record MassBank"); pg.wait_for_selector("#fairdlg[open]")
            pg.fill("#fa-name", "Test compound"); pg.click("#fa-go"); pg.wait_for_timeout(400)
            t = pg.inner_text("#fa-msg"); assert "SMILES" in t and "InChIKey" in t and "autori" in t.lower(), t        # nothing is downloaded with fields missing
        step("MassBank dialog: missing fields are listed and block the download", refuse)
        def record():
            pg.fill("#fa-smiles", "Cn1cnc2c1c(=O)n(C)c(=O)n2C"); pg.dispatch_event("#fa-smiles", "change"); pg.wait_for_function("document.querySelector('#fa-calc').textContent.includes('C8H10N4O2')", timeout=30000)
            pg.fill("#fa-ik", "RYYVLZVUVIJVGH-UHFFFAOYSA-N"); pg.fill("#fa-contrib", "Prova_Lab"); pg.fill("#fa-authors", "A. Autore"); pg.fill("#fa-instr", "Exploris"); pg.fill("#fa-itype", "LC-ESI-ITFT")
            try:
                with pg.expect_download(timeout=8000) as dl:
                    pg.click("#fa-go")
            except Exception:
                raise AssertionError("no download: " + pg.inner_text("#fa-msg"))
            txt = open(dl.value.path(), encoding="utf-8").read(); print(txt[:400])
            assert txt.startswith("ACCESSION: MSBNK-Prova_Lab-") and txt.rstrip().endswith("//") and "CH$FORMULA: C8H10N4O2" in txt and "LICENSE: CC BY" in txt and "AC$MASS_SPECTROMETRY: MS_TYPE MS2" in txt, txt[:300]
            n = int(txt.split("PK$NUM_PEAK: ")[1].split("\n")[0]); assert n > 0 and txt.count("\n  ") == n, n
            assert not pg.evaluate("document.querySelector('#fairdlg').open")
        step("MassBank record: downloaded with the compound, the spectrum and the licence", record)
        def sirius():
            pos = pg.evaluate("(()=>{const s=" + S2 + ";const r=s.cv.getBoundingClientRect();return {x:r.left+r.width/2,y:r.top+r.height/2}})()")
            pg.mouse.click(pos["x"], pos["y"], button="right"); pg.wait_for_timeout(400)
            pg.click("#ctx >> text=SIRIUS 6"); pg.wait_for_selector("#fairdlg[open]")
            with pg.expect_download() as dl:
                pg.click("#fa-go")
            txt = open(dl.value.path(), encoding="utf-8").read(); print(txt[:300])
            assert ">compound" in txt and ">parentmass" in txt and ">ionization [M+H]+" in txt and ("\n>collision" in txt or "\n>ms2" in txt), txt[:300]
        step("SIRIUS: .ms with parent mass, ionisation and MS2", sirius)
        def lr():
            assert pg.evaluate("FAIR.check({lr:true,mz:[],y:[]},{},{}).errors[0].key") == "lr"
        step("low resolution: the check refuses", lr)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
