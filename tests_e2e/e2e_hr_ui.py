"""B8/B9: HR and DDA pills in the list of the files, calculator with the observed m/z (ppm), window «Metodo» with the parameters of the scans."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_hr_ui.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\\n")[0][:240]))
D = synth("hrui")
r = Run(port=8900, wd="/tmp/wdhr10")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "B_FullMass-t0.mzML"], 4000, 1)
        def calc_qqq():
            pg.click("#np-calc2"); pg.wait_for_timeout(400)
            pg.fill("#calcin", "C14H13F4N3O2S"); pg.wait_for_timeout(1500)
            t = pg.inner_text("#calcdlg"); assert "364.0737" in t and not pg.locator("#calcobsw").count(), t       # QqQ: 4 decimals as before, no field for the observed value
            assert "errore (ppm)" not in t
            shut(pg, "#calcx"); pg.wait_for_timeout(200)
        step("calculator with a QqQ file only: as before", calc_qqq)
        def pills_qqq():
            rows = pg.evaluate("[...document.querySelectorAll('#flst .fl:not(.ghost)')].map(r=>[r.querySelector('.nm').textContent,[...r.querySelectorAll('.hrb')].map(x=>x.textContent)])"); print(rows)
            assert rows and all(b == [] for a, b in rows), rows
        step("no pill «HR» or «DDA» on the QqQ file", pills_qqq)
        def method_qqq():
            pg.evaluate("(()=>{const f=E.files.find(f=>f.file.startsWith('B_Full'));E.cur=f.k;showMethod()})()"); pg.wait_for_timeout(1500)
            t = pg.inner_text("#bigdlg"); assert "Manca il metodo di acquisizione" in t and "Parametri delle scansioni" not in t, t
            pg.evaluate("document.querySelector('#bigdlg').close()")
        step("window «Metodo» of a QqQ file: as before", method_qqq)
        def losses_qqq():
            pg.evaluate("QQQRef.open('ls',{q:'27.995'})"); pg.wait_for_timeout(800)
            hit = pg.evaluate("[...document.querySelectorAll('#refdlg .nlr.hit')].map(r=>r.dataset.f)"); ex = pg.evaluate("document.querySelector('#refdlg .nlr[data-f=CO] .nlm small')")
            print(hit, ex)
            assert "CO" in hit and "C2H4" in hit and ex is None, (hit, ex)                       # QqQ: ±0.5 Da and no exact masses
            pg.evaluate("document.querySelector('#refdlg').close()")
        step("neutral losses with a QqQ file: ±0.5 Da and no exact masses", losses_qqq)
        pg.evaluate("fetch('api/new',{method:'POST',body:JSON.stringify({fresh:true})})"); pg.reload(); pg.wait_for_timeout(1500)
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        def pills():
            assert pg.locator("#flst .hrb").count() == 0, "no pills in the file list"
        step("no pills on the Orbitrap file", pills)
        def calc_hr():
            pg.click("#np-calc2"); pg.wait_for_timeout(400)
            pg.fill("#calcin", ""); pg.fill("#calcin", "C14H13F4N3O2S"); pg.wait_for_timeout(1500)
            assert pg.is_visible("#calcobs"), "no field for the observed m/z"
            t = pg.inner_text("#calcdlg"); assert "364.0737" in t and "errore (ppm)" not in t, t
            pg.fill("#calcobs", "364.0740"); pg.wait_for_timeout(1500)
            t = pg.inner_text("#calcdlg"); print(t[:400])
            assert "errore (ppm)" in t, t
            e = pg.evaluate("[...document.querySelectorAll('#calcout tr')].map(r=>[...r.children].map(c=>c.textContent))[1]"); print(e)
            assert abs(float(e[2]) - 0.8) < 0.15, e                                         # (364.0740 - 364.07374) / 364.07374 = 0.7 ppm
            shut(pg, "#calcx"); pg.wait_for_timeout(200)
        step("calculator with an Orbitrap file: 4 decimals and the error in ppm of the observed m/z", calc_hr)
        def method():
            pg.evaluate("(()=>{const f=E.files.find(f=>f.file.includes('Exploris')&&f.kind==='full');E.cur=f.k;showMethod()})()"); pg.wait_for_timeout(2500)
            t = pg.inner_text("#bigdlg"); print(t[:600].replace("\n", " | "))
            assert "Il metodo completo non è nel file mzML" in t and "Parametri delle scansioni" in t and "NCE" in t and "HCD" in t and "45 000" in t and "±0.75" in t, t
            assert "Manca il metodo di acquisizione" not in t
            pg.evaluate("document.querySelector('#bigdlg').close()")
        step("window «Metodo» of an Orbitrap file without .dam: the parameters of the scans", method)
        def losses():
            pg.evaluate("QQQRef.open('ls',{q:'27.995'})"); pg.wait_for_timeout(800)
            hit = pg.evaluate("[...document.querySelectorAll('#refdlg .nlr.hit')].map(r=>r.dataset.f)"); ex = pg.evaluate("document.querySelector('#refdlg .nlr[data-f=CO] .nlm small')?.textContent")
            print(hit, ex)
            assert hit == ["CO"] and ex == "27.9949", (hit, ex)                                  # 3 mDa: CO yes, C2H4 (28.0313) no
            assert "Stessa massa nominale (28)" in pg.inner_text("#refdlg")      # the sentence about equal nominal masses stays
            pg.evaluate("document.querySelector('#refdlg').close()")
        step("neutral losses with an Orbitrap file: exact masses and «Cerca Δm» within 3 mDa", losses)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
