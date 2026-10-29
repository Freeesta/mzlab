"""A4: the XIC from a chromatogram sits right under it; no peak-table button; short caption; the RT label has a fixed width."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_pannelli.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8839, wd="/tmp/wd_pann")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("setTab('full',true)"); pg.wait_for_timeout(600)
        order = lambda: pg.evaluate("stackOrder().map(p => p.type + ':' + p.id)")
        def xic_under():
            pg.click("#np-chrom"); pg.wait_for_timeout(900)
            chroms = pg.evaluate("E.panels.filter(p => p.type === 'chrom').map(p => p.id)"); assert len(chroms) >= 2, chroms
            o0 = order(); first = pg.evaluate("stackOrder().find(p => p.type === 'chrom').id"); i0 = [x for x in o0].index("chrom:%d" % first)
            pg.locator(f'.pnl.chrom >> nth=0').locator('[data-a="xic"]').click(); pg.wait_for_timeout(500)
            pg.fill("#xic-mz", "364"); pg.click("#xic-go"); ready(pg)
            o1 = order(); j = [x for x in o1 if x.startswith("xic:")]; assert j, o1
            assert o1.index(j[0]) == i0 + 1, (o0, o1)          # right under the chromatogram it came from, the others slid down
            assert len(o1) == len(o0) + 1
            # same from the right-click menu on a chromatogram that is not the first
            n = pg.evaluate("stackOrder().filter(p => p.type === 'chrom').length"); assert n >= 2
            last = pg.evaluate("stackOrder().filter(p => p.type === 'chrom').pop().id")
            pg.evaluate(f"openXic(null, {{ after: E.panels.find(p => p.id === {last}) }})"); pg.wait_for_timeout(500); pg.fill("#xic-mz", "194"); pg.click("#xic-go"); ready(pg)
            o2 = order(); assert o2.index("chrom:%d" % last) + 1 == [k for k, x in enumerate(o2) if x.startswith("xic:") and k > o2.index("chrom:%d" % last)][0], o2
        step("A4.1: the XIC sits right under its chromatogram", xic_under)
        def split():
            pg.evaluate("openXic(E.panels.find(p => p.type === 'xic'))"); pg.wait_for_timeout(400); pg.fill("#xic-mz", "152"); pg.click("#xic-go"); ready(pg)
            xp = pg.evaluate("stackOrder().find(p => p.type === 'xic').id"); o0 = order(); k = o0.index("xic:%d" % xp)
            pg.locator('.pnl.xic [data-o="split"]').first.click(); pg.wait_for_timeout(2500)
            o1 = order(); assert o1[k] == "xic:%d" % xp and o1[k + 1].startswith("xic:"), (o0, o1)      # the split panel follows the original
        step("A4.1: «Separa» puts the new panels right under the original", split)
        def notable():
            assert pg.locator(".pnl.spec [data-a=ptab]").count() == 0
            assert pg.evaluate("typeof peakTable") == "undefined"
        step("A4.2: no «Tabella dei picchi» in the spectra", notable)
        def caption():
            pg.evaluate("setActive(E.panels.find(p => p.type === 'spec'))")
            t = pg.inner_text(".pnl.spec .leg").strip()
            import re; assert re.fullmatch(r"(scansione \d+/\d+|media di \d+ scansioni)", t), t
        step("A4.3: caption without TIC, base peak, RT", caption)
        def rtl():
            sp = "E.panels.find(p => p.type === 'spec')"
            def meas(rt):
                pg.evaluate(f"(() => {{ const p = {sp}; p.r0 = {rt}; p.r1 = {rt}; afterDraw(p); }})()")
                return pg.evaluate(f"(() => {{ const e = {sp}.el, l = e.querySelector('.rtl'); return [l.getBoundingClientRect().width, e.querySelector('.ctl, .bt').getBoundingClientRect().left, l.textContent]; }})()")
            a, b = meas(1.05), meas(14.30)
            assert abs(a[0] - b[0]) < 0.6 and abs(a[1] - b[1]) < 0.6, (a, b)
            w = pg.evaluate("(() => { const l = document.querySelector('.pnl.spec .rtl'), s = l.cloneNode(); s.textContent = '88.88 min'; s.style.cssText += ';min-width:0;position:absolute;visibility:hidden'; l.parentNode.appendChild(s); const w = s.getBoundingClientRect().width; s.remove(); return w; })()")
            assert w <= a[0] + 0.5, (w, a)                  # wide enough for «RT 88.88 min»
        step("A4.4: the RT label has a fixed width, the buttons do not move", rtl)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
