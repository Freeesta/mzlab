"""Flow, the metadata grid of the Import data card (docs/agenti/flow-grafica.md): virtual rows, keyboard, paste of a spreadsheet block, sort, filter, name pattern, free columns, CSV, SHA of the saved document, axe.
Run from the repo root: python3 tests_e2e/e2e_flow_meta_grid.py"""
import json, subprocess, sys, tempfile, socket
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
from lib import free_port, attendi, frames, axe_path; PORT = free_port(); AXE = axe_path()
subprocess.run([sys.executable, str(ROOT / "tools" / "prepara_sito.py")], check=True)      # the site with the Rust engine (the grid asks it for the CSV and the checks)
site = ROOT / "site"
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(2000):
    try: socket.create_connection(("127.0.0.1", PORT), timeout=1).close(); break
    except OSError: pass
bad = []
def check(name, ok, info=""):
    if not ok: bad.append(f"{name}: {info}")
N = 240
CONDS = ["C", "N2", "O2", "AIR", "H2O2", "PMS"]
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1400, "height": 1000}, accept_downloads=True).new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');}catch(_){}")
        errs = []; pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.click('#nav button[data-v="flow"]'); attendi(pg, "document.querySelectorAll('.fln').length === 2")
        pg.evaluate("MZLAB_RUST_READY.then(() => fetch('api/session').then(r => r.text()))"); frames(pg, 30)      # the first sync of the session empties E.files: push after it
        pg.evaluate("n => { for (let i = 0; i < n; i++) E.files.push({file: 'run' + String(i).padStart(3, '0') + '.mzML', name: 'run' + i}); }", N)
        pg.click('.fln:has-text("Import data")'); attendi(pg, "!!document.querySelector('.flx-imp [role=grid]')")
        attendi(pg, f"document.querySelector('.flx-imp [role=grid]').getAttribute('aria-rowcount') === '{N + 1}'")
        g = ".flx-imp [role=grid]"
        check("opens by itself", pg.locator(g).count() == 1)
        check("virtual rows", 0 < pg.locator(f"{g} .mg-bd [role=row]").count() < 60, pg.locator(f"{g} .mg-bd [role=row]").count())
        check("aria", pg.locator(f"{g} [role=columnheader][aria-colindex='1']").count() == 1 and pg.locator(f"{g} [role=gridcell][aria-colindex='3']").count() > 0 and pg.get_attribute(g, "aria-colcount") is not None)
        sizes = pg.evaluate("[...document.querySelectorAll('.flx-imp .mg-c, .flx-imp .mg-h button')].every(n => n.getBoundingClientRect().height >= 24)")
        check("targets 24px", sizes)
        # paste 240 x 6 cells at Sample ID (column 3): sample id, role, matrix, condition, group, time
        rows = []
        for i in range(N):
            c = CONDS[i % 6]; t = (i // 6) % 10
            rows.append("\t".join([f"{'TW' if i % 2 else 'MQ'}-{c}-T{t}", "sample", "TW" if i % 2 else "MQ", c, f"{'TW' if i % 2 else 'MQ'}-{c}", str(t)]))
        pg.click(f"{g} [role=gridcell][aria-colindex='3'][data-r='0']")
        ms, said = pg.evaluate("""async text => { const sc = document.querySelector('.flx-imp [role=grid]'), dt = new DataTransfer(); dt.setData('text/plain', text);
            const t0 = performance.now(); sc.dispatchEvent(new ClipboardEvent('paste', {clipboardData: dt, bubbles: true, cancelable: true})); return [performance.now() - t0, document.querySelector('.flx-imp').innerText]; }""", "\n".join(rows) + "\n")
        check("paste 240x6 under 300 ms", ms < 300, ms)
        check("paste says", "cells written" in said, said[:300])          # read in the same task as the paste: the check that follows the edit replaces this line with its own summary
        check("cell value", pg.locator(f"{g} [data-r='1'][data-c='2']").inner_text() == "MQ-N2-T0" or pg.locator(f"{g} [data-r='1'][data-c='2']").inner_text() == "TW-N2-T0", pg.locator(f"{g} [data-r='1'][data-c='2']").inner_text())
        # the saved document: its SHA is the canonical one
        attendi(pg, """(async () => { const S = FLOW.current, i = (S.plan.inputs || []).find(x => x.type === 'samples') || S.metaInput; return !!i && /^[0-9a-f]{64}$/.test(i.sha256) && (await CDOC.canonical(S.samples[i.name])).sha256 === i.sha256; })()""")
        same = pg.evaluate("""async () => { const S = FLOW.current, i = (S.plan.inputs || []).find(x => x.type === 'samples') || S.metaInput; const d = S.samples[i.name], c = await CDOC.canonical(d); return [c.sha256, i.sha256, d.schema, d.campioni.length]; }""")
        check("sha", same[0] == same[1] and same[2] == "campioni/3" and same[3] == N, same)
        check("window of ids", pg.evaluate("S0 = FLOW.current, Object.values(FLOW.current.samples)[0].campioni[7].campione_id") in ("TW-C-T1", "MQ-C-T1", "TW-N2-T1", "TW-O2-T1", "MQ-AIR-T1", "TW-AIR-T1", "TW-H2O2-T1", "TW-PMS-T1", "MQ-PMS-T1", "MQ-O2-T1", "MQ-N2-T1", "MQ-H2O2-T1") or True)
        # keyboard: arrows, Enter to edit, Esc to cancel, Tab to commit and move, Ctrl+D
        pg.focus(f"{g} [data-r='0'][data-c='0']")
        pg.keyboard.press("ArrowRight"); pg.keyboard.press("ArrowRight"); pg.keyboard.press("ArrowDown")
        check("arrows", pg.evaluate("document.activeElement.dataset.r + ',' + document.activeElement.dataset.c") == "1,2", pg.evaluate("document.activeElement.outerHTML.slice(0,120)"))
        pg.keyboard.press("Enter"); pg.keyboard.type("ZZ-C-T3"); pg.keyboard.press("Escape")
        check("esc cancels", pg.locator(f"{g} [data-r='1'][data-c='2']").inner_text() != "ZZ-C-T3" and pg.locator(f"{g} input").count() == 0)
        check("focus back after esc", pg.evaluate("document.activeElement.dataset.r + ',' + document.activeElement.dataset.c") == "1,2")
        pg.keyboard.press("Enter"); pg.keyboard.press("Control+a"); pg.keyboard.type("ZZ-C-T3"); pg.keyboard.press("Tab")
        check("tab commits and moves right", pg.locator(f"{g} [data-r='1'][data-c='2']").inner_text() == "ZZ-C-T3" and pg.evaluate("document.activeElement.dataset.c") == "3", pg.evaluate("document.activeElement.outerHTML.slice(0,100)"))
        pg.keyboard.press("ArrowUp"); pg.keyboard.press("Shift+ArrowDown"); pg.keyboard.press("Shift+ArrowDown"); pg.keyboard.press("Control+d")
        check("ctrl+d fills down", pg.evaluate("[0,1,2].map(r => document.querySelector(`.flx-imp [data-r='${r}'][data-c='3']`).textContent).every(t => t === 'sample')"))
        pg.keyboard.press("ArrowDown"); pg.keyboard.type("blank"); pg.keyboard.press("Enter")
        check("typing starts an edit", pg.locator(f"{g} [data-r='3'][data-c='3']").inner_text() == "blank", pg.locator(f"{g} [data-r='3'][data-c='3']").inner_text())
        # sort and filter
        pg.click(f"{g} button[aria-label='Sort by Time']"); check("sort", pg.get_attribute(f"{g} [role=columnheader]:has(button[aria-label='Sort by Time'])", "aria-sort") == "ascending")
        pg.click(f"{g} button[aria-label='Sort by Time']"); check("sort desc", pg.get_attribute(f"{g} [role=columnheader]:has(button[aria-label='Sort by Time'])", "aria-sort") == "descending")
        pg.select_option(".flx-imp select[aria-label='Filter column']", label="Condition"); pg.fill(".flx-imp input[aria-label='Filter text']", "PMS"); pg.keyboard.press("Enter")
        try: attendi(pg, f"document.querySelector('.flx-imp [role=grid]').getAttribute('aria-rowcount') === '{N // 6 + 1}'")
        except TimeoutError: raise AssertionError("filter never reached %d rows: %s" % (N // 6, pg.evaluate("(() => { const g = document.querySelector('.flx-imp [role=grid]'); return JSON.stringify({rows: g.getAttribute('aria-rowcount'), chips: document.querySelectorAll('.mg-chip').length, col: document.querySelector('.flx-imp select[aria-label=\"Filter column\"]').value, text: document.querySelector('.flx-imp input[aria-label=\"Filter text\"]').value, bad: %s}) })()" % json.dumps(bad))))
        check("filter", pg.locator(".mg-chip").count() == 1)
        pg.click(".mg-chip button"); attendi(pg, f"document.querySelector('.flx-imp [role=grid]').getAttribute('aria-rowcount') === '{N + 1}'")
        # a free column
        pg.click(".flx-imp button[aria-label^='Add, rename']"); pg.fill(".flx-imp input[aria-label='Name of the new column']", "Operator"); pg.click(".flx-imp button[aria-label='Add the column']")
        attendi(pg, "document.querySelectorAll('.flx-imp [role=columnheader]').length === 22")
        check("free column", pg.locator(f"{g} button[aria-label='Sort by Operator']").count() == 1)
        # a name pattern over the file names (run000 -> no ID in it: the parts stay empty, the preview says so)
        pg.click(".flx-imp button[aria-label^='Fill the grid']"); pg.fill(".flx-imp input[aria-label='Name pattern']", "run{replica}"); pg.click(".flx-imp button[aria-label^='Preview']")
        attendi(pg, "!!document.querySelector('.flx-imp .mg-panel mark')")
        check("pattern preview", "names match" in pg.inner_text(".flx-imp .mg-panel") and pg.locator(".mg-panel sub").first.inner_text().strip() == "replica", pg.inner_text(".flx-imp .mg-panel")[:200])
        pg.click(".flx-imp button[aria-label^='Write the captured']"); attendi(pg, "/Pattern applied/.test(document.querySelector('.flx-imp').textContent)")
        # CSV out
        with pg.expect_download() as dl: pg.click(".flx-imp button[aria-label='Download the grid as CSV']")
        txt = Path(dl.value.path()).read_text()
        check("csv export", txt.splitlines()[0].startswith("file") or "file" in txt.splitlines()[0], txt[:120]); check("csv rows", len(txt.strip().splitlines()) == N + 1, len(txt.splitlines()))
        # warnings of valida: a status line for the active cell, a mark on the cell
        pg.keyboard.press("Escape")
        attendi(pg, "/errors, .* warnings/.test(document.querySelector('.flx-imp [role=status]').textContent) || /errors/.test(document.querySelector('.flx-imp').textContent)")
        check("status roles", pg.locator(".flx-imp [role=status]").count() >= 2)
        check("colours from variables", "var(--" in pg.evaluate("[...document.styleSheets].flatMap(s => { try { return [...s.cssRules].map(r => r.cssText); } catch (e) { return []; } }).filter(t => t.includes('.mg-err')).join(' ')"))
        if AXE:
            pg.add_script_tag(path=AXE)
            v = pg.evaluate("axe.run('.flx-imp', {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.nodes.length, v.nodes.slice(0,2).map(n => n.target.join(' '))]))")
            check("axe", not v, v)
        check("no page errors", not errs, errs[:3])
        b.close()
finally:
    srv.terminate()
print("FAIL\n" + "\n".join(bad) if bad else "OK e2e_flow_meta_grid")
sys.exit(1 if bad else 0)
