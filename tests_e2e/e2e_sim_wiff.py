"""The manual integrations of SIM traces on the built site with the Rust engine (synthetic SIM .wiff of tools/wiff_sim.py, `sim3`: a blank and three named
samples, seven ions at a 0.8 s cycle, an isomer pair in one channel; no laboratory data): dragging the pair with an msconvert log opens four files (the
log is skipped with a note); windows integrated on the ion traces fill the card «Manual integrations», one row per window grouped by channel; the card
and its CSV export (with and without the sum rows) and «Copy table» (tab-separated) are the bytes of `mzlab-cli manual-table` on the same .wiff and windows (after the provenance line);
a row's Delete removes the integration; a window drawn on one sample stays drawn when another sample is chosen in the left panel, measured on that sample with no row added; the channels of a sample equal `mzlab-cli channels`. The copy whose .wiff.scan is cut at 60 % measures the
points before the cut. Needs the site and the .wasm (python3 tools/prepara_sito.py) and cargo for the CLI. Run from the repo root: python3 tests_e2e/e2e_sim_wiff.py"""
import json, shutil, subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

if not shutil.which("cargo"):
    print("SKIP cargo is needed for mzlab-cli manual-table"); sys.exit(0)
D = Path(tempfile.mkdtemp(prefix="sim_wiff_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_sim.py"), str(D)], check=True, capture_output=True)
(D / "sim3.wiff.log").write_text("msconvert log (synthetic)\n")
TRUE = json.loads((D / "sim3.json").read_text())
CLI = ["cargo", "run", "-q", "-p", "mzlab-cli", "--"]
cli = lambda *a: subprocess.run(CLI + [str(x) for x in a], cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout
CHANNELS = json.loads(cli("channels", D / "sim3.wiff"))
throw = lambda m: (_ for _ in ()).throw(AssertionError(m))
SHOWN = "(() => { const c = document.querySelector('#manint'); return !!c && !c.hidden && c.querySelectorAll('tbody tr').length === %d; })()"

# integrates the window [a, b] (min) of ion `ion` (place in the list of ions) of sample `name` on the SIM panel, as a drag of the user ends (addInt, no copy to other panels)
INTEG = """([name, ion, a, b, bm]) => { const p = E.panels.find(q => q.type === 'mrm' && q._a);
  const f = E.files.find(x => x.label === name || x.file.startsWith(name + '.')); const s = p._a.sr.filter(x => x.k === f.k)[ion];
  addInt(p, s, a, b, false); if (bm) { const it = p.ints[p.ints.length - 1], q = bm === "zero" ? "zero" : "curve"; it.snap = { a: q, b: q }; it.st = E.seq++; draw(p); } return s.key; }"""
# the anchor buttons (.iep) of integration `id` on the SIM panel: [end, x, y, mode, hidden] in page pixels
ANCH = """id => E.panels.find(q => q.type === 'mrm' && q._a)._ep.filter(b => b._it.id === id).map(b => { const r = b.getBoundingClientRect(); return [b._e, r.x + r.width / 2, r.y + r.height / 2, b.dataset.m, b.hidden, r.width, r.height]; })"""
REC = "id => { const i = E.panels.find(q => q.type === 'mrm').ints.find(x => x.id === id); return i && JSON.parse(JSON.stringify({ snap: i.snap, my: i.my || null, bm: i.bm, bl: i.bl || null, st: i.st || 0, a: i.a, b: i.b })); }"

def manual(st, files, sums=False):
    # the page numbers the files it sends; the CLI reads every sample of the .wiff, the blank first
    order = [n[:-5] for n in st["names"]]
    ws = [{**x, "file": TRUE["campioni"].index(order[x["file"]])} for x in json.loads(st["windows"])]
    w = D / "windows.json"; w.write_text(json.dumps(ws))
    return cli("manual-table", *files, "--set", f"windows={w}", *(["--set", "sums=true"] if sums else []))

def download(pg):
    with pg.expect_download() as d:
        pg.click("#manint [data-dl]")
    text = Path(d.value.path()).read_text(encoding="utf-8")
    return text.split("\n", 1)[1] if text.startswith("# mzLab ") else text

def all_ions(pg):
    pg.evaluate("(() => { E.browse = false; E.files.forEach(f => { f.vis = true; }); const p = E.panels.find(q => q.type === 'mrm'); p.tr = ''; draw(p); })()")
    wait_for_js(pg, "(() => { const p = E.panels.find(q => q.type === 'mrm' && q._a); return !!p && new Set(p._a.sr.map(s => s.k)).size === E.files.length; })()", timeout=60000)

r = Run(port=8876, wd="/tmp/wd_sim_wiff")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.context.grant_permissions(["clipboard-read", "clipboard-write"])
        stage(pg, [D / "sim3.wiff", D / "sim3.wiff.scan", D / "sim3.wiff.log"]); pg.click("text=Carica dati"); ready(pg); wait_for_js(pg, "E.files.length >= 4", timeout=120000)
        open_first(pg)
        pg.click("#np-mrm"); wait_for_js(pg, "E.panels.some(p => p.type === 'mrm' && p._a)", timeout=60000)
        all_ions(pg)
        pk = TRUE["picchi"]
        def rows():
            need = lambda c, m: c or throw(m)
            pg.evaluate(INTEG, ["Sample_C", 0, pk[3][0][0] - 0.15, pk[3][0][0] + 0.15, None])
            pg.evaluate(INTEG, ["Sample_A", 0, pk[1][0][0] - 0.15, pk[1][0][0] + 0.15, "edges"])
            pg.evaluate(INTEG, ["Sample_B", 2, pk[2][2][0] - 0.15, pk[2][2][0] + 0.15, None])
            wait_for_js(pg, SHOWN % 3, timeout=60000)
            cells = pg.evaluate("[...document.querySelectorAll('#manint tbody tr')].map(r => [...r.children].slice(0, 3).map(c => c.textContent))")
            need(cells == [["253.1", "ionA", "Sample_A"], ["253.1", "ionA", "Sample_C"], ["251.1", "ionC", "Sample_B"]], cells)
            need(pg.evaluate("document.querySelector('#manint h3').textContent") == pg.evaluate("I18N.t('canali.manint.title', {n: 3})"), "title")
            need(pg.evaluate("[...document.querySelectorAll('#manint tbody button')].every(b => b.getBoundingClientRect().height >= 24 && b.getAttribute('aria-label'))"), "24 px, named")
        step("windows on the ion traces fill the card, grouped by channel then sample", rows)
        def parity():
            st = pg.evaluate("MANINT.state()")
            files = [D / "sim3.wiff"]
            want = manual(st, files)
            assert st["csv"] == want, (st, want)
            assert download(pg) == want
            pg.click("#manint [data-copy]")
            wait_for_js(pg, "document.querySelector('#manint [data-ok]').textContent === I18N.t('canali.manint.copied')", timeout=10000)
            got = pg.evaluate("navigator.clipboard.readText()")
            assert got.startswith("# mzLab ") and got.split("\n", 1)[1] == want.replace(",", "\t"), got
            pg.click("#manint [data-sums]")
            wait_for_js(pg, "MANINT.state().sums && /,sum,/.test(MANINT.state().csv)", timeout=60000)
            want = manual(st, files, sums=True)
            assert pg.evaluate("MANINT.state().csv") == want
            assert download(pg) == want
            assert pg.evaluate("document.querySelectorAll('#manint tr.manint-sum').length") == 2
            pg.click("#manint [data-sums]"); wait_for_js(pg, "!MANINT.state().sums && !/,sum,/.test(MANINT.state().csv)", timeout=60000)
        step("the table and its CSV (with and without the sums) are the bytes of mzlab-cli manual-table", parity)
        def row_actions():
            need = lambda c, m: c or throw(m)
            need(pg.evaluate("[...document.querySelectorAll('#manint tbody tr[data-go]')].every(r => r.tabIndex === 0 && r.getAttribute('aria-label') && r.querySelectorAll('button').length === 1)"), "a row is one focusable control with one X")
            x = pg.evaluate("(() => { const b = document.querySelector('#manint tbody [data-del]'), r = b.getBoundingClientRect(); return [r.width, r.height, b.title, b.textContent.trim(), !!document.querySelector('#manint [data-go] button:not([data-del])')]; })()")
            need(x[0] >= 24 and x[1] >= 24 and x[2] == pg.evaluate("I18N.t('canali.manint.del')") and x[3] == "×" and not x[4], x)
            pg.evaluate("(() => { window.__show = []; const o = window.reveal; window.reveal = p => { window.__show.push(p.id); return o(p); }; window.__orev = o; })()")
            pg.click("#manint tbody tr[data-go] td:nth-child(4)")
            pg.focus("#manint tbody tr[data-go]"); pg.keyboard.press("Enter"); pg.keyboard.press(" ")
            n = pg.evaluate("(() => { const n = window.__show.length; window.reveal = window.__orev; return n; })()")
            need(n == 3, n)
            wait_for_js(pg, SHOWN % 3, timeout=10000)
        step("a row shows its integration (click, Enter, Space) and its X is the only button", row_actions)
        def delete():
            pg.click("#manint tbody tr:nth-child(2) [data-del]")
            wait_for_js(pg, SHOWN % 2, timeout=60000)
            assert pg.evaluate("E.panels.find(q => q.type === 'mrm').ints.length") == 2
        step("Delete removes the integration and its row", delete)
        def follow():
            need = lambda c, m: c or throw(m)
            kof = lambda n: pg.evaluate("n => E.files.find(x => x.label === n || x.file.startsWith(n + '.')).k", n)
            ka, kb = kof("Sample_A"), kof("Sample_B")
            def only(k):                                     # the left panel: tick the sample, then untick the others (a first click may only close a popup)
                for _ in range(3):
                    if not pg.evaluate(f"E.files[{k}].vis"): pg.click(f"#flst .fl:not(.pr) > input[data-k='{k}']"); frames(pg, 2)
                    for x in pg.evaluate("E.files.filter(f => f.vis && !f.gone).map(f => f.k)"):
                        if x != k: pg.click(f"#flst .fl:not(.pr) > input[data-k='{x}']"); frames(pg, 2)
                    if pg.evaluate(f"E.files.every(f => f.gone || f.vis === (f.k === {k}))"): break
                wait_for_js(pg, f"(() => {{ const p = E.panels.find(q => q.type === 'mrm' && q._a); return !!p && p._a.sr.length > 0 && p._a.sr.every(s => s.k === {k}) && (p._a.lbls || []).some(l => l.int); }})()", timeout=60000)
            LB = "E.panels.find(q => q.type === 'mrm' && q._a)._a.lbls.filter(l => l.int).map(l => ({ text: l.text, tip: l.tip, fol: !!l.int.fol, ion: l.int.ion, a: l.int.a, b: l.int.b, area: l.int.area, key: l.int.key }))"
            table = lambda: pg.evaluate("[...document.querySelectorAll('#manint tbody tr')].map(r => r.textContent)")
            before = table()
            only(ka)
            la = [l for l in pg.evaluate(LB) if not l["fol"]]
            need(len(la) == 1 and la[0]["key"].startswith(f"m|{ka}|"), la)
            need(la[0]["text"].startswith("A = ") and "(" not in la[0]["text"], la[0]["text"])
            need(pg.evaluate("I18N.t('int.base.tag.edges')") in la[0]["tip"], "the baseline mode is in the hover text")
            only(kb)
            fb = [l for l in pg.evaluate(LB) if l["fol"]]
            need(len(fb) == 1 and fb[0]["ion"] == la[0]["ion"] and fb[0]["key"].startswith(f"m|{kb}|"), fb)
            need(abs(fb[0]["a"] - la[0]["a"]) < 1e-9 and abs(fb[0]["b"] - la[0]["b"]) < 1e-9, (fb, la))
            need(fb[0]["text"] not in ("A = —", la[0]["text"]) and fb[0]["area"] > 0 and fb[0]["area"] != la[0]["area"], (fb, la))
            need(not any(l["fol"] and l["ion"] == x["ion"] for l in pg.evaluate(LB) for x in pg.evaluate(LB) if not x["fol"]), "a committed window is shown as it is")
            wait_for_js(pg, LB + ".some(l => l.fol && !l.tip.includes('…') && l.tip.includes('S/N'))", timeout=60000)
            frames(pg, 3); need(table() == before, "a follower adds no row")
            none = pg.evaluate("SEGUE.all({ type: 'mrm', ints: [{ id: 1, ion: 'x', key: 'm|0|x', a: 50, b: 51, bm: 'edges' }] }, [{ key: 'm|1|x', ion: 'x', k: 1, name: 'n', x: [1, 2, 3] }]).filter(i => i.fol).map(i => i.none)")
            need(none == [True], none)
            only(ka)
            need([l for l in pg.evaluate(LB) if not l["fol"]] == la and table() == before, "the committed row of A is unchanged")
            all_ions(pg)
        step("a manual window follows the sample chosen in the left panel, measured on that sample, with no row added", follow)
        def anchors():
            need = lambda c, m: c or throw(m)
            P = "E.panels.find(q => q.type === 'mrm' && q._a)"
            a0 = pk[1][0][0]
            need(pg.evaluate(f"{P}.el.querySelectorAll('.ibh').length === 0 && !{P}.ints.some(i => !i.snap)"), "SIM/MRM: no bars, every integration is two anchors")
            n0 = pg.evaluate(f"{P}.ints.length")
            pg.evaluate(INTEG, ["Sample_A", 0, a0 + 0.3, a0 + 0.6, None])
            ints = pg.evaluate(f"{P}.ints.filter(i => i.key.startsWith('m|' + E.files.find(x => x.label === 'Sample_A' || x.file.startsWith('Sample_A.')).k + '|')).map(i => i.id)")
            need(pg.evaluate(f"{P}.ints.length") == n0 + 1 and len(ints) == 2, ("a click on another peak adds an integration", ints))
            wait_for_js(pg, SHOWN % (n0 + 1), timeout=60000)
            pg.evaluate(f"(() => {{ const p = {P}; p.zoom = [{a0 - 0.4}, {a0 + 1.4}]; draw(p); }})()")   # close anchors of two windows stand for each other: zoom in
            frames(pg, 3); new = ints[-1]
            an = pg.evaluate(ANCH, new)
            need(len(an) == 2 and all(x[5] >= 24 and x[6] >= 24 and x[3] == "curve" for x in an), an)
            need(pg.evaluate(REC, new)["bm"] == "edges", "a new integration follows the curve at both ends")
            pg.evaluate("STORIA.annulla()"); wait_for_js(pg, f"{P}.ints.length === {n0}", timeout=10000)
            pg.evaluate("STORIA.ripeti()"); wait_for_js(pg, f"{P}.ints.length === {n0 + 1}", timeout=10000); frames(pg, 3)
            new = pg.evaluate(f"{P}.ints[{P}.ints.length - 1].id")
            # S cycles the mode of the focused anchor: curve -> zero; a mixed pair is the line through the two anchors
            pg.evaluate(f"{P}._ep.find(b => b._it.id === {new} && b.dataset.e === '0').focus()"); pg.keyboard.press("s")
            v = pg.evaluate(REC, new); lo = "a" if v["a"] <= v["b"] else "b"
            need(v["snap"][lo] == "zero" and v["bm"] == "line" and v["bl"][1] == 0 and v["st"] > 0, v)
            title = pg.evaluate(f"document.activeElement.title")
            need(pg.evaluate("I18N.t('int.snap.zero')") in title and pg.evaluate("document.activeElement.getAttribute('aria-valuenow')") in title and pg.evaluate("I18N.sciE(0)") in title, title)
            # the menu of the anchor (Shift+F10): three radio items, the checked one first in focus; ↓ Enter picks «Manual»
            pg.keyboard.press("Shift+F10"); wait_for_js(pg, "document.querySelectorAll('.iep-menu [role=menuitemradio]').length === 3", timeout=5000)
            need(pg.evaluate("document.activeElement.dataset.m === 'zero' && document.activeElement.getAttribute('aria-checked') === 'true'"), "focus on the checked mode")
            pg.keyboard.press("ArrowDown"); pg.keyboard.press("Enter")
            wait_for_js(pg, "!document.querySelector('.iep-menu') && document.activeElement.classList.contains('iep')", timeout=5000)
            v = pg.evaluate(REC, new); need(v["snap"][lo] == "man" and abs(v["my"][lo]) < 1e-9, v)
            pg.keyboard.press("ArrowUp"); v2 = pg.evaluate(REC, new); need(v2["my"][lo] > 0 and v2["bl"][1] == v2["my"][lo], (v, v2))
            pg.evaluate("STORIA.annulla()"); need(pg.evaluate(REC, new)["my"][lo] == v["my"][lo], "one undo per key")
            # dragging the other anchor (on the curve) 40 px up makes it manual at the pointer's level
            hi = "b" if lo == "a" else "a"; e1 = [x for x in pg.evaluate(ANCH, new) if x[0] == hi][0]
            pg.mouse.move(e1[1], e1[2]); pg.mouse.down(); pg.mouse.move(e1[1], e1[2] - 20, steps=3); pg.mouse.move(e1[1], e1[2] - 40, steps=3); pg.mouse.up()
            v = pg.evaluate(REC, new); need(v["snap"][hi] == "man" and v["my"][hi] > 0 and v["bm"] == "line", v)
            need(pg.evaluate(f"{P}.isel") == new and pg.evaluate(f"{P}._ep.filter(b => b._it.id === {new}).every(b => b.classList.contains('sel'))"), "the integration in hand is the selected one")
            pg.evaluate(f"draw({P})"); frames(pg, 2)
            need(pg.evaluate(f"{P}._ep.filter(b => b._it.id !== {new}).every(b => !b.classList.contains('sel'))"), "only one selected")
            # a new integration inherits the last mode the user chose: both anchors manual at the same levels
            pg.evaluate(INTEG, ["Sample_A", 0, a0 + 0.8, a0 + 1.1, None]); w = pg.evaluate(f"{P}.ints[{P}.ints.length - 1].id")
            u = pg.evaluate(REC, w); need(u["snap"] == v["snap"] and u["my"] == v["my"] and u["bm"] == "line", (u, v))
            wait_for_js(pg, SHOWN % (n0 + 2), timeout=60000)
            rows = pg.evaluate("[...document.querySelectorAll('#manint tbody tr')].map(r => r.textContent)")
            need(sum(pg.evaluate("I18N.t('int.base.line')") in x for x in rows) >= 2, rows)
            # «Clear integrations» empties the panel; undo brings every integration back
            k = pg.evaluate(f"{P}.ints.length"); pg.evaluate(f"{P}.el.querySelector('[data-a=iclr]').click()")
            wait_for_js(pg, f"{P}.ints.length === 0", timeout=10000)
            pg.evaluate("STORIA.annulla()"); wait_for_js(pg, f"{P}.ints.length === {k}", timeout=10000)
            pg.evaluate(f"(() => {{ const p = {P}; p.zoom = null; draw(p); }})()")
            all_ions(pg)
        step("SIM/MRM: an integration is two anchors (curve, zero, manual: S, menu, drag), several per trace, the last mode inherited, undo", anchors)
        step("the msconvert log is skipped with a note", lambda: (lambda u: "sim3.wiff.log" in u or throw(u))(pg.evaluate("document.getElementById('unopen').textContent")))
        def channels():
            k = pg.evaluate("E.files.find(f => f.label === 'Blank' || /Blank/.test(f.file)).k")
            got = pg.evaluate(f"fetch('api/canali?k={k}').then(r => r.json())")
            assert got["canali"] == CHANNELS["canali"], (got, CHANNELS)
        step("the channels of a sample are those of mzlab-cli channels", channels)
        step("the engine answered every request", lambda: (lambda m: m and throw(m))(pg.evaluate("window.MZLAB_LR_MISS || []")))
        step("no page error", lambda: (lambda e: e and throw(e))([x for x in r.errs if x[0] == "pageerror"]))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass

r2 = Run(port=8877, wd="/tmp/wd_sim_wiff_cut")
try:
    with sync_playwright() as p:
        pg = r2.page(p)
        stage(pg, [D / "sim3_cut.wiff", D / "sim3_cut.wiff.scan"]); pg.click("text=Carica dati"); ready(pg); wait_for_js(pg, "E.files.length >= 3", timeout=120000)
        open_first(pg)
        pg.click("#np-mrm"); wait_for_js(pg, "E.panels.some(p => p.type === 'mrm' && p._a)", timeout=60000)
        all_ions(pg)
        def cut():
            a = TRUE["picchi"][1][0][0]
            pg.evaluate(INTEG, ["Sample_A", 0, a - 0.15, a + 0.15, None])
            wait_for_js(pg, SHOWN % 1, timeout=60000)
            st = pg.evaluate("MANINT.state()")
            assert st["csv"] == manual(st, [D / "sim3_cut.wiff"]), st
        step("a .wiff.scan cut at 60 %: a window before the cut is the CLI's", cut)
        step("no page error (cut file)", lambda: (lambda e: e and throw(e))([x for x in r2.errs if x[0] == "pageerror"]))
    r2.close()
except Exception as e:
    steps.append(("run cut", "FAIL " + str(e)[:300]))
    try: r2.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report(); r2.report()
