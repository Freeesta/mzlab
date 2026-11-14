"""The DAD of a file with no scans on the built site with the Rust engine: an MRM and a SIM (Q1MI) 3200 `.wiff` with a UV/Vis matrix (synthetic pairs of
tools/wiff_dad_mrm.py, no laboratory data). `api/chrom?kind=pda&mz0&mz1` answers the wavelength trace (its band in nm, its maximum the synthetic absorbance),
not the TWC; without a band it answers the TWC; the MS chromatograms of the file stay as they were.
The detector switch: the picker of the MRM / SIM panel and of the MS2 chromatogram has a «UV/Vis» row that opens the DAD TWC in the same tab, right under the
panel, time-linked, by mouse and by keyboard (arrows, Esc, Enter), axe clean; a file without a DAD has no such row. Run from the repo root: python3 tests_e2e/e2e_dad_mrm.py"""
# covers: tools/wiff_dad_mrm.py, mzlab/web/canali.js, mzlab/web/canali-uv.js, mzlab/web/rust-bridge-info.js, mzlab/web/lr-method.js
import json, subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

D = Path(tempfile.mkdtemp(prefix="dad_mrm_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_dad_mrm.py"), str(D)], check=True, capture_output=True)
TRUE = json.loads((D / "dadmrm.json").read_text())
AXE = axe_path()       # axe-core (axe-playwright-python): its steps run when it is installed, as in e2e_axe_grafici
MRM, MS2 = "E.panels.find(q => q.type === 'mrm')", "E.panels.find(q => q.type === 'chrom' && q.tab === 'ms2' && q.kind !== 'pda')"
UV = lambda tab: f"E.panels.find(q => q.type === 'chrom' && q.tab === '{tab}' && q.kind === 'pda')"
def load(pg, names):
    pg.set_input_files("#pick", [str(D / f"{n}{x}") for n in names for x in (".wiff", ".wiff.scan")])
    wait_for_js(pg, f"document.querySelectorAll('#flist input[data-k=use]').length >= {len(names)}", timeout=90000); pg.click("text=Carica dati"); ready(pg)
    wait_for_js(pg, f"E.files.length >= {len(names)}", timeout=120000)
def axe_ok(pg, sel):
    v = pg.evaluate("""(sel) => axe.run(document.querySelector(sel) || document, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}})
      .then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.nodes.length, v.nodes.slice(0, 3).map(n => n.target.join(' ')).join(' | ')]))""", sel)
    assert not v, v
