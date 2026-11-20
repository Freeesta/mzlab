"""«Report a bug» and «Suggest an improvement» (two groups of the settings panel, bug-report.js): one field (a suggestion also «What I was doing»), no header button; Esc closes the panel and gives the focus
back to the gear; the diagnostics are a list of fields the user can untick and hold NO name of a loaded file (a file with a recognisable name is loaded) nor of an error; «Copy» fills the clipboard;
«Download full report» saves the same fields as JSON; «GitHub» has the expected link (form bug.yml / suggestion.yml, label, <= 6000 characters).
Run from the repo root: python3 tests_e2e/e2e_bug_report.py"""
# covers: mzlab/web/bug-report
import json, os, shutil, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth
from urllib.parse import urlparse, parse_qs
steps = []
def report(kind="bug"): return pg.evaluate("BUGREP.corpo(%r)" % kind)
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + " | ".join(str(e).split("\n"))[:400])); import traceback; traceback.print_exc()
r = Run(port=8871, wd="/tmp/wd_bug")
SECRET = "ZZSegretoRossi_campione"
tmp = Path("/tmp/wd_bug_in"); shutil.rmtree(tmp, ignore_errors=True); tmp.mkdir()
src = synth("bugrep") / "B_FullMass-t0.mzML"; named = tmp / (SECRET + ".mzML"); shutil.copy(src, named)
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.context.grant_permissions(["clipboard-read", "clipboard-write"], origin=f"http://127.0.0.1:{r.port}")
        pg.reload(); pg.wait_for_function('window.qqReady===true', timeout=60000)
        def opengroup():
            if not pg.is_visible("#uipset"): pg.click("#np-set")
            pg.wait_for_selector("#uipset")
            g = pg.locator("#uip-g-bug"); assert g.count() == 1 and g.locator("button svg").count() == 1, "plain button with the bug icon, no fold"
            if pg.locator("#uip-bug").is_hidden(): pg.click("#uip-bugbtn")
            pg.wait_for_selector("#bug-what", timeout=10000)
        def opensug():
            if not pg.is_visible("#uipset"): pg.click("#np-set")
            pg.wait_for_selector("#uipset")
            if pg.locator("#uip-sug").is_hidden(): pg.click("#uip-sugbtn")
            pg.wait_for_selector("#sug-what", timeout=10000)
        def closepanel():
            pg.focus("#uip-bugbtn")      # Esc goes to the focused document: in the drawing view the sketcher iframe can take the focus while the panel opens
            pg.keyboard.press("Escape")
            try: pg.wait_for_selector("#uipset", state="detached", timeout=5000)
            except TimeoutError:
                st = pg.evaluate("JSON.stringify({n: document.querySelectorAll('#uipset').length, ae: document.activeElement && (document.activeElement.id || document.activeElement.tagName), view: S.view, dlg: !!document.querySelector('dialog[open]'), pal: !!document.querySelector('#paletta:not([hidden])')})")
                pg.keyboard.press("Escape"); late = pg.evaluate("document.querySelectorAll('#uipset').length")
                raise AssertionError(f"Escape did not close the settings: {st}; after a second Escape: {late}")
        def icon():
            assert pg.locator("#bug-btn").count() == 0, "no bug button in the header"
            assert pg.evaluate("typeof window.BUGREP") == "undefined", "the form is lazy: not loaded before the button opens it"
            pg.click("#np-set"); pg.wait_for_selector("#uipset")
            assert pg.inner_text("#uip-bugbtn").strip() == "Report a bug" and pg.locator("#bugdlg").count() == 0
            pg.keyboard.press("Escape")
        step("no header button; the settings have the «Report a bug» button with the insect; form not loaded at start", icon)
        pg.set_input_files("#pick", [str(named)]); pg.wait_for_selector("text=Load data"); pg.click("text=Load data"); ready(pg)
        def opens():
            pg.evaluate("window.ERRORI && ERRORI.segnala(new Error('Cannot read %s.mzML and %s'), 'test')" % (SECRET, SECRET))
            opengroup()
            assert pg.locator("#uipset #bug-what").count() == 1 and pg.locator("#uipset textarea").count() == 1, "ONE field, inside the settings panel"
            assert pg.locator("#bug-prev").count() == 0 and pg.locator("#bug-exp").count() == 0 and pg.locator("#bug-diag").count() == 0
            assert pg.evaluate("document.querySelector('#uipset').getAttribute('role')") == "dialog"
            assert pg.get_attribute("#bug-what", "id") == "bug-what" and pg.locator("label[for=bug-what]").count() == 1, "the field has a label"
            pg.click("#uip-bug details > summary"); wait_for_js(pg, "document.querySelector('#bug-diagtxt').textContent !== ''")
            assert pg.is_visible("#bug-what"), "form still open"
            t = pg.inner_text("#bug-diagtxt")
            assert "- mzLab:" in t and "- Mode: web" in t and "- Loaded files: 1 (mzML" in t and "- Service worker:" in t and "Recent errors (last" in t, t[t.find("Loaded files") - 5:][:200]
            assert SECRET not in t and "ZZ" not in t, "no name of the loaded file in the report"
            assert "<file>" in t, "the error line is kept, without the name"
        step("the group opens the single field; the diagnostics hold no file name", opens)
        def field():
            pg.fill("#bug-what", "The chart " + SECRET + " does not open")
            t = report(); assert "## What happened\n\nThe chart" in t and "What I expected" not in t and "## Diagnostics" in t and "- mzLab:" in t
        step("the field builds the text; the diagnostics are always added", field)
        def copy():
            pg.click("#bug-copy"); wait_for_js(pg, "document.querySelector('#bug-st').textContent !== ''")
            clip = pg.evaluate("navigator.clipboard.readText()"); assert clip == report() and "## What happened" in clip
            assert pg.inner_text("#bug-st") == "Report copied"
        step("Copy report puts the whole text on the clipboard", copy)
        def link():
            a = pg.locator("#bug-go"); href = a.get_attribute("href"); u = urlparse(href); q = parse_qs(u.query)
            assert u.netloc == "github.com" and u.path == "/Freeesta/mzlab/issues/new" and q["template"] == ["bug.yml"] and q["labels"] == ["bug"], href
            assert q["title"][0].startswith("[Bug] The chart") and "does not open" in q["cosa"][0] and len(href) <= 6000 and q["ambiente"][0].startswith("Mozilla/5.0")
            assert a.get_attribute("target") == "_blank" and "noopener" in a.get_attribute("rel") and a.inner_text() == "GitHub"
            pg.fill("#bug-what", "x" * 20000); h = pg.get_attribute("#bug-go", "href"); assert len(h) <= 6000 and pg.inner_text("#bug-st"), "long text: the link is cut and the panel says so"
            pg.fill("#bug-what", "")
        def mail():
            assert pg.evaluate("document.documentElement.outerHTML.includes('federico.cristaudo@unito.it')") is False, "the address is not written whole in the page"
            pg.fill("#bug-what", "The chart does not open")
            m = pg.evaluate("BUGREP.mailUrl()"); u = urlparse(m["url"]); q = parse_qs(u.query)
            assert u.scheme == "mailto" and u.path == "federico.cristaudo@unito.it" and q["subject"][0].startswith("[Bug] The chart") and "The chart does not open" in q["body"][0], m
            pg.fill("#bug-what", "x" * 20000); m = pg.evaluate("BUGREP.mailUrl()"); assert len(m["url"]) <= 1900 and m["truncated"]
            assert pg.inner_text("#bug-mail") == "Send by email"; pg.fill("#bug-what", "The chart " + SECRET + " does not open")
        step("Send by email: the address is joined at runtime, mailto with the report, cut at 1900 characters", mail)
        step("Open on GitHub: expected link, new tab, never more than 6000 characters", link)
        def fields():
            opengroup(); pg.click("#uip-bug details > summary") if pg.locator("#bug-diagtxt").is_hidden() else None
            t = pg.inner_text("#bug-diagtxt")
            for need in ("- Language: en", "- Screen and window: screen ", "pixel ratio", "- Cross-origin isolated:", "- OPFS available:", "- Engine:", "- Active tool:", "- Open cards:", "- Changed settings:", "- Stored state:", "- Recent actions (oldest first):", "- Recent errors (last"):
                assert need in t, (need, t)
            assert SECRET not in t and "ZZ" not in t and ".mzML" not in t and "/tmp" not in t, t
            n = pg.locator("#bug-diagtxt input[type=checkbox]").count(); assert n >= 12 and pg.locator("#bug-diagtxt input:not(:checked)").count() == 0, "every field is ticked and visible"
        step("the diagnostics list the new fields, all ticked, with no name", fields)
        def untick():
            pg.fill("#bug-what", "boom"); assert "Recent errors" in report() and "Recent actions" in report()
            pg.uncheck("#bug-diagtxt input[data-k=errors]"); pg.uncheck("#bug-diagtxt input[data-k=actions]")
            r = report(); assert "Recent errors" not in r and "Recent actions" not in r and "- mzLab:" in r
            j = json.loads(pg.evaluate("BUGREP.json('bug')")); assert "errors" not in j["fields"] and "actions" not in j["fields"] and j["fields"]["version"] and j["kind"] == "bug"
            pg.check("#bug-diagtxt input[data-k=errors]"); pg.check("#bug-diagtxt input[data-k=actions]")
        step("a field can be unticked: it leaves the text and the JSON", untick)
        def download():
            pg.fill("#bug-what", "The chart " + SECRET + " does not open")
            with pg.expect_download() as d: pg.click("#bug-dl")
            data = Path(d.value.path()).read_text(encoding="utf-8"); j = json.loads(data)
            assert d.value.suggested_filename == "mzlab-bug-report.json" and j["schema"] == "mzlab-bug-report/1" and j["fields"]["lang"] == "en" and isinstance(j["fields"]["errors"], list), data[:300]
            assert "ZZ" not in json.dumps(j["fields"]) and pg.inner_text("#bug-st") == "Report downloaded (JSON)"
        step("Download full report: the JSON has the same fields by key and no file name", download)
        def sug_open():
            assert pg.inner_text("#uip-sugbtn").strip() == "Suggest an improvement" and pg.locator("#uip-g-sug svg").count() == 1
            assert pg.evaluate("document.querySelector('#uip-g-bug').compareDocumentPosition(document.querySelector('#uip-g-sug')) & Node.DOCUMENT_POSITION_FOLLOWING"), "next to «Report a bug»"
            pg.keyboard.press("Escape"); pg.wait_for_selector("#uipset", state="detached")
            opensug()
            assert pg.locator("#sug-what").count() == 1 and pg.locator("#sug-doing").count() == 1 and pg.locator("label[for=sug-what]").count() == 1 and pg.locator("label[for=sug-doing]").count() == 1
            tip = lambda i: pg.get_attribute(i, "title") or pg.get_attribute(i, "data-tip") or ""
            bad = [i for i in ("#uip-sugbtn", "#sug-what", "#sug-doing", "#sug-go", "#sug-copy", "#sug-mail", "#sug-dl") if not tip(i) or len(tip(i)) > 60]
            assert not bad, "tooltips missing or over 60 characters: %s" % bad
            assert pg.get_attribute("#sug-st", "aria-live") == "polite"
        step("«Suggest an improvement» sits next to «Report a bug», same pattern, labelled fields, short tooltips", sug_open)
        def sug_text():
            pg.fill("#sug-what", "Add a dark palette for " + SECRET); pg.fill("#sug-doing", "Comparing two chromatograms")
            t = report("suggestion")
            assert t.startswith("## Suggestion\n\nAdd a dark palette") and "\n## What I was doing\n\nComparing two chromatograms\n" in t and "\n## Diagnostics\n" in t, t
            d = t.split("## Diagnostics")[1]
            assert "- mzLab:" in d and "- View:" in d and "- Language: en" in d, d
            for no in ("- Browser and system", "- Loaded files", "- Recent errors", "- Screen", "- Engine", "- Recent actions"): assert no not in d, (no, d)
            assert "ZZ" not in d, "minimal diagnostics: no file name"
            ids = pg.locator("#sug-diagtxt input[type=checkbox]").count(); assert ids == 3, ids
            j = json.loads(pg.evaluate("BUGREP.json('suggestion')")); assert j["kind"] == "suggestion" and j["doing"] == "Comparing two chromatograms" and sorted(j["fields"]) == ["lang", "version", "view"], j
        step("the suggestion has fixed headings and minimal diagnostics only (version, view, language)", sug_text)
        def sug_link():
            a = pg.locator("#sug-go"); href = a.get_attribute("href"); u = urlparse(href); q = parse_qs(u.query)
            assert u.netloc == "github.com" and u.path == "/Freeesta/mzlab/issues/new" and q["template"] == ["suggestion.yml"] and q["labels"] == ["suggestion"], href
            assert q["title"][0].startswith("Suggestion: Add a dark palette") and "## Suggestion" in q["idea"][0] and "ambiente" not in q and len(href) <= 6000
            assert a.get_attribute("target") == "_blank" and "noopener" in a.get_attribute("rel") and a.inner_text() == "GitHub"
            pg.fill("#sug-what", "x" * 20000); h = pg.get_attribute("#sug-go", "href"); assert len(h) <= 6000 and "too long for a link" in pg.inner_text("#sug-st") and "Download full report" in pg.inner_text("#sug-st"), "long text: say it on screen"
            m = pg.evaluate("BUGREP.mailUrl('suggestion')"); assert len(m["url"]) <= 1900 and m["truncated"]
            pg.fill("#sug-what", "Add a dark palette")
            m = pg.evaluate("BUGREP.mailUrl('suggestion')"); q = parse_qs(urlparse(m["url"]).query); assert urlparse(m["url"]).scheme == "mailto" and q["subject"][0].startswith("Suggestion: Add a dark palette"), m
        step("Suggestion on GitHub: issue form suggestion.yml, label suggestion, nothing sent; long text says so", sug_link)
        def sug_copy_dl():
            pg.fill("#sug-what", "Add a dark palette")
            pg.click("#sug-copy"); wait_for_js(pg, "document.querySelector('#sug-st').textContent !== ''")
            assert pg.evaluate("navigator.clipboard.readText()") == report("suggestion") and pg.inner_text("#sug-st") == "Report copied"
            with pg.expect_download() as d: pg.click("#sug-dl")
            j = json.loads(Path(d.value.path()).read_text(encoding="utf-8")); assert d.value.suggested_filename == "mzlab-suggestion.json" and j["schema"] == "mzlab-suggestion/1" and j["what"] == "Add a dark palette"
            pg.click("#uip-sug details > summary") if pg.locator("#sug-diagtxt").is_hidden() else None
            pg.uncheck("#sug-diagtxt input[data-k=lang]"); assert "- Language" not in report("suggestion")
        step("Suggestion: Copy and Download work; a field can be unticked", sug_copy_dl)
        def sug_keys():
            pg.focus("#sug-what"); pg.keyboard.press("Tab"); assert pg.evaluate("document.activeElement.id") == "sug-doing"
            pg.keyboard.press("Escape"); pg.wait_for_selector("#uipset", state="detached")
            assert pg.evaluate("document.activeElement.id") == "np-set"
        step("Suggestion keyboard: Tab moves between the fields, Esc closes and returns the focus", sug_keys)
        def keys():
            opengroup(); pg.focus("#bug-what"); pg.keyboard.press("Tab")
            assert pg.evaluate("document.querySelector('#uipset').contains(document.activeElement)")
            pg.keyboard.press("Escape"); pg.wait_for_selector("#uipset", state="detached")
            assert pg.evaluate("document.activeElement.id") == "np-set", "Esc closes the settings and gives the focus back to the gear"
        step("keyboard: Tab moves through the panel, Esc closes it and returns the focus to the gear", keys)
        def other_views():
            pg.click("#nav button[data-v=draw]"); wait_for_js(pg, "document.querySelector('#nav button[data-v=draw]').classList.contains('on')")
            assert pg.locator("#bug-btn").count() == 0
            pg.click("#nav button[data-v=draw]"); wait_for_js(pg, "document.querySelector('#nav button[data-v=draw]').classList.contains('on')")
            opengroup(); assert "draw" in report(); closepanel()
            pg.click("#nav button[data-v=data]")
        step("the report is in the settings of Data and Drawing", other_views)
        def english():
            opengroup()
            assert pg.inner_text("#uip-bugbtn").strip() == "Report a bug" and pg.inner_text("#bug-go") == "GitHub" and pg.inner_text("#bug-copy") == "Copy" and pg.inner_text("#bug-mail") == "Send by email"
            assert pg.get_attribute("#bug-what", "placeholder") == "What happened?"
            assert "## What happened" in report(); closepanel()
        step("English", english)
        def narrow():
            pg.set_viewport_size({"width": 390, "height": 800}); wait_for_js(pg, "innerWidth === 390")
            try: wait_for_js(pg, "document.documentElement.scrollWidth <= innerWidth + 1", 5000)      # the header relayouts on the resize event
            except TimeoutError: raise AssertionError("no horizontal scroll: " + pg.evaluate("JSON.stringify({sw: document.documentElement.scrollWidth, iw: innerWidth, wide: [...document.querySelectorAll('body *')].filter(e => e.getBoundingClientRect().right > innerWidth + 1).slice(0, 5).map(e => e.id || e.className || e.tagName)})"))
            pg.set_viewport_size({"width": 1500, "height": 2200})
        step("narrow window: the header does not overflow", narrow)
finally:
    r.close(); r.report(); shutil.rmtree(tmp, ignore_errors=True)
print("\nSTEPS"); [print(" ", s) for s in steps]
sys.exit(1 if any(not s[1].startswith("ok") for s in steps) else 0)
