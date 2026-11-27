"""Test helpers for the shared panel bar (#pbar): reach a panel button in the bar when it is on, in the header when it is off.
MZLAB_PBAR=1 (lib.py) runs a whole suite with the bar on. Imported by lib.py; the lib functions are looked up at call time."""
import lib as _l


def _pbar_on(pg):
    return pg.evaluate("!!document.querySelector('#pbar') && !document.querySelector('#pbar').hidden")

NAV = ("mapzin", "mapzout", "maphelp")   # the zoom and gestures buttons of the RT × m/z map: data-nav in the header, data-a in the bar

def _hd(id):
    return f"[data-nav={id}]" if id in NAV else f"[data-a={id}]"

def bar_loc(pg, panel, id):
    """The panel button `id` (its data-a) of the panel `panel` (a CSS selector such as '.pnl.spec'): with the shared panel bar on, select the panel and give the twin in `#pbar`;
    with it off (the default), the button in the panel header. Works with both so one test covers the two settings (MZLAB_PBAR=1 runs the whole suite with the bar on)."""
    if not _pbar_on(pg) or (id == "max" and pg.evaluate("s => !!document.querySelector(s)?.classList.contains('max')", panel)):   # an expanded panel covers the bar: its inline button collapses it
        return pg.locator(f"{panel} {_hd(id)}").first
    pg.evaluate("s => { const el = document.querySelector(s); setActive(E.panels.find(q => q.el === el)); }", panel)
    _l.wait_for_js(pg, "document.querySelector('#pbar .pbchip') && E.active && document.querySelector('#pbar .pbchip').textContent.startsWith(E.active.num + ' ')")
    return pg.locator(f"#pbar [data-a={id}]").first

def bar_pick(pg, panel, id, **kw):
    """Choose an option of the select `id` of `panel` (the integration target `intf`): in `#pbar` with the bar on, else in the header, where a narrow header moves it into «⋯» (opened first, closed after)."""
    loc = bar_loc(pg, panel, id)
    if _pbar_on(pg): return loc.select_option(**kw)
    tog = "s => { const m = s.closest('.hmore'); if (!m || m.hidden !== %s) return false; m.closest('.pnl').querySelector('[data-a=more]').click(); return true; }"
    loc.evaluate(tog % "true"); loc.select_option(**kw); loc.evaluate(tog % "false")

def bar(pg, panel, id, keep=False):
    """Click the panel button `id` of `panel` the way a user reaches it: in `#pbar` with the bar on, else in the header (through «⋯» when a narrow header moved it, see hd_click)."""
    if _pbar_on(pg): bar_loc(pg, panel, id).click()   # the header of an expanded panel is never in «⋯» for this button
    else: _l.hd_click(pg, f"{panel} {_hd(id)}", keep)
