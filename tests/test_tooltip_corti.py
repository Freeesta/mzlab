"""Settings-panel and adducts/losses tooltips stay one short sentence (census rule), IT and EN."""
import glob, re

KEYS = ["settings.pal.title", "settings.ext.title", "settings.cen.title",
        "settings.xiclens.title", "settings.libs.title", "settings.reset.title",
        "cx.score.tip", "calc.in.title", "tbl.loss.searchTitle", "cmd.addloss.tip"]


def _val(lang, key):
    for f in glob.glob(f"mzlab/web/lang/{lang}/*.js"):
        m = re.search(r'"%s": "((?:[^"\\]|\\.)*)"' % re.escape(key), open(f, encoding="utf8").read())
        if m:
            return m.group(1)
    raise AssertionError(f"{key} missing in {lang}")


def test_short_tooltips():
    for lang in ("it", "en"):
        for k in KEYS:
            assert len(_val(lang, k)) <= 100, (lang, k)
