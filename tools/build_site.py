"""Builds the static site (the whole program running in the browser) into site/.

    python tools/build_site.py

site/index.html + site/static/ (the web folder as is). The engine of the program, mzFinder included, is Rust (static/wasm/, static/wasm-feat/,
built by the workflow): the site downloads nothing from the net and carries no Python. `--pyodide-dir DIR` and `--no-pyodide` are still
accepted and do nothing (the workflows pass them). GitHub Pages serves the site as it is (.github/workflows/pages.yml).
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import sys
import zipfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import tema_testa  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
# The visible name of the program lives only in mzlab/web/appname.js
APP_NAME = re.search(r'APP_NAME\s*=\s*"([^"]+)"', (ROOT / "mzlab" / "web" / "appname.js").read_text(encoding="utf-8")).group(1)

# Address of the published site (GitHub Pages): link previews (WhatsApp, Telegram, Slack...) need ABSOLUTE addresses for the page and the picture
SITE_URL = "https://freeesta.github.io/mzlab/"
TAGLINE = "Analisi MS"         # Italian tagline of the link previews (the previews are Italian)
TITLE_TAG = "MS Analysis"      # tagline of the static <title>: the English one, the default language; same as index.html and as the catalog key app.title in English
DESCRIPTION = "Esplora cromatogrammi, spettri, XIC e MRM dei tuoi file LC-MS/MS nel browser. Nessuna installazione: i dati restano sul tuo computer."


def preview_tags() -> str:
    """Open Graph + Twitter card + description: what a chat app reads (without running any JavaScript) to build the preview of a shared link."""
    a = lambda t: t.replace("&", "&amp;").replace('"', "&quot;")
    title = f"{APP_NAME} · {TAGLINE}"
    img = SITE_URL + "static/anteprima.png?v=3"          # ?v=N: change N with a new picture, the chat apps keep the old one by its address
    meta = [("name", "description", DESCRIPTION), ("property", "og:type", "website"), ("property", "og:site_name", APP_NAME), ("property", "og:locale", "it_IT"), ("property", "og:url", SITE_URL),
            ("property", "og:title", title), ("property", "og:description", DESCRIPTION), ("property", "og:image", img), ("property", "og:image:type", "image/png"),
            ("property", "og:image:width", "1200"), ("property", "og:image:height", "630"), ("property", "og:image:alt", f"Logo e nome di {APP_NAME}"),
            ("name", "twitter:card", "summary_large_image"), ("name", "twitter:title", title), ("name", "twitter:description", DESCRIPTION), ("name", "twitter:image", img)]
    return "\n".join(f'<meta {k}="{n}" content="{a(v)}">' for k, n, v in meta) + f'\n<link rel="canonical" href="{SITE_URL}">'


# Web app manifest (PWA). Paths are relative to the manifest, which sits in the site root, so it works under /QqQ-lab/.
MANIFEST = {
    "name": APP_NAME, "short_name": APP_NAME, "lang": "it", "dir": "ltr",
    "description": "Esplora i dati LC-MS/MS del triplo quadrupolo (Full Scan, Product ion, MRM) direttamente nel browser.",
    "start_url": "./", "scope": "./", "id": "./", "display": "standalone",
    "background_color": "#ffffff", "theme_color": "#ffffff", "categories": ["education", "science"],
    "icons": [
        {"src": "static/app-icon-192.png", "sizes": "192x192", "type": "image/png", "purpose": "any"},
        {"src": "static/app-icon-512.png", "sizes": "512x512", "type": "image/png", "purpose": "any"},
        {"src": "static/app-icon-maskable-512.png", "sizes": "512x512", "type": "image/png", "purpose": "maskable"},
    ],
}


def python_files(out: Path) -> list[str]:
    """The Python the site carries: any `pyodide*` file or folder and any zip with a `.py` inside (there must be none, mzFinder's folder included)."""
    found = []
    for f in sorted(out.rglob("*")):
        rel = f.relative_to(out)
        if "pyodide" in f.name.lower():
            found.append(rel.as_posix())
        elif f.is_file() and f.suffix == ".zip":
            with zipfile.ZipFile(f) as z:
                if any(n.endswith(".py") for n in z.namelist()):
                    found.append(rel.as_posix())
    return found


def pack_mzfinder(static: Path) -> None:
    """mzFinder (sources in TP_Mine/) in plain text in static/mzfinder/: the loader fetches it only after the 5 clicks on the logo."""
    src = ROOT / "TP_Mine"
    if not (src / "js").is_dir():
        return
    dst = static / "mzfinder"
    dst.mkdir()
    js = [f.name for f in sorted((src / "js").glob("*.js"))]
    files = [f.name for f in sorted((src / "files").glob("*"))] if (src / "files").is_dir() else []
    data = [f.name for f in sorted((src / "data").iterdir()) if f.suffix == ".csv" or f.name in ("cancello.json", "regole_tp.json")] if (src / "data").is_dir() else []      # curated lists and the gate of quality, read by the scripts through QTOOLS.ctx.files
    for n in data:
        shutil.copy2(src / "data" / n, dst / n)
    for n in js:
        shutil.copy2(src / "js" / n, dst / n)
    lazy = [f.name for f in sorted((src / "lazy").glob("*.js"))] if (src / "lazy").is_dir() else []      # loaded on demand through TPLAZY.load, not at the start of Tabui
    if lazy:
        (dst / "lazy").mkdir()
        for n in lazy:
            shutil.copy2(src / "lazy" / n, dst / "lazy" / n)
    for n in files:
        shutil.copy2(src / "files" / n, dst / n)
    (dst / "indice.json").write_text(json.dumps({"js": js, "lazy": lazy, "files": files + data}, indent=1), encoding="utf-8")


def write_manifest(out: Path, version: str) -> None:
    """static/manifest.json: SHA-256 and size of every file of the site (the desktop app downloads only the ones that changed),
    the commit as `version` and `desktop_min`, the oldest native desktop binary this web part works with (crates/mzlab-desktop/DESKTOP_MIN).
    sw.js is left out (the desktop app has no service worker) and so is the manifest itself."""
    files = {}
    for f in sorted(out.rglob("*")):
        rel = f.relative_to(out).as_posix()
        if f.is_file() and rel not in ("sw.js", "static/manifest.json"):
            data = f.read_bytes()
            files[rel] = {"sha256": hashlib.sha256(data).hexdigest(), "size": len(data)}
    desktop_min = (ROOT / "crates" / "mzlab-desktop" / "DESKTOP_MIN").read_text(encoding="utf-8").strip()
    doc = {"version": version, "desktop_min": desktop_min, "files": files}
    (out / "static" / "manifest.json").write_text(json.dumps(doc, indent=1, sort_keys=True), encoding="utf-8")


def teoria_entry() -> str:
    """site/teoria/index.html: the short address of the Teoria (teoria/, teoria/#5 for a chapter). It only forwards to static/teoria/, the real
    page (no second copy to download, same files in the service-worker cache); teoria.js turns #5, #cap-5 or #09-quadrupolo into the chapter."""
    return (
        '<!doctype html>\n<html lang="it"><head><meta charset="utf-8">' + tema_testa.snippet("teoria") + '<meta name="viewport" content="width=device-width,initial-scale=1">\n'
        f"<title>Teoria · {APP_NAME}</title>\n"
        '<script>location.replace("../static/teoria/index.html"+location.hash)</script>\n'
        '<noscript><meta http-equiv="refresh" content="0;url=../static/teoria/index.html"></noscript></head>\n'
        '<body><p><a href="../static/teoria/index.html">Teoria</a></p></body></html>\n'
    )


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--pyodide-dir", type=Path, help="accepted, no effect (the site carries no Python)")
    ap.add_argument("--no-pyodide", action="store_true", help="accepted, no effect (the site carries no Python)")
    ap.add_argument("--out", type=Path, default=ROOT / "site")
    a = ap.parse_args()
    out = a.out
    if out.exists():
        shutil.rmtree(out)
    static = out / "static"
    shutil.copytree(ROOT / "mzlab" / "web", static)
    for lic in ("LICENSE", "LICENZE-TERZI.md"):            # the licences travel with the site (Apache-2.0, BSD-3, MPL-2.0 ask for the notices)
        if (ROOT / lic).exists():
            shutil.copy2(ROOT / lic, out / lic)
    # the page: same index.html, plus the bridge that answers api/... without a server
    html = (static / "index.html").read_text(encoding="utf-8")
    # right after telefono.js (it decides whether this is a phone: then the engine is not started at all)
    tel = '<script src="static/telefono.js"></script>'
    assert tel in html
    html = html.replace(tel, tel + '\n<script src="static/browser.js"></script>', 1)
    # the name in the title is written here, not by appname.js: the apps that build link previews do not run JavaScript
    html = re.sub(r"<title>\{APP\}[^<]*</title>", lambda m: f"<title>{APP_NAME} • {TITLE_TAG}</title>", html, count=1)
    html = html.replace("</head>", preview_tags() + "\n</head>", 1)
    # installable app (PWA): manifest at the site root + theme colour; the local program does not use it
    html = html.replace("</head>", '<link rel="manifest" href="manifest.webmanifest"><meta name="theme-color" content="#ffffff"></head>', 1)
    (out / "manifest.webmanifest").write_text(json.dumps(MANIFEST, ensure_ascii=False, indent=2), encoding="utf-8")
    (out / "index.html").write_text(html, encoding="utf-8")
    (static / "index.html").unlink()
    (out / "teoria").mkdir()
    (out / "teoria" / "index.html").write_text(teoria_entry(), encoding="utf-8")
    (out / ".nojekyll").write_text("", encoding="utf-8")
    # program version and git commit for reproducibility (.mzflow)
    try:
        import subprocess
        commit = subprocess.run(["git", "rev-parse", "--short", "HEAD"], capture_output=True, text=True, cwd=ROOT).stdout.strip() or "dev"
    except Exception:
        commit = "dev"
    ver_match = re.search(r'__version__\s*=\s*"([^"]+)"', (ROOT / "mzlab" / "__init__.py").read_text(encoding="utf-8"))
    ver = ver_match.group(1) if ver_match else "0.1.0"
    (static / "version.js").write_text(f'window.MZLAB_VERSION = "{ver}";\nwindow.MZLAB_COMMIT = "{commit}";\n', encoding="utf-8")
    pack_mzfinder(static)
    bad = python_files(out)
    if bad:
        raise SystemExit(f"the site must not carry Python: {bad[:5]}")
    # service worker at the site root (scope = the whole site): offline use after the first visit
    sw = (static / "sw.js").read_text(encoding="utf-8")
    (static / "sw.js").unlink()
    h = hashlib.sha1()
    for f in sorted(out.rglob("*")):
        if f.is_file() and "vendor" not in f.parts:
            h.update(f.relative_to(out).as_posix().encode() + f.read_bytes())
    big = "v-" + hashlib.sha1(b"".join(f.name.encode() + f.read_bytes() for f in sorted((static / "vendor").rglob("*")) if f.is_file())).hexdigest()[:8]   # the content, not the size: a same-size change must not be served from the old cache
    (out / "sw.js").write_text(sw.replace("__APP__", h.hexdigest()[:10]).replace("__BIG__", big), encoding="utf-8")
    write_manifest(out, commit)
    size = sum(f.stat().st_size for f in out.rglob("*") if f.is_file())
    print(f"site/ ready: {size / 1e6:.1f} MB")


if __name__ == "__main__":
    main()