CHROM = "fetch('api/chrom?k=%d&kind=%s&level=1%s').then(r => r.json()).then(j => ({max: Math.max(...j.y), at: j.rt[j.y.indexOf(Math.max(...j.y))], n: j.y.length, nm0: j.nm0 ?? null, nm1: j.nm1 ?? null}))"
r = Run(port=8879, wd="/tmp/wd_dad_mrm")
def pick(panel, kind):
    """A click on a row of the picker of a panel, as a user does."""
    pg.evaluate(f"{panel}.el.querySelector('[data-o=canali],[data-o=kind]').click()")
    wait_for_js(pg, f"!!{panel}.el.querySelector('.can-pop:not([hidden]) [data-kind={kind}]')")
    pg.evaluate(f"{panel}.el.querySelector('.can-pop [data-kind={kind}]').click()")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, ("dadmrm", "dadsim", "dadms2"))     # three pairs: three samples
        K = {kind: pg.evaluate(f"E.files.find(f => (f.label || f.file || '').startsWith('{kind}')).k") for kind in ("dadmrm", "dadsim", "dadms2")}
        def files():
            got = sorted(pg.evaluate("E.files.map(f => [f.kind, f.pda === true])"))
            assert got == [["mrm", True], ["mrm", True], ["ms2", True]], got
        step("the MRM, SIM and MS2 files all have a DAD", files)
        def band(kind):
            def f():
                for b in ("a", "b"):
                    t = TRUE[b]
                    got = pg.evaluate(CHROM % (K[kind], "pda", f"&mz0={t['nm0']:g}&mz1={t['nm1']:g}"))
                    assert (got["nm0"], got["nm1"]) == (t["nm0"], t["nm1"]), (b, got)
                    assert got["n"] == TRUE["rows"] and abs(got["max"] - t["max"]) < 0.01 * t["max"] and abs(got["at"] - t["rt"]) < 0.03, (b, got, t)
                twc = pg.evaluate(CHROM % (K[kind], "pda", ""))
                assert twc["nm0"] is None and abs(twc["max"] - TRUE["twc_max"]) < 0.01 * TRUE["twc_max"], twc
                assert twc["max"] > 10 * TRUE["a"]["max"], twc
            return f
        step("MRM: the 270-278 nm trace is the band (max and apex of the synthetic absorbance), the 228-232 nm one too, without a band the TWC", band("dadmrm"))
        step("SIM: the same", band("dadsim"))
        def ms_kinds():
            for kind in ("dadmrm", "dadsim"):
                tic = pg.evaluate(CHROM % (K[kind], "tic", ""))
                assert tic["n"] > 300 and tic["nm0"] is None and tic["max"] > 5.0e4, (kind, tic)
        step("the TIC of both files is still the MS one", ms_kinds)
        def mrm_row():
            open_kind(pg, "mrm"); wait_for_js(pg, f"!!{MRM} && !!{MRM}.el.querySelector('[data-o=canali]')", timeout=30000)
            pg.evaluate(f"{MRM}.el.querySelector('[data-o=canali]').click()")
            wait_for_js(pg, f"!!{MRM}.el.querySelector('.can-pop:not([hidden]) [data-kind=pda]')")
            got = pg.evaluate(f"[...{MRM}.el.querySelectorAll('.can-pop [role=group]')].map(g => [g.getAttribute('aria-label'), g.querySelectorAll('.can-row').length])")
            assert [g[0] for g in got] == ["Spettrometria di massa", "UV/Vis"] and got[0][1] >= 2 and got[1][1] == 1, got
            assert pg.evaluate(f"{MRM}.el.querySelector('.can-pop [data-kind=pda]').getAttribute('title')") == pg.evaluate("I18N.t('ctl.kind.pda.title')")
        step("MRM tab: the transition picker has a «UV/Vis» group with the DAD row", mrm_row)
        if AXE:
            pg.add_script_tag(path=AXE)
            step("axe: the open MRM picker with the UV/Vis group", lambda: axe_ok(pg, ".pnl.mrm .can-pop"))
        def mrm_keys():
            pg.keyboard.press("Escape"); wait_for_js(pg, f"{MRM}.el.querySelector('.can-pop').hidden")
            assert pg.evaluate("document.activeElement.dataset.o") == "canali"
            pg.keyboard.press("ArrowDown"); wait_for_js(pg, f"!{MRM}.el.querySelector('.can-pop').hidden && document.activeElement.classList.contains('can-row')")
            pg.keyboard.press("End"); assert pg.evaluate("document.activeElement.dataset.kind") == "pda"
            pg.keyboard.press("ArrowUp"); assert pg.evaluate("document.activeElement.dataset.omo || ''").startswith("srm"), pg.evaluate("document.activeElement.outerHTML.slice(0, 200)")
            pg.keyboard.press("ArrowDown"); pg.keyboard.press("Enter")
            wait_for_js(pg, f"!!{UV('mrm')} && !!{UV('mrm')}._a && {UV('mrm')}._a.sr.length === shownFor({UV('mrm')}).length", timeout=60000)
        step("keyboard: Esc closes, ArrowDown opens, End reaches the DAD row, Enter opens the DAD panel", mrm_keys)
        def mrm_panel():
            got = pg.evaluate(f"(() => {{ const m = {MRM}, q = {UV('mrm')}, st = stackOrder(); return {{ tab: E.tab, n: E.panels.filter(x => x.tab === 'mrm' && x.type === 'chrom').length, after: st.indexOf(q) === st.indexOf(m) + 1, tl: q.tl, kind: q.kind, max: q._a.sr.map(s => Math.max(...s.y)), focus: q.el.contains(document.activeElement), btn: q.el.querySelector('[data-o=kind]').textContent }}; }})()")
            assert got["tab"] == "mrm" and got["n"] == 1 and got["after"] and got["tl"] and got["kind"] == "pda" and got["focus"], got
            assert all(abs(v - TRUE["twc_max"]) < 0.01 * TRUE["twc_max"] for v in got["max"]), got
            assert got["btn"].startswith(pg.evaluate("I18N.t('ctl.kind.pda')")), got
            pick(MRM, "pda")
            assert pg.evaluate("E.panels.filter(x => x.tab === 'mrm' && x.type === 'chrom').length") == 1
        step("the DAD panel is in the MRM tab, right under the transitions, time-linked, with the TWC of every file shown; asking again opens no second one", mrm_panel)
        def ms2_row():
            open_kind(pg, "ms2"); wait_for_js(pg, f"!!{MS2} && !!{MS2}.el.querySelector('[data-o=kind]')", timeout=30000)
            pick(MS2, "pda")
            wait_for_js(pg, f"!!{UV('ms2')} && !!{UV('ms2')}._a && {UV('ms2')}._a.sr.length === 1", timeout=60000)
            got = pg.evaluate(f"(() => {{ const m = {MS2}, q = {UV('ms2')}, st = stackOrder(); return {{ tab: E.tab, kind: m.kind, after: st.indexOf(q) === st.indexOf(m) + 1, max: Math.max(...q._a.sr[0].y), xic: q.el.querySelector('[data-a=xic]').disabled, mxic: m.el.querySelector('[data-a=xic]').disabled }}; }})()")
            assert got["tab"] == "ms2" and got["kind"] == "tic" and got["after"] and not got["xic"] and got["mxic"], got
            assert abs(got["max"] - TRUE["twc_max"]) < 0.01 * TRUE["twc_max"], got
        step("MS2 tab: the chromatogram picker has «MS2» and the DAD row, which opens the TWC under it (the MS2 chromatogram stays on its scans)", ms2_row)
        if AXE:
            def ms2_axe():
                pg.evaluate(f"{MS2}.el.querySelector('[data-o=kind]').click()"); wait_for_js(pg, f"!{MS2}.el.querySelector('.can-pop').hidden")
                assert pg.evaluate(f"{MS2}.el.querySelector('.can-pop [data-kind=ms2]').getAttribute('aria-selected')") == "true"
                axe_ok(pg, ".pnl.chrom .can-pop:not([hidden])"); pg.keyboard.press("Escape")
            step("axe: the open MS2 picker", ms2_axe)
        step("the engine answered every request", lambda: (lambda m: m and (_ for _ in ()).throw(AssertionError(m)))(pg.evaluate("window.MZLAB_LR_MISS || []")))
        step("no page error", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r.errs if x[0] == "pageerror"]))
        for names, want in ((("dadsim",), True), (("nodad",), False)):
            q = r.page(p); load(q, names); open_kind(q, "mrm"); wait_for_js(q, f"!!{MRM} && !!{MRM}.el.querySelector('[data-o=canali]')", timeout=30000)
            def alone(q=q, want=want, name=names[0]):
                q.evaluate(f"{MRM}.el.querySelector('[data-o=canali]').click()"); wait_for_js(q, f"!{MRM}.el.querySelector('.can-pop').hidden")
                has = q.evaluate(f"!!{MRM}.el.querySelector('.can-pop [data-kind=pda]')")
                assert has == want and q.evaluate(f"{MRM}.el.querySelectorAll('.can-pop [role=group]').length") == (2 if want else 0), (name, has)
                if want:
                    q.evaluate(f"{MRM}.el.querySelector('.can-pop [data-kind=pda]').click()")
                    wait_for_js(q, f"!!{UV('mrm')} && !!{UV('mrm')}._a && {UV('mrm')}._a.sr.length === 1", timeout=60000)
                    m = q.evaluate(f"Math.max(...{UV('mrm')}._a.sr[0].y)")
                    assert abs(m - TRUE["twc_max"]) < 0.01 * TRUE["twc_max"], m
            step("SIM file alone: the ion picker has the DAD row and opens the TWC" if want else "MRM file without a DAD: no UV/Vis group", alone)
            q.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
