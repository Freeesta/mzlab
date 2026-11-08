"""DDA file whose index gives no precursor intensity (a Thermo .raw; here a synthetic mzML with its `peak intensity` hidden, same bytes long):
«Segui questo ione» and a row of the precursor list open the most intense MS2 of that precursor, the intensity read by the engine from the parent
Full Scans (`/api/dda_window` with `mz`), the same MS2 a reading of each parent scan around the precursor gives, not just the first one."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_dda_pint.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("ddapint")
src = D / "HR_DDA-Exploris-t30.mzML"; f = D / "HR_DDA-NoPint-t30.mzML"
b = src.read_bytes(); assert b.count(b'accession="MS:1000042"') > 10
f.write_bytes(b.replace(b'accession="MS:1000042"', b'accession="MS:1000043"'))      # same length: the offsets of the index still hold
r = Run(port=8981, wd="/tmp/wdddapint")
TRIO = "(()=>{const s1=E.panels.find(p=>p.type==='spec'&&p.duo!=null),c=s1&&E.panels.find(p=>p.id===s1.link),s2=s1&&E.panels.find(p=>p.id===s1.duo);return {c,s1,s2}})()"
# the precursor whose most intense MS2 (apex of the parent scan within the isolation half-width, at most 0.5 Da) is not its first one
PICK = """(async()=>{const t=""" + TRIO + """;const D=await DDA.get(t.s1.k),seen=new Set();
  for(let j=0;j<D.sid.length;j++){const m=D.prec[j]; if(m==null||seen.has(m)) continue;
    t.s2.ion=m; const l=DDA.list(t.s2,D); l.forEach(i=>seen.add(D.prec[i])); if(l.length<3) continue;
    const v=[]; for(const i of l){const s=await J(`api/scan?k=${t.s1.k}&sid=${D.parent[i]}`),h=D.lo[i]!=null?Math.min((D.hi[i]-D.lo[i])/2,0.5):0.5;
      let top=0; s.mz.forEach((x,n)=>{if(Math.abs(x-D.prec[i])<=h+1e-9&&s.y[n]>top) top=s.y[n]}); v.push(top)}
    const b=v.indexOf(Math.max(...v)); if(b>0&&v.filter(x=>x===v[b]).length===1){t.s2.ion=null; return {m, sid:D.sid[l[b]], first:D.sid[l[0]], n:l.length}}}
  t.s2.ion=null; return null})()"""
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [f], 6000, 1)
        wait_for_js(pg, "(()=>{const t=" + TRIO + ";return t.s1&&t.s1._a&&t.s2&&DDA.ready(t.s1.k)})()", 60000); ready(pg)
        want = {}
        def none():
            n = pg.evaluate("(()=>{const t=" + TRIO + ";const D=DDA.ready(t.s1.k);return {n:D.sid.length,with:D.pint.filter(v=>v!=null).length}})()"); print(n)
            assert n["n"] > 20 and n["with"] == 0, n
            want.update(pg.evaluate(PICK) or {}); print(want)
            assert want and want["sid"] != want["first"], want
        step("the index gives no precursor intensity, and a precursor's most intense MS2 is not its first", none)
        def follow():
            pg.evaluate("(m=>{const t=" + TRIO + ";return DDA.follow(t.s1,m)})", want["m"])
            wait_for_js(pg, "(()=>{const s2=E.panels.find(p=>p.type==='spec'&&p.ion!=null&&p.sid!=null);return s2&&s2._scan})()", 30000); ready(pg)
            got = pg.evaluate("(()=>{const s2=E.panels.find(p=>p.type==='spec'&&p.ion!=null&&p.sid!=null);return s2.sid})()")
            assert got == want["sid"], (got, want)
        step("«Segui questo ione» opens the most intense MS2 of the precursor", follow)
        def from_list():
            pg.evaluate("(()=>{const s2=E.panels.find(p=>p.type==='spec'&&p.ion!=null);s2.sid=null})()")
            pg.evaluate("E.tab='ms2'"); pg.evaluate("DDA.openList()"); pg.wait_for_selector("#ddatb tr[data-r]", timeout=20000)
            row = pg.evaluate("(m=>[...document.querySelectorAll('#ddatb tr[data-r]')].findIndex(r=>Math.abs(parseFloat(r.cells[0].textContent)-m)<0.01))", want["m"])
            assert row >= 0, row
            pg.locator("#ddatb tr[data-r]").nth(row).click(); ready(pg)
            wait_for_js(pg, "E.panels.some(p=>p.type==='spec'&&p.sid===%d&&p._scan)" % want["sid"], 30000)
        step("a row of the precursor list opens the same MS2", from_list)
finally:
    r.close()
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n + ("" if s == "ok" else " -- " + s))
bad = [e for e in r.errs if e[0] in ("pageerror", "console.error")]
for e in bad: print("ERR", e)
sys.exit(1 if any(s != "ok" for _, s in steps) or bad else 0)
