"""H5: «Formule compatibili» (elemental composition): the dialog gives the formula of the infusion progenitor, shows the checks, keeps the elements in use; the menu entry is only on high-resolution spectra."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_hr_composizione.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e)[:600]))
D = synth("hrcomp")
r = Run(port=8968, wd="/tmp/wd_hrcomp")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1400})
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        def dialog():
            pg.evaluate("COMP.open({mz: 317.1639, polarity: 'positive'})"); pg.wait_for_timeout(400)
            pg.fill("#cmp-els", "C:0-13,H:0-25,N:0-4,O:0-3,S:0-1"); pg.click("#cmp-go"); pg.wait_for_selector("#cmp-out table", timeout=15000)
            t = pg.inner_text("#cmp-out"); print(t[:300])
            assert "C13H25N4O3S" in t.replace("\n", "").replace(" ", "") or "C13H25N4O3S" in pg.inner_text("#cmp-out tr:nth-child(2)").replace("\t", ""), t
            row = pg.inner_text("#cmp-out tr:nth-child(2)").replace("\t", " "); assert "C13H25N4O3S" in row.replace(" ", ""), row
            assert "non identificazioni" in pg.inner_text("#bigbody")
        step("317.1639 gives C13H25N4O3S with the elements of the infusion file", dialog)
        def xl():
            with pg.expect_download() as d: pg.click("#cmp-xlsx")
            assert d.value.suggested_filename.endswith(".xlsx") and os.path.getsize(d.value.path()) > 1500
        step("Excel of the candidate formulas", xl)
        def narrow():
            pg.fill("#cmp-els", "C:0-5,H:0-10"); pg.click("#cmp-go"); pg.wait_for_timeout(1500)
            assert "Nessuna formula" in pg.inner_text("#cmp-out"), pg.inner_text("#cmp-out")
            pg.fill("#cmp-els", "C:5-2"); pg.click("#cmp-go"); pg.wait_for_timeout(1000)
            assert pg.is_visible("#cmp-out .fail")
        step("no candidate and a wrong range are said clearly", narrow)
        def keep():
            pg.evaluate("document.querySelector('#bigdlg').close()"); pg.evaluate("COMP.open({mz: 200.1})"); pg.wait_for_timeout(300)
            assert pg.input_value("#cmp-els") == "C:5-2", pg.input_value("#cmp-els")      # the elements in use are remembered
            pg.evaluate("document.querySelector('#bigdlg').close()")
        step("the elements in use are remembered", keep)
        def menu():
            pg.evaluate("localStorage.removeItem('qqq.comp')")
            sp = pg.evaluate("(()=>{const s=E.panels.find(q=>q.type==='spec'&&q._a&&q._a.hrp);return s?s.id:null})()"); assert sp is not None, "an HR spectrum panel"
            xy = pg.evaluate(f"(()=>{{const s=E.panels.find(q=>q.id=={sp}),a=s._a,d=a.data[0].d;let b=0;d.y.forEach((v,i)=>{{if(v>d.y[b])b=i}});const r=s.cv.getBoundingClientRect();return {{x:r.left+a.X(d.mz[b]),y:r.top+a.Y(d.y[b])+6}}}})()")
            pg.evaluate(f"E.panels.find(q=>q.id=={sp}).el.scrollIntoView({{block:'center'}})"); pg.wait_for_timeout(300)
            xy = pg.evaluate(f"(()=>{{const s=E.panels.find(q=>q.id=={sp}),a=s._a,d=a.data[0].d;let b=0;d.y.forEach((v,i)=>{{if(v>d.y[b])b=i}});const r=s.cv.getBoundingClientRect();return {{x:r.left+a.X(d.mz[b]),y:r.top+a.Y(d.y[b])+6}}}})()")
            pg.mouse.click(xy["x"], xy["y"], button="right"); pg.wait_for_timeout(400)
            t = pg.inner_text("#ctx"); assert "Genera formula (m/z" in t, t
        step("right click on a peak of a high-resolution spectrum: «Genera formula»", menu)
        def score():
            sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
            from mzlab.chem.isoscore import fine_pattern, merge_res
            from mzlab.chem.subformulas import subformulas
            pat = [p for p in merge_res(fine_pattern({"C": 13, "H": 25, "N": 4, "O": 3, "S": 1}, 1), 70000.0) if p["rel"] >= 0.5]
            spec = {"mz": [p["mz"] for p in pat], "y": [p["rel"] * 1e4 for p in pat]}
            ms2 = {"mz": [m for _, m in subformulas("C13H25N4O3S", 1, 100.0)[1][::25][:6]], "y": [5e4] * 6}
            pg.evaluate("document.querySelector('#bigdlg').close()")
            pg.evaluate("localStorage.removeItem('qqq.comp')")
            pg.evaluate(f"COMP.open({{mz: {pat[0]['mz']}, polarity: 'positive', spec: {spec}, ms2: {ms2}}})"); pg.wait_for_timeout(400)
            pg.fill("#cmp-els", "C:0-20,H:0-40,N:0-6,O:0-6,S:0-2"); pg.click("#cmp-go"); pg.wait_for_selector("#cmp-out table", timeout=15000)
            hd = pg.inner_text("#cmp-out tr:nth-child(1)"); hd = hd.lower(); assert "punteggio" in hd and "ms2 spiegata" in hd and "alogeni" in hd, hd
            row = pg.inner_text("#cmp-out tr:nth-child(2)").replace("\t", " "); assert "C13H25N4O3S" in row.replace(" ", ""), row
            assert "massa" in row and "isotopi" in row and "%" in row, row
            det = pg.inner_text("#cmp-det"); assert "Soglie" in det and "contributo" in det.lower(), repr(det)
            pg.fill("#cmp-w0", "0"); pg.click("#cmp-go"); pg.wait_for_timeout(1500)                     # the weights are editable
            assert "pesi massa, isotopi, frammenti = 0,2,1" in pg.inner_text("#cmp-det"), pg.inner_text("#cmp-det")
            pg.evaluate("document.querySelector('#bigdlg').close()")
        step("full isotope score and fragment constraint: columns, terms per peak, editable weights", score)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
