"""Tooltips must add information (docs/inventari/TOOLTIP.md): none may repeat the visible text of its own element,
and none may be only the «Right-click: actions» hint. The two lists below only shrink."""
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import inventario_tooltip as T  # noqa: E402

# keys still waiting for the chromatogram-peak rework (CROMA-CLICK-PICCO)
REPEATS_ALLOWED = {"etpk.tip.rt"}
HINT_ONLY_ALLOWED = {"tip.area", "tip.peakLabel"}
HINT = re.compile(r"right-click: actions|tasto destro: azioni", re.I)


def norm(x):
    return re.sub(r"[^\w]+", " ", re.sub(r"<[^>]+>|\[(?:icon|dyn)\]", "", x).lower()).strip()


def test_no_tooltip_repeats_its_own_text():
    bad = []
    for r in T.scan():
        t, v = norm(r["it"] or r["lit"]), norm(r["vis"])
        if r["tag"] and t and v and (t == v or (len(t) > 3 and t in v and len(v) - len(t) < 5)) and r["key"] not in REPEATS_ALLOWED:
            bad.append(f"{r['file']}:{r['line']} {r['key']} «{t}» = «{v}»")
    assert not bad, "tooltip that repeats the visible text (remove the title):\n" + "\n".join(bad)


def test_no_right_click_hint_only_tooltip():
    cat = T.catalogs()
    bad = [k for lang in ("it", "en") for k, v in cat[lang].items()
           if k not in HINT_ONLY_ALLOWED and re.fullmatch(r"(?:<[^>]+>)*\s*" + HINT.pattern + r"\s*(?:<[^>]+>)*", v, re.I)]
    assert not bad, "tooltip made only of the «Right-click: actions» hint: " + ", ".join(bad)
