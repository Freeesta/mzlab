"""Two Full Scan files with the TICs overlaid: scrolling or clicking the TIC never moves the scans (or the spectrum) to the other file.
The file selected in the side panel stays the active one; the linked mass spectrum shows that single file."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_tic_file_attivo.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
SPEC = "E.panels.find(p=>p.type==='spec'&&p.link!=null)"
r = Run(port=8864, wd="/tmp/wd64")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [mz(f) for f in FILES[:2]])
        pg.click("text=Carica dati"); ready(pg)
        attendi(pg, "E.files.filter(f=>f&&f.kind==='full').length>=2")
        k0 = pg.evaluate("(()=>{const f=E.files.filter(f=>f&&f.kind==='full');f.forEach(x=>{x.vis=true});STATO.file(f[0].k,'file');E.browse=false;renderNav();redrawAll();return f[0].k})()")
        attendi(pg, "E.panels.some(p=>p.type==='chrom'&&p.tab==='full'&&p._a&&p._a.sr&&p._a.sr.length>=2)")
        def put_cursor(rt):
            c = pg.evaluate("(rt)=>{const p=E.panels.find(p=>p.type==='chrom'&&p.tab==='full'),r=p.cv.getBoundingClientRect();return {px:r.left+p._a.X(rt),py:r.top+r.height/2}}", rt)
            pg.mouse.move(c["px"] - 5, c["py"]); pg.mouse.move(c["px"], c["py"]); pg.mouse.down(); pg.mouse.up()
        mid = 14.3                                                  # where the two TICs differ most (the parent decays with time)
        put_cursor(6.0); attendi(pg, f"!!({SPEC})&&Math.abs(({SPEC}.r0+{SPEC}.r1)/2-6)<0.2")      # the cursor leaves the peak: a click on it would drag it
        # select the file whose TIC is the LOWER one at the cursor: the old rule (highest trace) would have sent the scans to the other file
        k0 = pg.evaluate("(x)=>{const a=E.panels.find(p=>p.type==='chrom'&&p.tab==='full')._a;const v=s=>s.ys[nearIdx(s.x,x)];const lo=a.sr.reduce((b,s)=>v(s)<v(b)?s:b);STATO.file(lo.k,'file');renderNav();return lo.k}", mid)
        put_cursor(mid)
        attendi(pg, f"!!({SPEC})&&Math.abs(({SPEC}.r0+{SPEC}.r1)/2-{mid})<0.2")
        def click_selected():
            assert pg.evaluate(f"{SPEC}.k") == k0
        step("click on the overlaid TICs: spectrum of the selected file", click_selected)
        def walk():
            pg.evaluate("setActive(E.panels.find(p=>p.type==='chrom'&&p.tab==='full'))")
            si0 = pg.evaluate(f"{SPEC}.si")
            for i in range(25):
                pg.keyboard.press("ArrowRight")
                attendi(pg, f"{SPEC}.si!=null&&{SPEC}.si==={(si0 or 0) + i + 1}" if si0 is not None else f"{SPEC}.si!=null&&{SPEC}.si>={i}")
                assert pg.evaluate(f"{SPEC}.k") == k0, i
            assert pg.evaluate("E.panels.find(p=>p.type==='chrom'&&p.tab==='full')._sk") == k0
            assert pg.evaluate(f"!{SPEC}.all && {SPEC}._a.data.length") == 1
        step("25 arrow steps: always the selected file, one file in the spectrum", walk)
        def other_file():
            k1 = pg.evaluate("E.files.filter(f=>f&&f.kind==='full')[1].k")
            pg.evaluate(f"STATO.file({k1},'file')"); renderok = pg.evaluate("renderNav(),true"); assert renderok
            pg.evaluate("setActive(E.panels.find(p=>p.type==='chrom'&&p.tab==='full'))")
            si = pg.evaluate(f"{SPEC}.si"); pg.keyboard.press("ArrowRight")
            attendi(pg, f"{SPEC}.k==={k1}")
            assert pg.evaluate("E.panels.find(p=>p.type==='chrom'&&p.tab==='full')._sk") == k1
        step("changing the selection at the side moves the walk to that file", other_file)
finally:
    r.close(); r.report()
print("\nSTEPS")
for s in steps: print("  ", s)
