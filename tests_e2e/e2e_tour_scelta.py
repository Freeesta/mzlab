"""End of the tour that loaded its own example file: one question, two buttons. «Keep exploring with this file» leaves the file loaded and the app free of every
tour overlay; «Remove it and start fresh» closes the example and all that came from it (same empty start screen as a first visit) and keeps the stored settings.
Esc, «Skip tour» and «Finish» all reach the same question; a tour on the student's own files asks nothing and never removes them; the dialog is modal
(focus, Tab, Enter, Esc, ARIA) and its tooltips are short."""
# covers: mzlab/web/tour.js, mzlab/web/tour-dopo.js, mzlab/web/lista-file.js
import json, sys, os, shutil
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

r = Run(port=8896, wd="/tmp/wd96")
steps = []
OWN = "/tmp/Mio_FullScan.mzML"
shutil.copy(str(ROOT / "mzlab" / "web" / "esempi" / "FullScan_t15.mzML"), OWN)
SCELTA = "document.querySelector('#tour-scelta #tour-scelta-dlg')"
NOTHING_LEFT = """(() => !document.querySelector('#tour-dialog, #tour-spotlight, #tour-scelta, #tour-invito, #tour-live') && !TOUR.active)()"""


def step(name, fn):
    import traceback
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, f"FAIL line {traceback.extract_tb(e.__traceback__)[-1].lineno}: {type(e).__name__} " + str(e).split("\n")[0][:220]))


def fresh_start(pg):
    pg.evaluate("fetch('api/new', {method:'POST', body:JSON.stringify({fresh:true})})")
    pg.reload()
    pg.wait_for_selector("#drop", timeout=60000)
    pg.evaluate("localStorage.removeItem('mzlab.tour.lr')")


def example_tour(pg):
    """Starts «Take the tour with an example file» from the gear on the empty start screen and waits for its first step."""
    pg.click("#np-set"); pg.wait_for_selector("#uip-tour", timeout=5000)
    assert "Take the tour with an example file" in pg.inner_text("#uip-tour")
    pg.click("#uip-tour")
    pg.wait_for_selector("#tour-dialog", timeout=60000)
    wait_for_js(pg, "E.files.filter(f => !f.gone).length === 1")


def to_the_end(pg):
    """Goes to the last step of chapter 2 and presses «Finish»; the file list is opened by a step first, to see that its drawer is put back."""
    pg.evaluate("TOUR.vai(2, 8)")
    pg.wait_for_function("document.querySelector('#tour-btn-fine')", timeout=10000)


def settings(pg):
    """Every stored value but the tour's own marker and the bookkeeping of the saved session (a flag, the pending notebook write, the visit counter): what the tour must not change."""
    return pg.evaluate("Object.fromEntries(Object.keys(localStorage).filter(k => !['mzlab.tour.lr', 'mzlab.sessione', 'mzlab.nbpending', 'mzlab.avvii'].includes(k)).sort().map(k => [k, localStorage.getItem(k)]))")


def diff(a, b):
    return {k: (str(a.get(k))[:60], str(b.get(k))[:60]) for k in sorted(set(a) | set(b)) if a.get(k) != b.get(k)}


def asked(pg):
    pg.wait_for_selector("#tour-scelta", timeout=10000)
    assert not pg.is_visible("#tour-dialog") and not pg.is_visible("#tour-spotlight"), "the tour overlay is gone while the question is shown"
    dlg = pg.locator("#tour-scelta-dlg")
    assert dlg.get_attribute("role") == "dialog" and dlg.get_attribute("aria-modal") == "true"
    assert pg.inner_text("#tour-scelta-ttl") == "End of the tour"
    assert pg.get_attribute("#tour-scelta-dlg", "aria-labelledby") == "tour-scelta-ttl" and pg.get_attribute("#tour-scelta-dlg", "aria-describedby") == "tour-scelta-txt"
    assert pg.inner_text("#tour-sc-resta") == "Keep exploring with this file" and pg.inner_text("#tour-sc-via") == "Remove it and start fresh"
    for b in ("#tour-sc-resta", "#tour-sc-via"):
        assert len(pg.get_attribute(b, "title")) <= 60, (b, pg.get_attribute(b, "title"))
    assert pg.evaluate("document.activeElement.id") == "tour-sc-resta", "focus starts on the choice that destroys nothing"


