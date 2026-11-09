"""Garbage sweep of the typed numeric fields (mzlab/web/campi.js): every visible field of a scope gets letters, 1e999, «12abc», «1,2.3», empty, a number
outside its bounds, a decimal where a whole number is due and a «from» above its «to»; each must be refused with one note (role=status), the old value
back, aria-invalid set and the state given by `state` (a JS expression) unchanged."""
from lib import wait_for_js

NOTE = """(x => { const box = x.closest('.cpop, .sp-pop') || x.closest('.pnl'); const m = box ? box.querySelector(':scope > .xr-msg')
  : ((x.closest('label') || x).nextElementSibling); return m && !m.hidden && m.getAttribute('role') === 'status' ? m.textContent : ''; })"""
FIELDS = """(scope => [...document.querySelectorAll(scope + ' input[data-num]')].filter(x => x.offsetParent && !x.disabled)
  .map(x => { x.dataset.garb = Math.random().toString(36).slice(2); const s = CAMPI.spec(x), o = id => { const e = document.getElementById(id); return e ? CAMPI.num(e.value) : null; };
  return { id: x.dataset.garb, lo: s.lo, hi: s.hi, gt: s.gt, int: s.int, empty: s.empty, le: s.le ? o(s.le) : null, ge: s.ge ? o(s.ge) : null }; }))"""
KEYS = "['campo.num', 'campo.empty', 'campo.gt', 'campo.range', 'campo.min', 'campo.max', 'campo.int', 'campo.order'].map(k => I18N.t(k).split('{')[0])"


def garbage(f):
    out = ["abc", "1e999", "12abc", "1,2.3"]
    if not f["empty"]: out.append("")
    if f["lo"] is not None: out.append(str(f["lo"] - 1))
    if f["gt"] is not None: out.append(str(f["gt"]))
    if f["hi"] is not None: out.append(str(f["hi"] + 1))
    if f["int"]: out.append(str((f["lo"] or 0) + 0.5))
    if f["le"] is not None: out.append(str(f["le"] + 1))
    if f["ge"] is not None: out.append(str(f["ge"] - 1))
    return out


def sweep(pg, scope, state="''"):
    """Types the garbage into every visible numeric field of `scope` (a CSS selector); returns how many fields it found."""
    fs = pg.evaluate(FIELDS + f"({scope!r})")
    assert fs, f"no numeric field visible in {scope}"
    texts = pg.evaluate(KEYS)
    for f in fs:
        sel = f"[data-garb='{f['id']}']"
        for g in garbage(f):
            x = pg.locator(sel)
            if not x.count(): break          # a refusal never rebuilds anything: a missing field fails the check below
            before, st0 = x.input_value(), pg.evaluate(state)
            x.focus(); x.press("ControlOrMeta+a"); x.press_sequentially(g) if g else x.press("Delete"); x.press("Tab")
            wait_for_js(pg, f"(() => {{ const x = document.querySelector({sel!r}); return x && {NOTE}(x) !== ''; }})()", 5000)
            msg = pg.evaluate(f"{NOTE}(document.querySelector({sel!r}))")
            assert any(msg.startswith(t) for t in texts), f"{scope} {sel} {g!r}: note {msg!r}"
            assert pg.locator(sel).input_value() == before, f"{scope} {sel} {g!r}: value {pg.locator(sel).input_value()!r}, was {before!r}"
            assert pg.get_attribute(sel, "aria-invalid") == "true", f"{scope} {sel} {g!r}: aria-invalid"
            assert pg.evaluate(state) == st0, f"{scope} {sel} {g!r}: the state changed"
    return len(fs)


def typed(pg, sel, text, key="Tab"):
    """Types `text` into the field `sel` as a person does and leaves it with `key`."""
    x = pg.locator(sel).first; x.focus(); x.press("ControlOrMeta+a"); x.press_sequentially(text); x.press(key)
