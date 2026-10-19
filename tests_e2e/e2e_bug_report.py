"""«Report a bug» (header button #bug-btn, bug-report.js): the icon is in Dati, Disegno and Teoria; Enter opens the dialog, Esc closes it and gives the focus back, Tab stays inside;
the diagnostics hold NO name of a loaded file (a file with a recognisable name is loaded) nor of an error; «Copy report» fills the clipboard; «Open on GitHub» has the expected link (form bug.yml, label bug, <= 6000 characters).
Run from the repo root: python3 tests_e2e/e2e_bug_report.py"""
import os, shutil, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from urllib.parse import urlparse, parse_qs
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8871, wd="/tmp/wd_bug")
SECRET = "ZZSegretoRossi_campione"
tmp = Path("/tmp/wd_bug_in"); shutil.rmtree(tmp, ignore_errors=True); tmp.mkdir()
src = Path(mz(FILES[0])); named = tmp / (SECRET + ".mzML"); shutil.copy(src, named)
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.context.grant_permissions(["clipboard-read", "clipboard-write"], origin=f"http://127.0.0.1:{r.port}")
        assert pg.is_hidden("#bug-btn"), "the button is for the administrator mode only"
        # the administrator mode that shows it is tried in e2e_intestazione_unica; here it is shown by hand, also after a reload (language change)
        pg.context.add_init_script("document.addEventListener('DOMContentLoaded', () => { const b = document.getElementById('bug-btn'); if (b) b.hidden = false; })")
        pg.reload(); pg.wait_for_function('window.qqReady===true', timeout=60000)
        def icon():
            b = pg.locator("#bug-btn"); assert b.count() == 1 and b.is_visible()
            assert b.get_attribute("aria-label") == "Segnala un bug" and b.get_attribute("title") == "Segnala un bug" and b.locator("svg").count() == 1
            assert pg.evaluate("typeof window.BUGREP") == "undefined", "the dialog is lazy: not loaded before the first click"
        step("icon in the header with label and tooltip; dialog not loaded at start", icon)
        pg.set_input_files("#pick", [str(named)]); pg.wait_for_selector("text=Carica dati"); pg.click("text=Carica dati"); ready(pg)
        def opens():
            pg.evaluate("window.ERRORI && ERRORI.segnala(new Error('Cannot read %s.mzML and %s'), 'prova')" % (SECRET, SECRET))
            pg.locator("#bug-btn").focus(); pg.keyboard.press("Enter"); pg.wait_for_selector("#bugdlg:not([hidden])", timeout=10000)
            assert pg.evaluate("document.activeElement.id") == "bug-what" and pg.get_attribute("#bugdlg [role=dialog]", "aria-modal") == "true"
            t = pg.input_value("#bug-prev")
            assert "- mzLab:" in t and "- Mode: web" in t and "- Loaded files: 1 (mzML" in t and "- Service worker:" in t and "Recent errors:" in t, t
            assert SECRET not in t and "ZZ" not in t, "no name of the loaded file in the report"
            assert "<file>" in t, "the error line is kept, without the name"
        step("Enter opens the dialog; the diagnostics hold no file name", opens)
        def fields():
            pg.fill("#bug-what", "Il grafico " + SECRET + " non si apre"); pg.fill("#bug-exp", "che si apra")
            t = pg.input_value("#bug-prev"); assert "## Cosa è successo\n\nIl grafico" in t and "che si apra" in t
            pg.uncheck("#bug-diag"); t = pg.input_value("#bug-prev"); assert "Diagnostica" not in t and "- mzLab:" not in t
            pg.check("#bug-diag"); assert "- mzLab:" in pg.input_value("#bug-prev")
        step("fields and the diagnostics checkbox rebuild the text (the user's own words stay)", fields)
        def copy():
            pg.click("#bug-copy"); attendi(pg, "document.querySelector('#bug-st').textContent !== ''")
            clip = pg.evaluate("navigator.clipboard.readText()"); assert clip == pg.input_value("#bug-prev") and "## Cosa è successo" in clip
            assert pg.inner_text("#bug-st") == "Rapporto copiato"
        step("Copy report puts the whole text on the clipboard", copy)
        def link():
            a = pg.locator("#bug-go"); href = a.get_attribute("href"); u = urlparse(href); q = parse_qs(u.query)
            assert u.netloc == "github.com" and u.path == "/Freeesta/mzlab/issues/new" and q["template"] == ["bug.yml"] and q["labels"] == ["bug"], href
            assert q["title"][0].startswith("[Bug] Il grafico") and "che si apra" in q["cosa"][0] and len(href) <= 6000 and q["ambiente"][0].startswith("Mozilla/5.0")
            assert a.get_attribute("target") == "_blank" and "noopener" in a.get_attribute("rel") and a.inner_text() == "Apri su GitHub"
            pg.fill("#bug-what", "x" * 20000); h = pg.get_attribute("#bug-go", "href"); assert len(h) <= 6000 and pg.inner_text("#bug-st"), "long text: the link is cut and the dialog says so"
            pg.fill("#bug-what", "")
        step("Open on GitHub: expected link, new tab, never more than 6000 characters", link)
        def keys():
            for _ in range(12): pg.keyboard.press("Tab")
            assert pg.evaluate("document.querySelector('#bugdlg').contains(document.activeElement)"), "focus trapped"
            pg.keyboard.press("Shift+Tab"); assert pg.evaluate("document.querySelector('#bugdlg').contains(document.activeElement)")
            pg.keyboard.press("Escape"); pg.wait_for_selector("#bugdlg[hidden]", state="attached"); assert pg.evaluate("document.activeElement.id") == "bug-btn", "focus back on the button"
        step("Tab stays inside, Esc closes and returns the focus", keys)
        def other_views():
            for v in ("draw", "theory"):
                pg.click(f"#nav button[data-v={v}]"); attendi(pg, f"document.querySelector('#nav button[data-v={v}]').classList.contains('on')")
                assert pg.is_visible("#bug-btn"), v
            pg.click("#bug-btn"); pg.wait_for_selector("#bugdlg:not([hidden])"); assert "theory" in pg.input_value("#bug-prev"); pg.keyboard.press("Escape")
            pg.click("#nav button[data-v=data]")
        step("the icon is there in Dati, Disegno and Teoria", other_views)
        def english():
            pg.evaluate("I18N.set('en')"); pg.wait_for_function("document.querySelector('#bug-btn') && document.querySelector('#bug-btn').getAttribute('aria-label') === 'Report a bug'", timeout=60000)
            assert pg.get_attribute("#bug-btn", "aria-label") == "Report a bug"
            pg.click("#bug-btn"); pg.wait_for_selector("#bugdlg:not([hidden])")
            assert pg.inner_text("#bugdlg h2") == "Report a bug" and pg.inner_text("#bug-go") == "Open on GitHub" and pg.inner_text("#bug-copy") == "Copy report"
            assert "## What happened" in pg.input_value("#bug-prev"); pg.keyboard.press("Escape"); pg.evaluate("I18N.set('it')"); pg.wait_for_function("document.querySelector('#bug-btn') && document.querySelector('#bug-btn').getAttribute('aria-label') === 'Segnala un bug'", timeout=60000)
        step("English", english)
        def narrow():
            pg.set_viewport_size({"width": 390, "height": 800}); attendi(pg, "innerWidth === 390")
            assert pg.evaluate("document.documentElement.scrollWidth <= innerWidth + 1"), "no horizontal scroll"
            pg.set_viewport_size({"width": 1500, "height": 2200})
        step("narrow window: the header does not overflow", narrow)
finally:
    r.close(); r.report(); shutil.rmtree(tmp, ignore_errors=True)
print("\nSTEPS"); [print(" ", s) for s in steps]
sys.exit(1 if any(not s[1].startswith("ok") for s in steps) else 0)
