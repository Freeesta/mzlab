"""Seeded random sequences of list actions on synthetic .wiff pairs (tools/wiff_modi.py EPI / PI / MRM / Q1MI, tools/wiff_misto.py; no laboratory data).
After every action no panel on screen is blank: every MS2 pair draws a visible file that holds its precursor, a ticked precursor row is a drawn one, and the
MRM / SIM panel draws the channels of the files shown. Actions: row click, double click (only this file), file / group / precursor boxes, user groups,
the ‹ › arrows and the file chooser, remove + undo, undo, reload. Needs the site and the .wasm (python3 tools/prepara_sito.py).
Run from the repo root: python3 tests_e2e/e2e_ms2_invariante.py [seeds]"""
import json, random, subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

SEEDS = [int(s) for s in sys.argv[1].split(",")] if len(sys.argv) > 1 else [1, 2, 3]
N = 30
steps = []
D = Path(tempfile.mkdtemp(prefix="ms2_inv_"))
for tool in ("wiff_modi.py", "wiff_misto.py"):
    subprocess.run([sys.executable, str(ROOT / "tools" / tool), str(D)], check=True, capture_output=True)
FILES = [D / "misto.wiff", D / "misto.wiff.scan"] + [D / "modi" / f"{m}.wiff{x}" for m in ("EPI", "PI", "MRM", "Q1MI") for x in ("", ".scan")]

# the panels on screen that draw nothing, and the broken MS2 rules (a pair over a precursor no visible file holds, a ticked precursor row that is not drawn)
BROKEN = """() => {
  const out = [], live = f => f && f.vis && !f.gone, near = (f, v) => (f.precursors || []).some(x => Math.abs(x - Number(v)) < 0.6);
  for (const p of E.panels) {
    if (!p.el || !p.el.offsetParent) continue;
    const a = p._a; let why = null;
    if (!a) why = 'not drawn';
    else if (p.type === 'spec') { const d = a.data && a.data[0] && a.data[0].d; if (!d || !d.mz || !d.mz.length) why = 'spectrum without peaks'; }
    else if (p.type === 'chrom' || p.type === 'mrm') { if (!(a.sr || []).some(s => s.y && s.y.some(v => v > 0))) why = 'no trace'; }
    if (p.tab === 'ms2' && p.type === 'chrom' && p.prec != null && p.prec !== '' && !E.files.some(f => live(f) && f.kind === 'ms2' && near(f, p.prec) && (!p.ks || p.ks.includes(f.k))))
      why = (why ? why + '; ' : '') + 'no visible file holds its precursor';
    if (why) out.push(`${p.tab}/${p.type} ${p.prec ?? ''} ${p.type === 'mrm' ? JSON.stringify(p.tr) : ''}: ${why}`);
  }
  document.querySelectorAll('#flst input[data-pr]:checked').forEach(x => { const f = E.files[+x.dataset.pk];
    if (!live(f)) out.push(`precursor row ${x.dataset.pr} of ${f && f.file} ticked while its file is hidden`); });
  return out;
}"""
STATE = "E.files.filter(f => !f.gone).map(f => (f.vis ? '+' : '-') + f.file + (f.k === E.cur ? '*' : '') + (f.ugrp ? '@' + f.ugrp : ''))"
MOD = "Control"

def open_groups(pg):
    """Every group of the list open (the view choice of a student, not a state of the files): the rows can be clicked."""
    pg.evaluate("(() => { E._grpOpen = E._grpOpen || {}; E.files.forEach(f => { E._grpOpen[f.kind] = true; if (f.ugrp) E._grpOpen['u:' + f.ugrp] = true; f._prOpen = true; }); renderFileList(); })()")

def rows(pg):
    return pg.evaluate("E.files.filter(f => !f.gone).map(f => f.k)")

