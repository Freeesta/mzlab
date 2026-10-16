"""Shortcuts of the registry azioni/1: one keydown runs the field `scorciatoia` of the actions. Letters only with the focus on a chart and never in a text field;
Ctrl/⌘ and Alt+1-3 anywhere; the panel «Shortcuts» (key ?) lists the registry (axe clean); the keys of the registry in the palette."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_scorciatoie.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
AXE = axe_path()
RUN = """(sel) => axe.run(document.querySelector(sel) || document, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}})
  .then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.length]))"""
r = Run(port=8901, wd="/tmp/wd_scorciatoie")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 950})
        if AXE: pg.add_script_tag(path=AXE)
        pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML"), str(EX / "FullScan_t30.mzML")])
        pg.click("#opbtn"); ready(pg)
        pnl = lambda t: f"E.panels.filter(q => q.type === '{t}')"
        focus = lambda t: pg.evaluate(f"(()=>{{const q={pnl(t)}.slice(-1)[0]; q.cv.focus(); return q.id}})()")
        n = lambda t: pg.evaluate(f"{pnl(t)}.length")
        def panel_lists_registry():
            pg.evaluate("document.activeElement && document.activeElement.blur()"); pg.keyboard.press("Shift+?")
            attendi(pg, "document.querySelector('#scorc[open]')")
            got = pg.evaluate("[...document.querySelectorAll('#scorc li')].map(li => li.querySelector('kbd').textContent)")
            want = pg.evaluate("AZIONI.voci.filter(a => a.scorciatoia).map(a => AZIONI.testo(a.scorciatoia))")
            assert got and set(got) <= set(want), (got, want)
            for k in ("Alt+1", "Alt+2", "Alt+3", "Ctrl+O", "Ctrl+E", "?") if pg.evaluate("!/Mac/.test(navigator.platform)") else ("⌥1", "⌘O", "⌘E", "?"):
                assert k in got, (k, got)
            assert pg.evaluate("document.activeElement.closest('#scorc') !== null"), "the focus is inside the panel"
            if AXE: v = pg.evaluate(RUN, "#scorc"); assert not v, v
            pg.keyboard.press("Escape"); attendi(pg, "!document.querySelector('#scorc[open]')")
        step("? opens the panel generated from the registry (axe clean), Esc closes it", panel_lists_registry)
        def nothing_in_a_field():
            pg.evaluate("(()=>{const i=document.createElement('input');i.id='tmpin';document.body.appendChild(i);i.focus()})()")
            before = n("spec") + n("xic")
            for k in ("x", "Enter", "i", "r", "f", "?"): pg.keyboard.press(k)
            assert not pg.evaluate("!!document.querySelector('#scorc[open], dialog[open]')"), "no panel, no dialog"
            assert n("spec") + n("xic") == before
            pg.evaluate("document.getElementById('tmpin').remove()")
        step("the letters do nothing in a text field", nothing_in_a_field)
        def nothing_without_focus():
            pg.evaluate("document.activeElement && document.activeElement.blur()"); before = n("spec") + n("xic")
            for k in ("x", "Enter", "i", "r"): pg.keyboard.press(k)
            assert n("spec") + n("xic") == before and not pg.evaluate("!!document.querySelector('dialog[open]')")
        step("the letters need the focus on a chart", nothing_without_focus)
        def enter_spectrum():
            focus("chrom"); b = n("spec"); pg.keyboard.press("Enter"); attendi(pg, f"{pnl('spec')}.length === {b + 1}"); ready(pg)
        step("Enter on a chromatogram: the spectrum at the cursor", enter_spectrum)
        def measure():
            focus("spec"); pg.keyboard.press("r"); attendi(pg, f"(()=>{{const q={pnl('spec')}.slice(-1)[0]; return !!(q.meas && q.meas.ref != null)}})()")
        step("R on a spectrum: the measure starts at the highest peak", measure)
        def xic():
            focus("chrom"); pg.keyboard.press("x"); attendi(pg, "!!document.querySelector('dialog[open]') || " + pnl("xic") + ".length > 0")
            if pg.evaluate("!!document.querySelector('dialog[open]')"): pg.keyboard.press("Escape"); attendi(pg, "!document.querySelector('dialog[open]')")
        step("X on a chromatogram: the XIC request", xic)
        def integrate():
            focus("chrom"); pg.keyboard.press("i")
            ready(pg)
            if pg.evaluate("!!document.querySelector('dialog[open]')"): pg.keyboard.press("Escape")
        step("I on a chromatogram does not break (integration or its guard)", integrate)
        def fullscreen():
            q = focus("chrom"); pg.keyboard.press("f"); attendi(pg, f"E.panels.find(x => x.id === {q}).el.classList.contains('max')")
            pg.keyboard.press("f"); attendi(pg, f"!E.panels.find(x => x.id === {q}).el.classList.contains('max')")
        step("F: full screen and back", fullscreen)
        def esc_priority():
            q = focus("spec"); panel = f"E.panels.find(x => x.id === {q})"
            pg.evaluate(f"(()=>{{const q={panel}; if (!(q.meas && q.meas.ref != null)) measSet(q, 0)}})()")
            pg.keyboard.press("f"); attendi(pg, f"{panel}.el.classList.contains('max')")
            pg.keyboard.press("Escape"); attendi(pg, f"!(({panel}).meas && ({panel}).meas.ref != null)")
            assert pg.evaluate(f"{panel}.el.classList.contains('max')"), "the first Esc took back the measure only, full screen stays"
            pg.keyboard.press("Escape"); attendi(pg, f"!{panel}.el.classList.contains('max')")
            assert not pg.evaluate("ESC.prossimo()"), "nothing left for Esc"
        step("Esc: one thing per press, measure before full screen", esc_priority)
        def esc_menu_first():
            q = focus("chrom"); panel = f"E.panels.find(x => x.id === {q})"
            pg.keyboard.press("f"); attendi(pg, f"{panel}.el.classList.contains('max')")
            pg.evaluate("document.getElementById('ctx').hidden = false")
            pg.keyboard.press("Escape"); attendi(pg, "document.getElementById('ctx').hidden")
            assert pg.evaluate(f"{panel}.el.classList.contains('max')"), "the menu takes the Esc, full screen stays"
            pg.keyboard.press("Escape"); attendi(pg, f"!{panel}.el.classList.contains('max')")
        step("Esc: an open menu goes before full screen", esc_menu_first)
        def whole_view():
            q = focus("chrom"); panel = f"E.panels.find(x => x.id === {q})"
            pg.evaluate(f"(()=>{{const q={panel}; q.zoom=[q._a.x0+1, q._a.x1-1]; draw(q)}})()")
            pg.keyboard.press("Backspace"); attendi(pg, f"{panel}.zoom == null")
        step("Backspace: whole view (key of the registry)", whole_view)
        def views():
            pg.evaluate("document.activeElement && document.activeElement.blur()")
            pg.keyboard.press("Alt+Digit2"); attendi(pg, "S.view === 'draw'")
            pg.keyboard.press("Alt+Digit1"); attendi(pg, "S.view === 'data'")
        step("Alt+1-3 change view (by code)", views)
        def open_files():
            assert not pg.evaluate("!!document.querySelector('dialog[open]')"), "a dialog is still open"
            pg.evaluate("document.activeElement && document.activeElement.blur()")
            with pg.expect_file_chooser(timeout=20000): pg.keyboard.press("Control+o")
        step("Ctrl+O opens the file chooser", open_files)
        def palette_key():
            pg.keyboard.press("Control+k"); attendi(pg, "PALETTE.isOpen")
            pg.fill("#pal-in", "rapporto" if pg.evaluate("I18N.lang === 'it'") else "report")
            dbg = pg.evaluate("[PALETTE.search('report').map(c => c.id), [BARRA.commands.find(c => c.id === 'report').azione, !!AZIONI.voce('app.esporta'), document.querySelector('#pal-l li .pal-t').parentElement.innerHTML.replace(/<svg.*<\/svg>/s, '')]]")
            assert pg.locator("#pal-l .pal-k").count() >= 1, ("the palette shows the key of the report", dbg)
            pg.keyboard.press("Escape")
        step("the palette shows the key", palette_key)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
