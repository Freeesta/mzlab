"""Guard over every tooltip key of the catalogs (IT and EN): one short sentence that says only what
the control does. No privacy or policy promises, marketing, «not an identification» disclaimers,
or «instead of» comparisons (docs/studi/TOOLTIP_CENSUS.md, Rule)."""
import glob, json, re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MAX_LEN = 90
# Keys that name a tooltip: *.tip, *.title, *Title, tip.*. Hints and notes are visible text, not tooltips.
KEY = re.compile(r"(\.tip|\.title|Title|\.title\.[A-Za-z]+)$|^tip\.")
# Not tooltips although the key ends in .title/Title: headings, dialog and export titles, mark-up.
NOT_TOOLTIP = re.compile(r"^(help\.|tour\.|draw\.export\.|og\.s\d\.|.*\.dialog\.title$)")
BANNED = re.compile("|".join([
    r"non (carichiamo|inviamo|salviamo|viene inviat|vengono inviat|lascia|esce)",
    r"nessun dato|nulla (viene|lascia|esce|è inviato)|nothing (is |was |gets )?(sent|uploaded|leaves)",
    r"never leaves?|mai (lascia|esce)|non invia|invia nulla|sends? nothing|nessuna richiesta|no request",
    r"we (do not|don.t|never)\b",
    r"privacy|gratis|gratuit|\bfree\b|open.source",
    r"invece di|instead of",
    r"non (è|sono) (un.|delle? )?(identificazion)|not (an? )?identifications?",
    r"non (verdett|giudizi)|not (verdicts?|judg)",
    r"solo sullo schermo|(on )?screen only|mai le aree|never the areas",
]), re.I)
# Cushion sentences: reassurance or generic advice that says nothing about what the thing is or does.
CUSHION = re.compile("|".join([
    r"\b(puoi|potrai|potresti)\b|\byou (can|may|could|will be able)\b",
    r"lascia(re)? (i )?valori predefiniti|keep (the )?default|leave (the )?default|vanno bene così|settings are fine",
    r"se non (sai|sei sicur|conosci)|if (you are |you.re )?(not sure|unsure|in doubt)",
    r"non preoccuparti|don.t worry|do not worry",
    r"consigliamo|si consiglia|consigliat[ao]|we recommend|(is |are )?recommended",
    r"migliori risultati|risultati migliori|(best|better) results",
    r"tieni presente|tenere presente|keep in mind|bear in mind|feel free",
]), re.I)
ENTRY = re.compile(r'^\s*"([^"]+)":\s*("(?:[^"\\]|\\.)*")\s*,?\s*$')


def catalog(lang):
    out = {}
    for f in [ROOT / f"mzlab/web/lang/{lang}.js"] + sorted((ROOT / f"mzlab/web/lang/{lang}").glob("*.js")):
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
    assert len(tooltips("it")) > 400 and set(tooltips("it")) == set(tooltips("en"))


def test_short_and_plain():
    ok, bad = allowed(), []
    for lang in ("it", "en"):
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
    assert allowed() <= set(catalog("it"))


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
    for lang in ("it", "en"):
        for k, v in catalog(lang).items():
            m = CUSHION.search(v)
            if m and k not in ok:
                bad.append((lang, k, m.group(0)))
    assert not bad, "\n".join(map(str, bad))
