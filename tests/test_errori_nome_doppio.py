"""A file the site cannot open is listed as «name: message» (`unopen.err.one` in shell.js, `nm + ": " + msg` in agilent-d.js), so a message pushed to
those lists with the file name must not start with the name again («S.wiff.scan: S.wiff.scan: waiting for…»). Reads the push sites and the catalogs."""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WEB = ROOT / "mzlab" / "web"
PUSH = re.compile(r'errs\.push\(\[(?:T|I18N\.t)\("([\w.]+)"')   # every entry is [message, file name]


def keys():
    return sorted({k for f in ("shell.js", "agilent-d.js") for k in PUSH.findall((WEB / f).read_text(encoding="utf-8"))})


def text(lang, key):
    for f in sorted((WEB / "lang" / lang).glob("*.js")):
        m = re.search(r"^\s*" + re.escape(json.dumps(key)) + r"\s*:\s*(\"(?:[^\"\\]|\\.)*\")", f.read_text(encoding="utf-8"), re.M)
        if m:
            return json.loads(m.group(1))
    raise AssertionError(f"{key} not in lang/{lang}")


def test_the_push_sites_are_found():
    assert {"load.wiffWait", "load.wiffTwin"} <= set(keys()), keys()


def test_no_listed_message_starts_with_the_name():
    bad = [(lang, k, text(lang, k)) for k in keys() for lang in ("it", "en") if text(lang, k).lstrip().startswith("{name}")]
    assert not bad, bad
