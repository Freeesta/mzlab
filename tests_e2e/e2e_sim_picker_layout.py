"""Layout of the SIM ion picker on the built site: no «Channel» header row; the PDA / DAD rows stand at the very top, the ions right below them under a slim divider that
also holds All / None (a file without a DAD has no divider and starts with the ions); the popup stays inside the screen at 1400 and 390 px. Synthetic SIM files
(tools/wiff_dad_mrm.py with a DAD, tools/wiff_sim.py without), no laboratory data. With MZLAB_SHOTS=<dir> it also writes the screenshots of the open picker at deviceScaleFactor 2.
Needs the site and the .wasm (python3 tools/prepara_sito.py). Run from the repo root: python3 tests_e2e/e2e_sim_picker_layout.py"""
# covers: mzlab/web/canali.js, mzlab/web/canali.css, tools/wiff_dad_mrm.py, tools/wiff_sim.py
import subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

D = Path(tempfile.mkdtemp(prefix="sim_picker_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_dad_mrm.py"), str(D / "dad")], check=True, capture_output=True)
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_sim.py"), str(D / "plain")], check=True, capture_output=True)
SHOTS = os.environ.get("MZLAB_SHOTS")
MRM = "E.panels.find(q => q.type === 'mrm')"
r = Run(port=8884, wd="/tmp/wd_sim_picker_layout")

