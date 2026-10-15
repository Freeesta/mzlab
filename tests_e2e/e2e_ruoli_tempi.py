"""Roles and times in the loading table (campioni/2): the role select (no t0 role: a sample at time 0 is the t0; one Blank only), the times read from the names (plain editable values,
no strip, no warning, nothing to confirm: «Load data» is the confirmation) and the negative time of the dark. Synthetic files with invented names."""
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
        pg.wait_for_function("RUOLI.busy() === 0 && ST.files.length >= 6 && ST.files.every(f => f.prop)", timeout=30000)
        row = lambda n: f"#flist tr:has(td:text-is('{n}.mzML'))"
        def role(n): return pg.eval_on_selector(f"{row(n)} select[data-k=type]", "e => e.value")
        def tval(n): return pg.eval_on_selector(f"{row(n)} input[data-k=time]", "e => e.value")
        def proposals():
            assert role("X_t000min") == "sample", role("X_t000min")          # no t0 role: the time 0 is the t0
            assert pg.eval_on_selector(f"{row('X_t000min')} select[data-k=type]", "e => [...e.options].map(o => o.value)") == ["sample", "blank", "dark", "infusion", "standard", "qc"]
            assert pg.query_selector("#flist [data-box], #rl-warn, #rl-conf, .rl-bar") is None, "no strip above the table"
            assert role("X_t005min") == "sample" and role("X_t030min") == "sample"
            assert role("X_dark-adsorption") == "dark", role("X_dark-adsorption")
            assert role("X_blank").startswith("blank"), role("X_blank")
            assert role("X_standard_direct-infusion_MSn") == "infusion", role("X_standard_direct-infusion_MSn")
            assert tval("X_t000min") == "0" and tval("X_t005min") == "5" and tval("X_t030min") == "30", (tval("X_t005min"), tval("X_t030min"))
            assert role("X_blank") == "blank"
            assert not pg.eval_on_selector(f"{row('X_t005min')} input[data-k=time]", "e => e.classList.contains('guess')"), "a time from the name is a normal cell"
            assert not pg.is_disabled(f"{row('X_t000min')} input[data-k=time]"), "the time of the first sample is a normal cell"
        step("roles and times proposed from the names", proposals)
        def never_blocks():
            # times read from the names, two samples: «Load data» needs no confirmation, no message under the button, no cell marked invalid
            assert pg.eval_on_selector("#flist", "e => e.querySelectorAll('input[aria-invalid=true]').length") == 0
            assert pg.inner_text("#gomsg").strip() == "", pg.inner_text("#gomsg")
            assert pg.evaluate("ST.files.every(f => !f.use || !RUOLI.timed(f) || f.time == null || !('gT' in f))")
        step("times from the names are plain values, nothing to confirm", never_blocks)
        def dark_negative():
            sel = f"{row('X_dark-adsorption')} input[data-k=time]"
            assert "buio" in (pg.get_attribute(sel, "title") or ""), "the dark cell says what its time is"
            pg.fill(sel, "-30"); pg.keyboard.press("Tab")
            pg.wait_for_function("[...document.querySelectorAll('#flist tr')].some(r => r.textContent.includes('X_dark-adsorption.mzML') && r.querySelector('input[data-k=time]').value === '-30')", timeout=10000)
            pg.fill(sel, "45"); pg.keyboard.press("Tab")                    # a positive number typed in a dark is read as its negative
            pg.wait_for_function("[...document.querySelectorAll('#flist tr')].some(r => r.textContent.includes('X_dark-adsorption.mzML') && r.querySelector('input[data-k=time]').value === '-45')", timeout=10000)
            pg.fill(f"{row('X_t005min')} input[data-k=time]", "-5"); pg.keyboard.press("Tab")        # a sample takes no sign
            assert pg.eval_on_selector(f"{row('X_t005min')} input[data-k=time]", "e => e.classList.contains('bad')")
            pg.fill(f"{row('X_t005min')} input[data-k=time]", "5"); pg.keyboard.press("Tab")
            pg.wait_for_function("!document.querySelector('#flist input.bad')", timeout=10000)
            assert tval("X_t005min") == "5" and tval("X_dark-adsorption") == "-45"
        step("the dark takes a negative time", dark_negative)
        def no_t0():
            sel = f"{row('X_t000min')} input[data-k=time]"
            pg.fill(sel, ""); pg.keyboard.press("Tab")
            pg.wait_for_function(f"[...document.querySelectorAll('#flist tr')].some(r => r.textContent.includes('X_t000min.mzML') && r.querySelector('input[data-k=time]').value === '')", timeout=10000)
            assert pg.eval_on_selector("#flist", "e => e.querySelectorAll('input[aria-invalid=true]').length") == 0
            assert pg.inner_text("#gomsg").strip() == "" and pg.inner_text("#flist").count("Senza") == 0, "no time 0 and no warning"
            pg.focus(f"{row('X_blank')} select[data-k=type]")
            pg.select_option(f"{row('X_blank')} select[data-k=type]", "sample")
            assert role("X_blank") == "sample"
            pg.fill(sel, "0"); pg.keyboard.press("Tab")
        step("no time 0 or a role changed: nothing blocks, no warning", no_t0)
        def opens():
            pg.click("#opbtn")
            pg.wait_for_selector("#start", state="hidden", timeout=90000)
        step("Load data starts", opens)
finally:
    r.close(); shutil.rmtree(tmp, ignore_errors=True)
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("OK   " if s == "ok" else "FAIL ") + n + ("" if s == "ok" else "  -> " + s))
errs = [e for e in r.errs if e[0] in ("pageerror",)]
print("page errors:", errs)
sys.exit(1 if bad or errs else 0)
