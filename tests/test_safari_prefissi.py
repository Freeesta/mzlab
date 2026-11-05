"""Safari (macOS and iPadOS, every version) reads `user-select` only as `-webkit-user-select`: every unprefixed declaration in the
interface CSS (stylesheets and CSS kept in JS strings) carries the prefixed one beside it, or drags and resizes select text on Safari."""
import re
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
SKIP = ("vendor/", "esempi/", "lang/", "elements.js")


def test_user_select_has_webkit_prefix():
    bad = []
    for p in sorted(WEB.rglob("*")):
        rel = p.relative_to(WEB).as_posix()
        if p.suffix not in (".css", ".js", ".mjs", ".html") or rel.startswith(SKIP) or rel in SKIP:
            continue
        for i, line in enumerate(p.read_text(encoding="utf-8").splitlines(), 1):
            for m in re.finditer(r"(?<![-\w])user-select:\s*([a-z]+)", line):
                if not re.search(r"-webkit-user-select:\s*" + m.group(1), line[max(0, m.start() - 80):m.start()]):
                    bad.append(f"{rel}:{i}")
    assert not bad, bad
