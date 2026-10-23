"""«Report a bug» (group «Segnala un bug» of the settings panel, bug-report.js): one field, no header button; Esc closes the panel and gives the focus back to the gear;
the diagnostics hold NO name of a loaded file (a file with a recognisable name is loaded) nor of an error; «Copy» fills the clipboard; «GitHub» has the expected link (form bug.yml, label bug, <= 6000 characters).
Run from the repo root: python3 tests_e2e/e2e_bug_report.py"""
import os, shutil, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from urllib.parse import urlparse, parse_qs
steps = []
def report(): return pg.evaluate("BUGREP.corpo()")
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
        pg.reload(); pg.wait_for_function('window.qqReady===true', timeout=60000)
        def opengroup():
            pg.click("#np-set"); pg.wait_for_selector("#uipset")
            g = pg.locator("#uip-g-bug"); assert g.count() == 1 and g.locator("summary svg").count() == 2, "group with the bug icon (and the chevron)"
            if not pg.evaluate("document.querySelector('#uip-g-bug').open"): pg.click("#uip-g-bug summary")
            pg.wait_for_selector("#bug-what", timeout=10000)
        def closepanel(): pg.keyboard.press("Escape"); pg.wait_for_selector("#uipset", state="detached")
        def icon():
            assert pg.locator("#bug-btn").count() == 0, "no bug button in the header"
            assert pg.evaluate("typeof window.BUGREP") == "undefined", "the form is lazy: not loaded before the group is opened"
            pg.click("#np-set"); pg.wait_for_selector("#uipset")
            assert pg.inner_text("#uip-g-bug summary").strip() == "Segnala un bug" and pg.locator("#bugdlg").count() == 0
            pg.keyboard.press("Escape")
        step("no header button; the settings have the «Segnala un bug» group with the insect; form not loaded at start", icon)
        pg.set_input_files("#pick", [str(named)]); pg.wait_for_selector("text=Carica dati"); pg.click("text=Carica dati"); ready(pg)
        def opens():
            pg.evaluate("window.ERRORI && ERRORI.segnala(new Error('Cannot read %s.mzML and %s'), 'prova')" % (SECRET, SECRET))
            opengroup()
            assert pg.locator("#uipset #bug-what").count() == 1 and pg.locator("#uipset textarea").count() == 1, "ONE field, inside the settings panel"
            assert pg.locator("#bug-prev").count() == 0 and pg.locator("#bug-exp").count() == 0 and pg.locator("#bug-diag").count() == 0
            assert pg.evaluate("document.querySelector('#uipset').getAttribute('role')") == "dialog"
            assert pg.get_attribute("#bug-what", "id") == "bug-what" and pg.locator("label[for=bug-what]").count() == 1, "the field has a label"
            pg.click("#bug-diagtxt >> xpath=ancestor::details/summary")
            t = pg.inner_text("#bug-diagtxt")
            assert "- mzLab:" in t and "- Mode: web" in t and "- Loaded files: 1 (mzML" in t and "- Service worker:" in t and "Recent errors:" in t, t
            assert SECRET not in t and "ZZ" not in t, "no name of the loaded file in the report"
            assert "<file>" in t, "the error line is kept, without the name"
        step("the group opens the single field; the diagnostics hold no file name", opens)
        def field():
            pg.fill("#bug-what", "Il grafico " + SECRET + " non si apre")
            t = report(); assert "## Cosa è successo\n\nIl grafico" in t and "Cosa mi aspettavo" not in t and "## Diagnostica" in t and "- mzLab:" in t
        step("the field builds the text; the diagnostics are always added", field)
        def copy():
            pg.click("#bug-copy"); attendi(pg, "document.querySelector('#bug-st').textContent !== ''")
            clip = pg.evaluate("navigator.clipboard.readText()"); assert clip == report() and "## Cosa è successo" in clip
            assert pg.inner_text("#bug-st") == "Rapporto copiato"
        step("Copy report puts the whole text on the clipboard", copy)
        def link():
            a = pg.locator("#bug-go"); href = a.get_attribute("href"); u = urlparse(href); q = parse_qs(u.query)
            assert u.netloc == "github.com" and u.path == "/Freeesta/mzlab/issues/new" and q["template"] == ["bug.yml"] and q["labels"] == ["bug"], href
            assert q["title"][0].startswith("[Bug] Il grafico") and "non si apre" in q["cosa"][0] and len(href) <= 6000 and q["ambiente"][0].startswith("Mozilla/5.0")
            assert a.get_attribute("target") == "_blank" and "noopener" in a.get_attribute("rel") and a.inner_text() == "GitHub"
            pg.fill("#bug-what", "x" * 20000); h = pg.get_attribute("#bug-go", "href"); assert len(h) <= 6000 and pg.inner_text("#bug-st"), "long text: the link is cut and the panel says so"
            pg.fill("#bug-what", "")
        def mail():
            assert pg.evaluate("document.documentElement.outerHTML.includes('federico.cristaudo@unito.it')") is False, "the address is not written whole in the page"
            pg.fill("#bug-what", "Il grafico non si apre")
            m = pg.evaluate("BUGREP.mailUrl()"); u = urlparse(m["url"]); q = parse_qs(u.query)
            assert u.scheme == "mailto" and u.path == "federico.cristaudo@unito.it" and q["subject"][0].startswith("[Bug] Il grafico") and "Il grafico non si apre" in q["body"][0], m
            pg.fill("#bug-what", "x" * 20000); m = pg.evaluate("BUGREP.mailUrl()"); assert len(m["url"]) <= 1900 and m["troncato"]
            assert pg.inner_text("#bug-mail") == "Invia per email"; pg.fill("#bug-what", "Il grafico " + SECRET + " non si apre")
        step("Send by email: the address is joined at runtime, mailto with the report, cut at 1900 characters", mail)
        step("Open on GitHub: expected link, new tab, never more than 6000 characters", link)
        def keys():
            pg.focus("#bug-what"); pg.keyboard.press("Tab")
            assert pg.evaluate("document.querySelector('#uipset').contains(document.activeElement)")
            pg.keyboard.press("Escape"); pg.wait_for_selector("#uipset", state="detached")
            assert pg.evaluate("document.activeElement.id") == "np-set", "Esc closes the settings and gives the focus back to the gear"
        step("keyboard: Tab moves through the panel, Esc closes it and returns the focus to the gear", keys)
        def other_views():
            for v in ("draw", "theory"):
                pg.click(f"#nav button[data-v={v}]"); attendi(pg, f"document.querySelector('#nav button[data-v={v}]').classList.contains('on')")
                assert pg.locator("#bug-btn").count() == 0, v
            pg.click("#nav button[data-v=draw]"); attendi(pg, "document.querySelector('#nav button[data-v=draw]').classList.contains('on')")
            opengroup(); assert "draw" in report(); closepanel()
            pg.click("#nav button[data-v=data]")
        step("the report is in the settings of Dati and Disegno", other_views)
        def english():
            pg.evaluate("I18N.set('en')"); pg.wait_for_function("window.qqReady===true && I18N.lang === 'en'", timeout=60000)
            opengroup()
            assert pg.inner_text("#uip-g-bug summary").strip() == "Report a bug" and pg.inner_text("#bug-go") == "GitHub" and pg.inner_text("#bug-copy") == "Copy" and pg.inner_text("#bug-mail") == "Send by email"
            assert pg.get_attribute("#bug-what", "placeholder") == "What happened?"
            assert "## What happened" in report(); closepanel(); pg.evaluate("I18N.set('it')"); pg.wait_for_function("window.qqReady===true && I18N.lang === 'it'", timeout=60000)
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
