"""Flow, the «Import data» card (docs/agenti/flow-grafica.md): it says what the Data tab holds (no file / N files), shows no stray «null» and no «Ready.».
Run from the repo root: python3 tests_e2e/e2e_flow_import.py"""
import subprocess, sys, tempfile, socket
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
from lib import free_port, attendi; PORT = free_port()
subprocess.run([sys.executable, str(ROOT / "tools" / "prepara_sito.py")], check=True)      # the site with the Rust engine (the grid asks it for the CSV and the checks)
site = ROOT / "site"
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(2000):
    try: socket.create_connection(("127.0.0.1", PORT), timeout=1).close(); break
    except OSError: pass
bad = []
def check(name, ok, info=""):
    if not ok: bad.append(f"{name}: {info}")
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1400, "height": 1000}).new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.click('#nav button[data-v="flow"]'); attendi(pg, "document.querySelectorAll('.fln').length === 2")
        pg.click('.fln:has-text("Import data")'); attendi(pg, "document.querySelector('.fl-insp h3') && document.querySelector('.fl-insp h3').textContent === 'Import data'")
        t = pg.inner_text(".fl-insp")
        check("no file", "No file loaded" in t and pg.locator(".fl-insp button:has-text('Open Data')").count() == 1, t)
        check("no filler", "null" not in t.split() and "Ready." not in t and "What this step does" not in t and "Feeds into" not in t, t)
        attendi(pg, "document.querySelectorAll('.flx-imp .fl-b').length > 2")      # the late modules (the card's table) arrive after the first draw
        check("buttons", pg.locator(".flx-imp button:has-text('Add files')").count() == 1 and pg.locator(".flx-imp button:has-text('Add a folder')").count() == 1)
        check("generate off", pg.locator(".flx-imp button:has-text('Generate metadata')").is_disabled())
        pg.set_input_files(".flx-imp input[type=file]", files=[{"name": "meta.csv", "mimeType": "text/csv", "buffer": b"File,Role,Time (min)\na.mzML,sample,10\nb.mzML,blank,\n"}])
        attendi(pg, "document.querySelectorAll('.flx-imp .mg-bd [role=row]').length === 2")
        cell = lambda col, row: pg.locator(f".flx-imp .mg-bd [role=row]:nth-child({row}) [aria-colindex='{col}']")
        check("table", cell(8, 1).inner_text() == "10" and "2 rows read from meta.csv" in pg.inner_text(".flx-imp"), pg.inner_text(".flx-imp"))
        import io, zipfile
        xb = io.BytesIO()
        with zipfile.ZipFile(xb, "w", zipfile.ZIP_DEFLATED) as zf:
            zf.writestr("xl/sharedStrings.xml", "<sst><si><t>File</t></si><si><t>Series</t></si><si><t>a.mzML</t></si><si><t>S9</t></si></sst>")
            zf.writestr("xl/worksheets/sheet1.xml", '<worksheet><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1" t="s"><v>1</v></c></row><row r="2"><c r="A2" t="s"><v>2</v></c><c r="B2" t="s"><v>3</v></c></row></sheetData></worksheet>')
        pg.set_input_files(".flx-imp input[type=file]", files=[{"name": "meta.xlsx", "mimeType": "application/octet-stream", "buffer": xb.getvalue()}])
        attendi(pg, "/rows read from meta.xlsx/.test(document.querySelector('.flx-imp').textContent)")
        check("xlsx", cell(6, 1).inner_text() == "S9", pg.inner_text(".flx-imp"))
        pg.evaluate("E.files.push({file: 'a.mzML', name: 'a'}, {file: 'b.mzML', name: 'b'})")
        pg.click('.fln:has-text("Build feature table")'); pg.click('.fln:has-text("Import data")')
        attendi(pg, "!!document.querySelector('.fl-insp .fl-data') && /in Data/.test(document.querySelector('.fl-insp .fl-data').textContent)")
        t = pg.inner_text(".fl-insp")
        check("files", "2 files in Data: a.mzML, b.mzML" in t and "No file loaded" not in t, t)
        # the card states: yellow without data, green with them; one views selector; the template toggle; the divider widens the settings
        check("state", pg.locator(".fln:has-text('Import data') .fln-chip.ok").count() == 1 and "Runs" in pg.inner_text(".fln:has-text('Import data')"))
        pg.evaluate("E.files.length = 0"); pg.click('.fln:has-text("Build feature table")'); pg.click('.fln:has-text("Import data")')
        check("warn", pg.locator(".fln:has-text('Import data') .fln-chip.warn").count() == 1 and "No data" in pg.inner_text(".fln:has-text('Import data')"))
        check("views", pg.locator(".flx-views button").count() == 2 and pg.locator(".flx-views [aria-pressed=true]").count() == 1)
        attendi(pg, "!!document.querySelector('.flx-tpl.show')")      # shown on the first screen: the button hides it, then shows it again
        pg.click('button[aria-label="Start from a template"]'); attendi(pg, "!document.querySelector('.flx-tpl.show')")
        pg.click('button[aria-label="Start from a template"]'); attendi(pg, "!!document.querySelector('.flx-tpl.show')")
        w0 = pg.evaluate("document.querySelector('.flx-insp').getBoundingClientRect().width")
        pg.focus(".flx-split"); pg.keyboard.press("ArrowLeft"); pg.keyboard.press("ArrowLeft")
        check("split", pg.evaluate("document.querySelector('.flx-insp').getBoundingClientRect().width") > w0 + 50, str(w0))
        pg.click('button[data-tab="list"]'); pg.click('button[data-tab="canvas"]')
        pg.click('#flx-cli-show'); attendi(pg, "!!document.querySelector('.fl-cli textarea')")
        check("cli box", pg.locator(".fl-cli textarea").input_value().startswith("#") or "mzlab-cli" in pg.locator(".fl-cli textarea").input_value(), pg.locator(".fl-cli textarea").input_value()[:80])
        pg.fill(".fl-cli textarea", "echo edited"); check("cli box editable", pg.locator(".fl-cli textarea").input_value() == "echo edited")
        pg.click('#flx-cli-show'); check("arrow hides", pg.locator(".fl-cli").count() == 0)
        check("no molecules button", pg.locator("#mol-btn").count() == 0)
        b.close()
finally:
    srv.terminate()
print("FAIL\n" + "\n".join(bad) if bad else "OK e2e_flow_import")
sys.exit(1 if bad else 0)
