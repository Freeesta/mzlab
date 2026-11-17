"""Guard over every tooltip key of the catalog: one short sentence that says only what
the control does. No privacy or policy promises, marketing, «not an identification» disclaimers,
or «instead of» comparisons (docs/inventari/TOOLTIP.md, Regola)."""
import glob, json, re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MAX_LEN = 90
# Keys that name a tooltip: *.tip, *.title, *Title, tip.*. Hints and notes are visible text, not tooltips.
KEY = re.compile(r"(\.tip|\.title|Title|\.title\.[A-Za-z]+)$|^tip\.")
# Not tooltips although the key ends in .title/Title: headings, dialog and export titles, mark-up.
NOT_TOOLTIP = re.compile(r"^(help\.|tour\.|draw\.export\.|og\.s\d\.|.*\.dialog\.title$)")
BANNED = re.compile("|".join([
    r"nothing (is |was |gets )?(sent|uploaded|leaves)",
    r"never leaves?|sends? nothing|no request",
    r"we (do not|don.t|never)\b",
    r"privacy|\bfree\b|open.source",
    r"instead of",
    r"not (an? )?identifications?",
    r"not (verdicts?|judg)",
    r"(on )?screen only|never the areas",
]), re.I)
# Cushion sentences: reassurance or generic advice that says nothing about what the thing is or does.
CUSHION = re.compile("|".join([
    r"\byou (can|may|could|will be able)\b",
    r"keep (the )?default|leave (the )?default|settings are fine",
    r"if (you are |you.re )?(not sure|unsure|in doubt)",
    r"don.t worry|do not worry",
    r"we recommend|(is |are )?recommended",
    r"(best|better) results",
    r"keep in mind|bear in mind|feel free",
]), re.I)
ENTRY = re.compile(r'^\s*"([^"]+)":\s*("(?:[^"\\]|\\.)*")\s*,?\s*$')


def catalog(lang):
    out = {}
    for f in sorted((ROOT / f"mzlab/web/lang/{lang}").glob("*.js")):
        for ln in f.read_text(encoding="utf8").splitlines():
            m = ENTRY.match(ln)
            if m:
                out[m.group(1)] = json.loads(m.group(2))
    return out


def allowed():
    keys = set()
    for ln in (ROOT / "tests/tooltip_ammessi.txt").read_text(encoding="utf8").splitlines():
        if ln.strip() and not ln.lstrip().startswith("#"):
            assert "#" in ln, f"allow-list entry without a reason: {ln}"
            keys.add(ln.split("#")[0].strip())
    return keys


def tooltips(lang):
    return {k: v for k, v in catalog(lang).items() if KEY.search(k) and not NOT_TOOLTIP.match(k)}


def test_guard_covers_many_keys():
    assert len(tooltips("en")) > 400


def test_short_and_plain():
    ok, bad = allowed(), []
    for lang in ("en",):
        for k, v in tooltips(lang).items():
            if k in ok:
                continue
            if len(v) > MAX_LEN:
                bad.append((lang, k, "too long", len(v)))
            m = BANNED.search(v)
            if m:
                bad.append((lang, k, "banned", m.group(0)))
    assert not bad, "\n".join(map(str, bad))


def test_allow_list_has_no_stale_keys():
    assert allowed() <= set(catalog("en"))


# Literal tooltips written in the code (Tabui and Flow are English-only, no lang/ keys).
LITERAL = re.compile(r'\btitle="([^"$]*)"')


def test_literal_tooltips():
    bad = []
    for sub in ("TP_Mine/js", "TP_Mine/lazy", "flow/lazy", "mzlab/web"):
        for f in sorted((ROOT / sub).rglob("*")):
            if f.suffix not in (".js", ".html") or any(x in f.as_posix() for x in ("/vendor/", "/esempi/", "/lang/", "elements.js")):
                continue
            for n, ln in enumerate(f.read_text(encoding="utf8", errors="replace").splitlines(), 1):
                for m in LITERAL.finditer(ln):
                    v = m.group(1)
                    if len(v) > MAX_LEN or BANNED.search(v):
                        bad.append((f.relative_to(ROOT).as_posix(), n, v[:60]))
    assert not bad, "\n".join(map(str, bad))


def test_no_cushion_sentences():
    ok, bad = allowed(), []
    for lang in ("en",):
        for k, v in catalog(lang).items():
            m = CUSHION.search(v)
            if m and k not in ok:
                bad.append((lang, k, m.group(0)))
    assert not bad, "\n".join(map(str, bad))
