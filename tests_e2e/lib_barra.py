"""Test helpers for the shared panel bar (#pbar): reach a panel button in the bar. Imported by lib.py; the lib functions are looked up at call time."""
import lib as _l


NAV = ("mapzin", "mapzout", "maphelp")   # the zoom and gestures buttons of the RT × m/z map: data-nav in the header, data-a in the bar

def _hd(id):
    return f"[data-nav={id}]" if id in NAV else f"[data-a={id}]"

def bar_loc(pg, panel, id):
    """The panel button `id` (its data-a) of the panel `panel` (a CSS selector such as '.pnl.spec'): select the panel and give the button in `#pbar`.
    An expanded panel covers the bar: its inline button (`max`) is in the header."""
    if id == "max" and pg.evaluate("s => !!document.querySelector(s)?.classList.contains('max')", panel):
        return pg.locator(f"{panel} {_hd(id)}").first
    pg.evaluate("s => { const el = document.querySelector(s); setActive(E.panels.find(q => q.el === el)); }", panel)
    _l.wait_for_js(pg, "document.querySelector('#pbar .pbchip') && E.active && document.querySelector('#pbar .pbchip').textContent.startsWith(E.active.num + ' ')")
    return pg.locator(f"#pbar [data-a={id}]").first

def bar_pick(pg, panel, id, **kw):
    """Choose an option of the select `id` of `panel` (the integration target `intf`) in `#pbar`."""
    return bar_loc(pg, panel, id).select_option(**kw)

def bar(pg, panel, id, keep=False):
    """Click the panel button `id` of `panel` in `#pbar` (it selects the panel first)."""
    bar_loc(pg, panel, id).click()