def act(pg, rnd, name):
    open_groups(pg)
    ks = rows(pg); k = rnd.choice(ks) if ks else None
    if name == "click" and k is not None: pg.locator(f"#flst .fl[data-k='{k}'] .nm").click(); return f"click {k}"
    if name == "solo" and k is not None: pg.locator(f"#flst .fl[data-k='{k}'] .nm").dblclick(); return f"solo {k}"
    if name == "tick" and k is not None: pg.locator(f"#flst input[data-k='{k}']").click(); return f"tick {k}"
    if name == "prec":
        n = pg.locator("#flst input[data-pr]").count()
        if n: i = rnd.randrange(n); d = pg.locator("#flst input[data-pr]").nth(i); t = d.get_attribute("data-pr") + "/" + d.get_attribute("data-pk"); d.click(); return "prec " + t
    if name == "group":
        n = pg.locator("#flst .gall").count()
        if n: i = rnd.randrange(n); g = pg.locator("#flst .gall").nth(i); t = g.get_attribute("data-g"); g.click(); return "group " + t
    if name == "ugroup" and len(ks) > 1:
        two = rnd.sample(ks, 2)
        pg.evaluate(f"fileViewDo(() => {{ E.files.forEach(f => {{ f.vis = {json.dumps(two)}.includes(f.k); }}); }}); groupFiles()"); return f"user group {two}"
    if name == "nav":
        if pg.locator("#fsel").is_disabled(): return "nav (one file)"
        if rnd.random() < 0.5: b = rnd.choice(["#fprev", "#fnext"]); pg.click(b); return "nav " + b
        o = pg.evaluate("[...document.querySelectorAll('#fsel option')].map(o => o.value)"); v = rnd.choice(o); pg.select_option("#fsel", v); return "chooser " + v
    if name == "remove" and len(ks) > 1:
        # the Undo button takes back the removals of the tab on screen (storia.js): a file of that tab
        k = rnd.choice(pg.evaluate("E.files.filter(f => !f.gone && f.kind === E.tab).map(f => f.k)") or [None])
        if k is None: return None
        pg.locator(f"#flst .fl[data-k='{k}'] .fl-x").click(); pg.wait_for_function(f"E.files[{k}].gone === true"); ready(pg)
        pg.click("#np-undo"); pg.wait_for_function(f"E.files[{k}].gone === false"); return f"remove + undo {k}"
    if name == "undo": pg.mouse.click(5, 5); pg.keyboard.press(f"{MOD}+z"); return "undo"
    if name == "reload": pg.reload(); pg.wait_for_function("typeof E !== 'undefined' && E.files.length > 0", timeout=90000); return "reload"
    return None

ACTS = ["click"] * 4 + ["solo"] * 3 + ["tick"] * 3 + ["prec"] * 3 + ["group", "ugroup", "nav", "nav", "remove", "undo"]

def run(seed):
    """One sequence on a fresh page: its failure names the seed, the step and the last actions, so it replays alone (python3 tests_e2e/e2e_ms2_invariante.py SEED)."""
    rnd, log = random.Random(seed), []
    r = Run(port=free_port(), wd=f"/tmp/wd_ms2_inv_{seed}")
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            stage(pg, FILES); open_first(pg); ready(pg); drawer(pg)
            for i in range(N):
                name = "reload" if i == N // 2 else rnd.choice(ACTS)
                what = name
                try:
                    what = act(pg, rnd, name)
                    if what is None: continue
                    ready(pg, timeout=60000); frames(pg, 4)
                    pg.wait_for_function(f"({BROKEN})().length === 0", timeout=8000)
                except Exception as e:
                    raise AssertionError({"seed": seed, "step": i, "after": log[-6:] + [what], "error": str(e)[:200], "broken": pg.evaluate(BROKEN), "files": pg.evaluate(STATE)}) from None
                log.append(what)
            bad = [e for e in r.errs if e[0] == "pageerror"]
            assert not bad, bad
            miss = pg.evaluate("window.MZLAB_LR_MISS || []"); assert not miss, miss
    finally:
        r.close()

for seed in SEEDS:
    try: run(seed); steps.append((f"seed {seed}: {N} actions, no blank panel, every MS2 pair over a drawn file that holds its precursor", "ok"))
    except Exception as e: steps.append((f"seed {seed}", "FAIL " + str(e)[:1500]))
bad = [n for n, s in steps if s != "ok"]
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if bad else 0)
