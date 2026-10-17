"""Guard: the old loading animation (bars/histogram, `.ms` + `@keyframes pk`) must not come back; the only one is the ions canvas #ldion."""
import re
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"


def test_loading_has_only_the_ion_canvas():
    html = (WEB / "index.html").read_text(encoding="utf-8")
    box = re.search(r'<div id="loading".*?<div id="ldmsg"', html, re.S)
    assert box, "#loading not found"
    assert 'id="ldion"' in box.group(0) and "<i " not in box.group(0) and 'class="ms"' not in box.group(0), "old bars inside #loading"
    assert 'style="display:none"' not in box.group(0).split("<canvas")[1].split(">")[0], "canvas hidden at start"
    css = html + (WEB / "app.css").read_text(encoding="utf-8")
    assert not re.search(r"(^|[^\w-])\.ms\s*[{ i]|@keyframes pk\b|animation:\s*pk\b", css), "CSS of the old bars"
    assert "#loading .ms" not in (WEB / "avvio.js").read_text(encoding="utf-8")
