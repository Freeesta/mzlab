"""Roles and times in the loading table (campioni/2): the boxes Blank · t0 · Infusion, the role select, the proposed times (yellow until confirmed),
the warnings of the engine and «Open» held back by a missing time. Synthetic files with invented names; everything is done with the keyboard where it matters."""
import sys, os, shutil, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lib
lib.AUTO_TEMPI = False
from lib import *
import synth
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_ruoli_tempi.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
NAMES = ["X_t000min", "X_t005min", "X_t030min", "X_dark-adsorption", "X_blank", "X_standard_direct-infusion_MSn"]
tmp = Path(tempfile.mkdtemp(prefix="rt_"))
paths = []
for k, n in enumerate(NAMES):
    p = tmp / f"{n}.mzML"; synth.write_full(p, 5 * k, blank=n.endswith("blank"), seed=k); paths.append(str(p))
r = Run(port=8830 + 140, wd="/tmp/wd_ruoli")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.wait_for_selector("#start:not([hidden])", timeout=60000)
        stage(pg, paths)
        pg.wait_for_function("document.querySelector('#flist [data-box=t0] .rl-name') && document.querySelector('#flist [data-box=t0] .rl-name').textContent.trim() !== '—'", timeout=30000)
        row = lambda n: f"#flist tr:has(td:text-is('{n}.mzML'))"
        def role(n): return pg.eval_on_selector(f"{row(n)} select[data-k=type]", "e => e.value")
        def tval(n): return pg.eval_on_selector(f"{row(n)} input[data-k=time]", "e => e.value")
        def box(b): return pg.inner_text(f"#flist [data-box={b}] .rl-name").strip()
        def proposals():
            assert role("X_t000min") == "t0", role("X_t000min")
            assert role("X_t005min") == "sample" and role("X_t030min") == "sample"
            assert role("X_dark-adsorption") == "dark", role("X_dark-adsorption")
            assert role("X_blank").startswith("blank"), role("X_blank")
            assert role("X_standard_direct-infusion_MSn") == "infusion", role("X_standard_direct-infusion_MSn")
            assert tval("X_t000min") == "0" and tval("X_t005min") == "5" and tval("X_t030min") == "30", (tval("X_t005min"), tval("X_t030min"))
            assert box("t0") == "X_t000min.mzML" and box("blank") == "X_blank.mzML" and box("infusion").startswith("X_standard_direct-infusion_MSn"), (box("t0"), box("blank"), box("infusion"))
            assert pg.eval_on_selector(f"{row('X_t005min')} input[data-k=time]", "e => e.classList.contains('guess')"), "proposed time must be yellow"
            assert "proposto" in pg.get_attribute(f"{row('X_t005min')} input[data-k=time]", "title") or "nome del file" in pg.get_attribute(f"{row('X_t005min')} input[data-k=time]", "title")
            assert pg.is_disabled(f"{row('X_t000min')} input[data-k=time]"), "t0 is fixed at 0"
        step("roles and times proposed from the names", proposals)
        def warn(): return pg.inner_text("#rl-warn").strip()
        def unconfirmed():
            pg.wait_for_function("document.querySelector('#rl-warn') && document.querySelector('#rl-warn').textContent.includes('confermare')", timeout=20000)
            assert pg.get_attribute("#rl-warn", "role") == "status"
        step("warning: times to confirm", unconfirmed)
        def gate_blocks():
            pg.click("#opbtn")
            pg.wait_for_function("document.activeElement && document.activeElement.dataset && document.activeElement.dataset.k === 'time'", timeout=20000)
            f = pg.evaluate("document.activeElement.getAttribute('aria-label')")
            assert "X_t005min" in f, f
            assert pg.is_visible("#start"), "the start screen must stay"
        step("Open held back, focus on the first unconfirmed time", gate_blocks)
        def enter_confirms():
            pg.keyboard.press("Enter")
            assert not pg.eval_on_selector(f"{row('X_t005min')} input[data-k=time]", "e => e.classList.contains('guess')")
            assert pg.eval_on_selector(f"{row('X_t030min')} input[data-k=time]", "e => e.classList.contains('guess')")
        step("Enter confirms one row", enter_confirms)
        def confirm_all():
            pg.click("#rl-conf")
            pg.wait_for_function("!document.querySelector('#flist input.guess[data-k=time]')", timeout=10000)
            pg.wait_for_function("!document.querySelector('#rl-warn').textContent.includes('confermare')", timeout=10000)
            assert pg.is_hidden("#rl-conf")
        step("Confirm times", confirm_all)
        def no_t0():
            pg.focus("#flist [data-box=t0]"); pg.keyboard.press("Delete")
            pg.wait_for_function("document.querySelector('#rl-warn').textContent.includes('Senza t0')", timeout=15000)
            assert role("X_t000min") == "sample" and box("t0") == "—", (role("X_t000min"), box("t0"))
            assert pg.evaluate("document.activeElement.dataset.box") == "t0", "focus must stay on the box"
        step("deleting the t0 role warns", no_t0)
        def missing_blocks():
            pg.click("#opbtn")
            pg.wait_for_function("document.activeElement && document.activeElement.dataset && document.activeElement.dataset.k === 'time'", timeout=20000)
            assert pg.evaluate("document.activeElement.getAttribute('aria-invalid')") == "true"
            assert "richiesto" in pg.evaluate("document.activeElement.title"), pg.evaluate("document.activeElement.title")
            assert "mancante" in warn(), warn()
        step("missing time held back, aria-invalid", missing_blocks)
        def keyboard_menu():
            pg.focus("#flist [data-box=t0]"); pg.keyboard.press("Enter")
            pg.wait_for_selector("#flist .rl-menu")
            assert pg.evaluate("document.activeElement.closest('.rl-menu') !== null")
            pg.keyboard.press("ArrowDown"); pg.keyboard.press("ArrowUp")         # arrows move inside the list
            pg.focus("#flist .rl-menu [data-pick]:has-text('X_t000min')"); pg.keyboard.press("Enter")
            pg.wait_for_function("document.querySelector('#flist [data-box=t0] .rl-name').textContent.trim() !== '—'", timeout=10000)
            assert role("X_t000min") == "t0" and tval("X_t000min") == "0", (role("X_t000min"), tval("X_t000min"), box("t0"))
            pg.wait_for_function("!document.querySelector('#rl-warn').textContent.includes('Senza t0') && !document.querySelector('#rl-warn').textContent.includes('mancante')", timeout=15000)
        step("t0 box chosen with the keyboard", keyboard_menu)
        def opens():
            pg.click("#opbtn")
            pg.wait_for_selector("#start", state="hidden", timeout=90000)
        step("Open once the times are in", opens)
finally:
    r.close(); shutil.rmtree(tmp, ignore_errors=True)
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("OK   " if s == "ok" else "FAIL ") + n + ("" if s == "ok" else "  -> " + s))
errs = [e for e in r.errs if e[0] in ("pageerror",)]
print("page errors:", errs)
sys.exit(1 if bad or errs else 0)