with sync_playwright() as p:
    pg = r.page(p, tour=True)
    fresh_start(pg)
    pg.evaluate("localStorage.setItem('mzlab.test.setting', 'kept')")      # a value the student had before the tour
    before = settings(pg)

    def keep():
        example_tour(pg)
        to_the_end(pg)
        pg.click("#tour-btn-fine")
        asked(pg)
        pg.keyboard.press("Tab"); assert pg.evaluate("document.activeElement.id") == "tour-sc-via", "Tab stays inside the dialog"
        pg.keyboard.press("Tab"); assert pg.evaluate("document.activeElement.id") == "tour-sc-resta", "...and wraps around"
        pg.keyboard.press("Enter")
        pg.wait_for_selector("#tour-scelta", state="detached", timeout=5000)
        assert pg.evaluate(NOTHING_LEFT), "no tour overlay, highlight or dialog is left"
        assert pg.evaluate("E.files.filter(f => !f.gone).length") == 1, "the example stays loaded"
        assert settings(pg) == before, diff(before, settings(pg))
        # fully usable: the file opens from the list and its plots appear
        pg.click("#flst .fl[data-k] .nm"); wait_for_js(pg, "document.querySelector('.pnl.chrom')")
        assert pg.evaluate("document.querySelector('#v-workspace').dataset.work") == "panels"
        pg.click("#np-set"); pg.wait_for_selector("#uip-tour", timeout=5000)
        assert "Replay the tour" in pg.inner_text("#uip-tour"), "afterwards the file is an ordinary one: the tour is a replay"
        pg.keyboard.press("Escape")
    step("Keep exploring: the example stays loaded, no overlay or forced state is left, the app is usable, settings unchanged", keep)

    def remove():
        fresh_start(pg)
        before2 = settings(pg)
        example_tour(pg)
        pg.evaluate("TOUR.vai(1, 0)")
        pg.click("#flst .fl[data-k] .nm")      # the file opens: panels, a cursor: all derived from the example
        wait_for_js(pg, "document.querySelector('.pnl.chrom') && E.panels.length > 0")
        pg.evaluate("TOUR.vai(2, 8)")
        pg.wait_for_function("document.querySelector('#tour-btn-fine')", timeout=10000)
        pg.click("#tour-btn-fine")
        asked(pg)
        pg.click("#tour-sc-via")
        pg.wait_for_selector("#tour-scelta", state="detached", timeout=30000)
        assert pg.evaluate(NOTHING_LEFT)
        wait_for_js(pg, "document.querySelector('#v-workspace').dataset.work === 'landing'")
        left = pg.evaluate("({files: E.files.filter(f => !f.gone).length, panels: E.panels.length, rows: ST.files.length, list: document.querySelectorAll('#flst .fl').length, drop: !!document.querySelector('#drop')?.offsetParent})")
        assert left == {"files": 0, "panels": 0, "rows": 0, "list": 0, "drop": True}, left
        assert pg.evaluate("fetch('api/session').then(r => r.json()).then(j => (j.session || []).length)") == 0, "the engine session is empty too"
        assert settings(pg) == before2, diff(before2, settings(pg))
        wait_for_js(pg, "!Object.keys(localStorage).some(k => (localStorage.getItem(k) || '').includes('Esempio_FullScan'))")      # nothing stored still names the example
        # a new file opens by itself as at the first visit (the example had switched that off)
        stage(pg, [OWN]); pg.click("text=Load data")
        wait_for_js(pg, "E.files.filter(f => !f.gone).length === 1 && document.querySelector('.pnl.chrom')", 90000)
    step("Remove it and start fresh: no file, panel, row or session is left; the start screen is the first-visit one; settings kept; a new file opens as usual", remove)

    def early():
        fresh_start(pg)
        for how in ("Escape", "skip"):
            example_tour(pg)
            if how == "Escape": pg.keyboard.press("Escape")
            else: pg.click("#tour-btn-skip")
            asked(pg)
            assert pg.evaluate("JSON.parse(localStorage.getItem('mzlab.tour.lr')).state") == "skipped"
            pg.keyboard.press("Escape")      # Esc on the question is «Keep exploring»: nothing is destroyed
            pg.wait_for_selector("#tour-scelta", state="detached", timeout=5000)
            assert pg.evaluate("E.files.filter(f => !f.gone).length") == 1 and pg.evaluate(NOTHING_LEFT)
            if how == "Escape":
                pg.evaluate("removeAllFiles()"); wait_for_js(pg, "E.files.filter(f => !f.gone).length === 0")
                fresh_start(pg)
        # reload in the middle of the tour: the overlay lives in memory only, the example is just a loaded file (the same state as «Keep exploring»)
        fresh_start(pg)
        example_tour(pg)
        saved(pg); pg.reload(); ready(pg)
        wait_for_js(pg, "E.files.filter(f => !f.gone).length === 1")
        assert pg.evaluate(NOTHING_LEFT)
    step("early exit: Esc and «Skip tour» ask the same question; Esc on it keeps the file; a reload leaves the file loaded and no overlay", early)

    def own_files():
        fresh_start(pg)
        stage(pg, [OWN]); ready(pg)
        pg.evaluate("localStorage.setItem('mzlab.tour.lr', JSON.stringify({state:'skipped'}))")
        pg.click("#np-set"); pg.wait_for_selector("#uip-tour", timeout=5000)
        assert "Replay the tour" in pg.inner_text("#uip-tour")
        pg.click("#uip-tour"); pg.wait_for_selector("#tour-dialog", timeout=10000)
        pg.click("#tour-btn-skip")
        pg.wait_for_selector("#tour-dialog", state="detached", timeout=5000)
        assert not pg.query_selector("#tour-scelta"), "no example was loaded by the tour: nothing to ask"
        assert pg.evaluate("E.files.filter(f => !f.gone).length") == 1
        # the student adds an own file while the example tour runs: «Remove» takes the example only
        fresh_start(pg)
        example_tour(pg)
        pg.set_input_files("#pick", [OWN]) if pg.query_selector("#pick") else None
        wait_for_js(pg, "E.files.filter(f => !f.gone).length === 2", 90000)
        pg.evaluate("TOUR.salta()")
        asked(pg)
        pg.click("#tour-sc-via")
        pg.wait_for_selector("#tour-scelta", state="detached", timeout=30000)
        names = pg.evaluate("E.files.filter(f => !f.gone).map(f => f.file.split('#')[0])")
        assert names == ["Mio_FullScan.mzML"], names
    step("the student's own files are never removed: no question without an example; with both loaded only the example goes", own_files)

    def shot():
        fresh_start(pg)
        example_tour(pg)
        pg.evaluate("TOUR.vai(2, 8)"); pg.wait_for_function("document.querySelector('#tour-btn-fine')", timeout=10000)
        pg.click("#tour-btn-fine"); pg.wait_for_selector("#tour-scelta", timeout=10000)
        (HERE / "shots").mkdir(exist_ok=True)
        pg.screenshot(path=str(HERE / "shots" / "tour_scelta.png"))
        pg.keyboard.press("Escape")
    step("screenshot of the final dialog", shot)

    late = [e for e in r.errs if "i18n: key asked before the rest" in str(e)]
    step("no text asked before the catalog", lambda: (_ for _ in ()).throw(AssertionError(late)) if late else None)

r.close()
for s in steps:
    print(" ", s)
r.report()
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
