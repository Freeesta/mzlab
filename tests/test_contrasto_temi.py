"""Contrast (WCAG AA, 4.5:1) of the shared theme variables used by notice boxes (.avv ok/warn/bad), read from index.html."""
import re
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
HTML = (WEB / "index.html").read_text(encoding="utf-8")


def _vars(block):
    return {k: v.lower() for k, v in re.findall(r"(--[a-z0-9-]+):(#[0-9a-fA-F]{3,6})\b", block)}


def _rgb(h):
    h = h.lstrip("#")
    if len(h) == 3:
        h = "".join(c * 2 for c in h)
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


def _lum(c):
    f = lambda v: (v / 255 / 12.92) if v / 255 <= 0.03928 else ((v / 255 + 0.055) / 1.055) ** 2.4
    r, g, b = map(f, c)
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def _ratio(a, b):
    la, lb = sorted((_lum(a), _lum(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)


def _mix(fg, bg, p=0.12):
    return tuple(round(f * p + b * (1 - p)) for f, b in zip(fg, bg))


def _line(prefix):
    m = [l for l in HTML.splitlines() if l.startswith(prefix)]
    assert m, prefix
    return m[0]


def _themes():
    light = _vars(_line(":root{color-scheme:light"))
    dark = _vars(_line(":root[data-theme=dark]{color-scheme:dark"))
    hcl = {**light, **_vars(_line(":root[data-a11y-bg=hc][data-theme=light]"))}
    hcd = {**dark, **_vars(_line(":root[data-a11y-bg=hc][data-theme=dark]"))}
    return {"chiaro": light, "scuro": dark, "contrasto-chiaro": hcl, "contrasto-scuro": hcd}


def test_riquadri_avviso_leggibili():
    for nome, v in _themes().items():
        for kind in ("ok", "warn", "bad"):
            box = _mix(_rgb(v["--" + kind]), _rgb(v["--panel"]))
            for fg in ("--ink", "--" + kind):
                r = _ratio(_rgb(v[fg]), box)
                assert r >= 4.5, f"{nome}: {fg} su riquadro .avv.{kind} = {r:.2f}"


def test_testo_su_sfondi_base():
    for nome, v in _themes().items():
        for bg in ("--bg", "--panel", "--soft", "--sel"):
            if bg not in v:
                continue
            for fg in ("--ink", "--muted", "--accent", "--ok", "--bad", "--warn"):
                if bg in ("--soft", "--sel") and fg != "--ink" and fg != "--muted":
                    continue
                r = _ratio(_rgb(v[fg]), _rgb(v[bg]))
                assert r >= 4.5, f"{nome}: {fg} su {bg} = {r:.2f}"


def test_nessuna_variabile_inventata():
    # --fg, --border and --text are not theme variables: with their fallback they gave white-on-white notes
    for f in list(WEB.glob("*.js")) + list(WEB.glob("*.html")):
        t = f.read_text(encoding="utf-8")
        for bad in ("var(--fg", "var(--border,", "var(--border)"):
            assert bad not in t, f"{f.name}: {bad}"
