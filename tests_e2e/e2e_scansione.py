"""«+ scan» of the spectrum cell (high-resolution bench): every scan of the file in time order, opened as a TIC with the scan under it at the chosen place.
Synthetic Orbitrap DDA file (no real data needed)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_scansione.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("hrscan")
r = Run(port=8981, wd="/tmp/wd_hrscan")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1600})
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        SPEC = "E.panels.find(p=>p.type==='spec'&&p.link!=null&&p.dda==null&&p.tab==='full')"
        def sel(): return f"{SPEC}.el"
        def opener(): pg.evaluate(f"{sel()}.querySelector('[data-bf=scan]').click()"); pg.wait_for_selector(".bsp .r", timeout=15000)
        def btn():
            assert pg.evaluate(f"!!{sel()}.querySelector('button[data-bf=scan]')"), "the spectrum cell has +scan"
            assert pg.evaluate(f"!{sel()}.querySelector('select[data-bf=filt]')"), "and no more the menu of the types"
            b = pg.evaluate(f"(()=>{{const r={sel()}.querySelector('[data-bf=scan]').getBoundingClientRect();return [r.width,r.height]}})()"); assert b[0] >= 24 and b[1] >= 24, b
        step("spectrum cell: «+ scan» button instead of the type menu", btn)
        def listing():
            opener()
            n = pg.evaluate("+document.querySelector('.bsp [data-sp=n]').textContent.split(' ')[0].replace(/\\s/g,'')")
            tot = pg.evaluate(f"J('api/filters?k='+{SPEC}.k).then(j=>j.filters.reduce((a,g)=>a+g.n,0))")
            assert n == tot and n > 2, (n, tot)
            rts = pg.evaluate("[...document.querySelectorAll('.bsp .r')].map(r=>parseFloat(r.textContent.split('·')[1]))")
            assert rts == sorted(rts) and len(rts) > 2, rts
            assert pg.evaluate("[...document.querySelectorAll('.bsp .r')].some(r=>/MS2/.test(r.textContent))") or n < 40, "MS1 and MS2 scans in the same list"
        step("the list has every scan of the file, sorted by time", listing)
        def typefilter():
            keys = pg.evaluate("[...document.querySelectorAll('.bsp [data-sp=type] option')].map(o=>o.value).filter(Boolean)")
            g = pg.evaluate(f"J('api/filters?k='+{SPEC}.k).then(j=>j.filters.find(g=>g.key==={keys[0]!r}).n)")
            cnt = "+document.querySelector('.bsp [data-sp=n]').textContent.split(' ')[0].replace(/\\s/g,'')"
            pg.select_option(".bsp [data-sp=type]", keys[0]); attendi(pg, f"{cnt}==={g}")
            pg.select_option(".bsp [data-sp=type]", ""); attendi(pg, f"{cnt}!=={g}")
        step("the type menu inside narrows the list", typefilter)
        def esc():
            pg.keyboard.press("Escape"); attendi(pg, "!document.querySelector('.bsp')")
        step("Esc closes it", esc)
        def open_at(where, idx):
            n0 = pg.evaluate("E.panels.length"); ids0 = pg.evaluate("E.panels.map(p=>p.id)")
            opener(); pg.select_option(".bsp [data-sp=where]", where)
            pg.evaluate(f"document.querySelectorAll('.bsp .r')[{idx}].click()"); attendi(pg, f"E.panels.length=={n0 + 2}")
            assert pg.evaluate("E.panels.length") == n0 + 2, "a TIC and a spectrum are added"
            return pg.evaluate("(ids0=>{const o=stackOrder().map(q=>[q.id,q.type,q.link]);return {o,ids0}})", ids0)
        def at_end():
            g = open_at("end", 3); o, ids0 = g["o"], g["ids0"]
            assert o[-1][1] == "spec" and o[-2][1] == "chrom" and o[-2][0] not in ids0 and o[-1][2] == o[-2][0], o
            attendi(pg, "stackOrder().at(-2).cur!=null")
        step("«At the end»: new TIC with its spectrum under it", at_end)
        def at_top():
            g = open_at("top", 5); o, ids0 = g["o"], g["ids0"]
            assert o[0][1] == "chrom" and o[0][0] not in ids0 and o[1][1] == "spec" and o[1][2] == o[0][0], o
        step("«At the top»", at_top)
        def above_below():
            for where in ("above", "below"):
                src = pg.evaluate(f"{SPEC}.id")
                g = open_at(where, 7); o = g["o"]; ids = [q[0] for q in o]; i = ids.index(src)
                if where == "above": assert o[i - 1][1] == "spec" and o[i - 2][1] == "chrom", o
                else: assert o[i + 1][1] == "chrom" and o[i + 2][1] == "spec", o
        step("«Above / below this chart»", above_below)
        def keyboard():
            opener(); pg.keyboard.press("ArrowDown"); pg.keyboard.press("ArrowDown"); n0 = pg.evaluate("E.panels.length"); pg.keyboard.press("Enter"); attendi(pg, f"E.panels.length=={n0 + 2}")
        step("keyboard: arrows + Enter", keyboard)
        step("state is coherent", lambda: pg.evaluate("STATO.verifica()") == [] or (_ for _ in ()).throw(AssertionError("stato")))
    errs = [e for e in r.errs if not (e[0].startswith("console") and ("GL" in str(e[1]) or "sandbox" in str(e[1])))]
    steps.append(("no browser errors", "ok" if not errs else "FAIL " + str(errs[:3])))
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("OK  " if s == "ok" else "FAIL"), n, "" if s == "ok" else s)
sys.exit(1 if bad else 0)
