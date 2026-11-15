"""The preview of the shared link (WhatsApp & co): static title, Open Graph tags with ABSOLUTE addresses, a picture of the right size and weight."""
import importlib.util
import re
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("build_site", ROOT / "tools" / "build_site.py")
bs = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bs)


def test_picture_is_1200x630_and_light():
    png = (ROOT / "mzlab" / "web" / "anteprima.png").read_bytes()
    assert png[:8] == b"\x89PNG\r\n\x1a\n"
    w, h = struct.unpack(">II", png[16:24])
    assert (w, h) == (1200, 630) and len(png) < 300_000            # WhatsApp shows the large preview only below ~300 KB


def test_tags_have_the_name_and_absolute_addresses():
    t = bs.preview_tags()
    for k in ("og:title", "og:description", "og:image", "og:url", "twitter:card", "og:site_name"):
        assert k in t, k
    assert f'content="{bs.APP_NAME}"' in t and bs.APP_NAME in t.split("og:title")[1].split(">")[0]
    assert re.search(rf'property="og:image" content="{re.escape(bs.SITE_URL)}static/anteprima\.png(\?v=\d+)?"', t) and bs.SITE_URL.startswith("https://")
    assert re.search(rf'name="twitter:image" content="{re.escape(bs.SITE_URL)}static/anteprima\.png', t)
    assert 'property="og:image:width" content="1200"' in t and 'property="og:image:height" content="630"' in t
    assert "{APP}" not in t and len(bs.DESCRIPTION) < 200


def test_the_page_has_one_title_placeholder_and_one_head_to_close():
    html = (ROOT / "mzlab" / "web" / "index.html").read_text(encoding="utf-8")
    assert len(re.findall(r"<title>[^<]*\{APP\}[^<]*</title>", html)) == 1 and html.count("</head>") == 1
    out = re.sub(r"<title>[^<]*\{APP\}[^<]*</title>", f"<title>{bs.APP_NAME} · {bs.TAGLINE}</title>", html, count=1).replace("</head>", bs.preview_tags() + "\n</head>", 1)
    assert "{APP}</title>" not in out and out.index("og:image") < out.index("</head>")