def load(pg, files):
    pg.set_input_files("#pick", [str(f) for f in files])
    wait_for_js(pg, "document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=90000); pg.click("text=Load data"); ready(pg)
    wait_for_js(pg, "E.files.length >= 1 && E.files.every(f => f.sim > 0)", timeout=120000)
    open_kind(pg, "mrm"); wait_for_js(pg, f"!!{MRM} && !!{MRM}.el.querySelector('[data-o=canali]')", timeout=60000)

def open_picker(pg):
    pg.evaluate(f"{MRM}.el.scrollIntoView({{ block: 'start' }})")
    pg.evaluate(f"{MRM}._canOpen = true; ctl({MRM})"); wait_for_js(pg, f"!{MRM}.el.querySelector('.can-pop').hidden"); frames(pg, 3)

# what the checks read: the popup box, the screen, and the rows / divider / All-None in the order they stand on the screen
READ = """(() => { const p = %s, q = p.el.querySelector('.can-pop'), b = q.getBoundingClientRect(), W = document.documentElement.clientWidth, H = window.innerHeight, box = e => { const r = e.getBoundingClientRect(); return { t: r.top, b: r.bottom, l: r.left, r: r.right, h: r.height }; };
  return { l: b.left, r: b.right, t: b.top, bt: b.bottom, W, H, oy: getComputedStyle(q).overflowY, sh: q.scrollHeight, ch: q.clientHeight, hd: q.querySelectorAll('.can-hd').length, text: q.innerText,
    seq: [...q.querySelectorAll('.can-row,.can-tb')].map(e => ({ k: e.classList.contains('can-tb') ? 'tools' : e.dataset.kind ? 'kind:' + e.dataset.kind : 'ion', ...box(e) })), gh: [...q.querySelectorAll('.can-gh')].map(e => ({ text: e.textContent, ...box(e) })),
    btn: [...q.querySelectorAll('.can-all,.can-none')].map(e => box(e)) }; })()""" % MRM

def check(pg, tag, dad):
    d = pg.evaluate(READ)
    assert d["hd"] == 0 and "Channel" not in d["text"], "%s: the «Channel» header row is back: %s" % (tag, d["text"][:80])
    kinds = [x["k"] for x in d["seq"]]
    ions = [x for x in d["seq"] if x["k"] == "ion"]
    assert len(ions) >= 2 and kinds.count("tools") == 1, (tag, kinds)
    if dad:
        assert kinds[0] == "kind:pda" and kinds[1] == "tools" and set(kinds[2:]) == {"ion"}, "%s: order must be PDA, divider with All / None, ions: %s" % (tag, kinds)
        assert d["seq"][0]["t"] - d["t"] < 14, "%s: the PDA row is not at the very top: %s" % (tag, d["seq"][0]["t"] - d["t"])
        assert len(d["gh"]) == 1 and d["gh"][0]["text"] == "Mass spectrometry" and d["gh"][0]["h"] <= 28, "%s: the group name is a slim divider: %s" % (tag, d["gh"])
    else:
        assert kinds[0] == "tools" and set(kinds[1:]) == {"ion"} and not d["gh"], "%s: without a DAD the ions follow All / None, no group name: %s %s" % (tag, kinds, d["gh"])
    tools = next(x for x in d["seq"] if x["k"] == "tools")
    assert tools["h"] <= 34 and tools["b"] <= ions[0]["t"] + 1, "%s: All / None is a compact row above the ions: %s" % (tag, tools)
    assert len(d["btn"]) == 2 and all(x["r"] - x["l"] >= 24 and x["h"] >= 24 and x["l"] >= 0 and x["r"] <= d["W"] for x in d["btn"]), "%s: All / None: %s" % (tag, d["btn"])
    assert d["l"] >= 0 and d["r"] <= d["W"], "%s: the popup runs off the screen: %s" % (tag, d)
    assert d["bt"] <= d["H"] + 1 or d["oy"] in ("auto", "scroll"), "%s: the popup runs below the screen and does not scroll: %s" % (tag, d)
    return d

AXE = axe_path()       # axe-core (axe-playwright-python): the axe steps run when it is installed, as in e2e_dad_mrm
def axe_ok(pg, sel):
    v = pg.evaluate("""(sel) => axe.run(document.querySelector(sel) || document, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}})
      .then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.nodes.length, v.nodes.slice(0, 3).map(n => n.target.join(' ')).join(' | ')]))""", sel)
    assert not v, v

def shot(pg, name):
    if not SHOTS: return
    Path(SHOTS).mkdir(parents=True, exist_ok=True)
    c = pg.evaluate(f"(() => {{ const b = {MRM}.el.querySelector('.canb').getBoundingClientRect(), q = {MRM}.el.querySelector('.can-pop').getBoundingClientRect(), x = Math.max(0, Math.min(b.left, q.left) - 12), y = Math.max(0, b.top - 12); return {{ x, y, width: Math.min(document.documentElement.clientWidth - x, Math.max(b.right, q.right) - x + 12), height: q.bottom - y + 12 }}; }})()")
    pg.screenshot(path=str(Path(SHOTS) / name), clip=c)

try:
    with sync_playwright() as p:
        for dad, files in ((True, [D / "dad" / f"dadsim{x}" for x in (".wiff", ".wiff.scan")]), (False, [D / "plain" / f"sim3{x}" for x in (".wiff", ".wiff.scan")])):
            word = "with a DAD" if dad else "without a DAD"
            pg = r.page(p, viewport={"width": 1400, "height": 900}, device_scale_factor=2)
            load(pg, files)
            if AXE: pg.add_script_tag(path=AXE)
            for w, h in ((1400, 900), (390, 844)):
                pg.set_viewport_size({"width": w, "height": h}); frames(pg, 4)       # loaded wide, then narrowed: the phone layout hides the data view behind the sidebar sheet while it opens
                open_picker(pg)
                step("SIM picker %s at %d px: no Channel header, %s, the popup fits the screen" % (word, w, "PDA / DAD first, ions below" if dad else "ions first"), lambda: check(pg, "%d px" % w, dad))
                if AXE: step("axe: the open SIM picker %s at %d px" % (word, w), lambda: axe_ok(pg, ".pnl.mrm .can-pop"))
                shot(pg, "sim-picker-%s-%d.png" % ("dad" if dad else "plain", w))
                if dad:
                    def keys():
                        pg.keyboard.press("Escape"); wait_for_js(pg, f"{MRM}.el.querySelector('.can-pop').hidden")
                        pg.keyboard.press("ArrowDown"); wait_for_js(pg, f"!{MRM}.el.querySelector('.can-pop').hidden && document.activeElement.classList.contains('can-row')")
                        pg.keyboard.press("Home"); assert pg.evaluate("document.activeElement.dataset.kind") == "pda"
                        pg.keyboard.press("ArrowDown"); assert pg.evaluate("document.activeElement.classList.contains('can-row') && !document.activeElement.dataset.kind"), "ArrowDown leaves the PDA row for the first ion"
                    step("keyboard at %d px: Home is the PDA row, ArrowDown the first ion (the buttons between are not rows)" % w, keys)
                pg.evaluate(f"{MRM}._canOpen = false; ctl({MRM})"); frames(pg, 2)
            pg.close()
        step("no page error", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r.errs if x[0] == "pageerror"]))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
