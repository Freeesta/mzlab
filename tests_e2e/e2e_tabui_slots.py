"""Tabui, the «Slots» tab of an open row (TP_Mine/js/48_tp_slots.js, `TPSLOTS`): fed with a synthetic `feature-slots/1` (no wasm needed), it lays the seven slots out in a fixed order
(three columns, one under 600 px), says filled / empty / not applicable in words, names the step and the command that fill an empty slot, lists the source files with their SHA-256 for a filled one,
and reuses the charts of TPRIGA. Checked at 390 px without horizontal scroll, in the dark theme, with reduced motion and with axe. Run from the repo root: python3 tests_e2e/e2e_tabui_slots.py"""
import os, subprocess, sys, tempfile, urllib.request
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402
from lib import open_mzfinder  # noqa: E402
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:700]))

tmp = Path(tempfile.mkdtemp()); site = tmp / "site"
reuse = os.environ.get("MZLAB_PIPE_SITE")
if reuse: site = Path(reuse)
else: subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(100):
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1).read(); break
    except Exception: __import__("threading").Event().wait(0.1)
errs = []
SHA = "0" * 64
def slot(name, state, ev=None, reason=None, src=None, fill=None): return {"slot": name, "state": state, "reason": reason, "evidence": ev, "source": src, "fill": fill}
SRC = lambda step, n=2: {"output": "table.mzfeat", "step": step, "files": [{"name": f"S{i:02d}.mzML", "sha256": SHA} for i in range(1, n + 1)]}
DOC = {"schema": "feature-slots/1", "id": 7, "mz": 268.1, "rt": 5.42, "output": "table.mzfeat", "slots": [
    slot("structure", "empty", fill={"step": "tp.generate", "command": None}),
    slot("xic", "filled", {"files": 2, "detected": 1, "filled": 1, "targeted": 0, "absent": 0, "na": 0, "rt_lo": 5.3, "rt_hi": 5.6}, src=SRC("features.align")),
    slot("kinetics", "na", reason="no_time_axis", fill={"step": None, "command": "mzlab-cli sample table"}),
    slot("ms2", "empty", fill={"step": "features.ms2_link", "command": "mzlab-cli spectrum list"}),
    slot("isotopes", "filled", {"iso_m1": 0.139, "adduct": "[M+H]+", "family_size": None}, src=SRC("features.group", 0)),
    slot("uv", "empty", fill={"step": None, "command": "mzlab-cli dad spectrum"}),
    slot("sim", "na", reason="not_a_transition_table", fill={"step": "chrom.transitions", "command": None})]}
MODEL = {"missing": {}, "apexRt": 5.42, "lo": 5.3, "hi": 5.6, "xic": [{"file": "S01", "group": "treated", "time": 0, "rt": [5.0 + i * 0.05 for i in range(20)], "y": [100 * (1 - ((i - 9) / 9) ** 2) if 2 < i < 17 else 0 for i in range(20)]}],
         "spectrum": {"kind": "ms1", "mz": [268.1, 269.1], "y": [100, 14]}}
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        def fresh(scheme="light", width=1100):
            c = b.new_context(viewport={"width": width, "height": 900}, reduced_motion="reduce", color_scheme=scheme); pg = c.new_page()
            pg.on("pageerror", lambda e: errs.append(str(e)))
            pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');}catch(_){}")
            pg.goto(f"http://127.0.0.1:{PORT}/"); open_mzfinder(pg)
            pg.wait_for_function("window.TPSLOTS && window.TPRIGA && window.TPRIGA.parts", timeout=60000)
            pg.evaluate("() => { const d = document.createElement('div'); d.className = 'tp'; d.id = 'sl-host'; d.style.cssText = 'position:fixed;left:0;top:0;width:100%;background:var(--panel)'; const p = document.createElement('div'); p.id = 'sl-panel'; d.appendChild(p); document.body.appendChild(d); }")
            return pg
        def show(pg, doc=DOC): pg.evaluate("([d, m]) => TPSLOTS.fill(document.getElementById('sl-panel'), d, m)", [doc, MODEL])
        def cards(pg): return pg.evaluate("() => [...document.querySelectorAll('#sl-panel .tpsl-card')].map(c => c.dataset.slot + ':' + c.dataset.state)")
        pg = fresh(); show(pg)
        def order():
            got = cards(pg); assert got == ["structure:empty", "xic:filled", "kinetics:na", "ms2:empty", "isotopes:filled", "uv:empty", "sim:na"], got
        step("the seven slots in the fixed order, each with its state", order)
        def words():
            t = pg.inner_text("#sl-panel"); assert "✓ filled" in t and "○ empty" in t and "– not applicable" in t, t
            t = t.lower(); assert "the files have no time axis" in t and "this is not a transition table" in t, t
        step("the state is a word and a mark; a n/a slot says why", words)
        def fillwith():
            t = pg.inner_text("#sl-panel [data-slot=ms2]"); assert "features.ms2_link" in t and "mzlab-cli spectrum list" in t, t
            t = pg.inner_text("#sl-panel [data-slot=uv]"); assert "mzlab-cli dad spectrum" in t and "step" not in t, t
        step("an empty slot names the step and the command that fill it", fillwith)
        def source():
            t = pg.inner_text("#sl-panel [data-slot=xic]"); assert "features.align" in t and "table.mzfeat" in t, t
            assert pg.evaluate("() => document.querySelectorAll('#sl-panel [data-slot=xic] details li code').length") == 2
            assert pg.evaluate("() => document.querySelectorAll('#sl-panel [data-slot=isotopes] details').length") == 0
        step("a filled slot shows its source: output, step, the input files with SHA-256", source)
        step("the XIC slot reuses the chart of TPRIGA", lambda: pg.evaluate("() => document.querySelectorAll('#sl-panel [data-slot=xic] .tpr-ch svg').length") == 1 or (_ for _ in ()).throw(AssertionError("no chart")))
        def grid():
            cols = lambda w: pg.evaluate("() => getComputedStyle(document.querySelector('#sl-panel .tpsl')).gridTemplateColumns.split(' ').length")
            assert cols(1100) == 3
            pn = fresh(width=390); show(pn)
            assert pn.evaluate("() => getComputedStyle(document.querySelector('#sl-panel .tpsl')).gridTemplateColumns.split(' ').length") == 1
            w = pn.evaluate("() => [document.documentElement.scrollWidth, document.documentElement.clientWidth]"); assert w[0] <= w[1], w
        step("three columns, one under 600 px, no horizontal scroll at 390 px", grid)
        def dark():
            pd = fresh("dark"); show(pd); assert len(cards(pd)) == 7
            assert pd.evaluate("() => getComputedStyle(document.querySelector('#sl-panel .tpsl-card h4')).color") != "rgb(0, 0, 0)"
        step("dark theme draws the same tab", dark)
        AXE = lib.axe_path()
        def axe():
            if not pg.evaluate("!!window.axe"): pg.add_script_tag(path=AXE)
            v = pg.evaluate("axe.run(document.getElementById('sl-panel'), {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.nodes.length]))")
            assert not v, v
        if AXE: step("axe: no serious or critical violation", axe)
        else: print("SKIP axe-core not available")
        b.close()
finally:
    srv.terminate()
for n, r in steps: print(("PASS " if r == "ok" else "FAIL ") + n + ("" if r == "ok" else " :: " + r))
if errs: print("pageerrors:", errs[:3])
sys.exit(1 if any(r != "ok" for _, r in steps) or errs else 0)
