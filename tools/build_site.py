"""Builds the static site (the whole program running in the browser) into site/.

    python tools/build_site.py

site/index.html + site/static/ (the web folder as is). The engine of the program, mzFinder included, is Rust (static/wasm/, static/wasm-feat/,
built by the workflow): the site downloads nothing from the net and carries no Python. `--pyodide-dir DIR` and `--no-pyodide` are still
accepted and do nothing (the workflows pass them). GitHub Pages serves the site as it is (.github/workflows/pages.yml).

Lab variant (parallel site for a lab, tools/lab/README.md): `MZLAB_LAB=1 MZLAB_LAB_EXPIRES=YYYY-MM-DD python tools/build_site.py` adds tools/lab/lab.js
(expiry page, only Data and Draw, banner), `noindex` and robots.txt. With MZLAB_LAB unset the output is byte for byte the normal site.
"""
from __future__ import annotations

import argparse
import datetime
import hashlib
import json
import os
import re
import shutil
import sys
import zipfile
from pathlib import Path

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
    data = [f.name for f in sorted((src / "data").iterdir()) if f.suffix == ".csv" or f.name in ("cancello.json", "regole_tp_2.json")] if (src / "data").is_dir() else []      # curated lists and the gate of quality, read by the scripts through QTOOLS.ctx.files
    for n in data:
        shutil.copy2(src / "data" / n, dst / n)
    # the step/1 descriptors that generate the form of the Pipeline panel (one datum, the Rust side is checked against it)
    shutil.copy2(ROOT / "docs" / "schemi" / "steps-1.json", dst / "steps-1.json")
    data.append("steps-1.json")
    for n in js:
        shutil.copy2(src / "js" / n, dst / n)
    # Flow (administrator page, flow/): its tab is registered by flow/js, the rest is lazy; the lists of descriptors of the other two packages are fetched when the page opens
    flow = ROOT / "flow"
    # the tab script is `early`: the loader runs it before the data files and the other scripts (it needs none of them), so Flow can draw while the rest of Tabui downloads
    early = []
    for n in sorted((flow / "js").glob("*.js")):
        shutil.copy2(n, dst / n.name)
        early.append(n.name)
    for n in ("steps-attributio-1.json", "steps-tabui-1.json", "steps-cli-1.json"):
        shutil.copy2(ROOT / "docs" / "schemi" / n, dst / n)
    lazy = [f.name for f in sorted((src / "lazy").glob("*.js"))] if (src / "lazy").is_dir() else []      # loaded on demand through TPLAZY.load, not at the start of Tabui
    lazy += [f.name for f in sorted((flow / "lazy").glob("*.js"))]
    if lazy:
        (dst / "lazy").mkdir()
        for n in lazy:
            shutil.copy2(src / "lazy" / n if (src / "lazy" / n).exists() else flow / "lazy" / n, dst / "lazy" / n)
    for n in files:
        shutil.copy2(src / "files" / n, dst / n)
    (dst / "indice.json").write_text(json.dumps({"early": early, "js": js, "lazy": lazy, "files": files + data}, indent=1), encoding="utf-8")


def file_hashes(out: Path) -> dict:
    """SHA-256 and size of every file of the site, by path under `out` (sw.js, the manifest itself and the service worker's manifest left out)."""
    files = {}
    for f in sorted(out.rglob("*")):
        rel = f.relative_to(out).as_posix()
        if f.is_file() and rel not in ("sw.js", "static/manifest.json", "static/sw-manifest.json"):
            data = f.read_bytes()
            files[rel] = {"sha256": hashlib.sha256(data).hexdigest(), "size": len(data)}
    return files


def write_manifest(out: Path, version: str) -> None:
    """static/manifest.json: SHA-256 and size of every file of the site (the desktop app downloads only the ones that changed),
    the commit as `version` and `desktop_min`, the oldest native desktop binary this web part works with (crates/mzlab-desktop/DESKTOP_MIN).
    sw.js is left out (the desktop app has no service worker) and so is the manifest itself."""
    desktop_min = (ROOT / "crates" / "mzlab-desktop" / "DESKTOP_MIN").read_text(encoding="utf-8").strip()
    doc = {"version": version, "desktop_min": desktop_min, "files": file_hashes(out)}
    (out / "static" / "manifest.json").write_text(json.dumps(doc, indent=1, sort_keys=True), encoding="utf-8")


def seal_sw(out: Path) -> None:
    """Names the service worker's cache after the site as it will be published: writes `static/sw-manifest.json` ({app, files: path -> SHA-256,
    nofill: the big ones, fetched only when used} of every file the worker may cache) and puts `app`, a hash of those paths and hashes, into sw.js. Runs again after the parts (.wasm,
    packages) are in place (`--build-json`), so a change in any of them is a new cache and a cached file is always the right version."""
    skip = ("sw.js", "static/manifest.json", "static/build.json", "static/sw-manifest.json")
    found = {p: h for p, h in file_hashes(out).items() if p not in skip and not p.startswith(("static/vendor/", "static/librerie/"))}
    files = {p: h["sha256"] for p, h in found.items()}
    nofill = sorted(p for p, h in found.items() if p.startswith("static/esempi/") or h["size"] > 6_000_000)   # cached when used, not fetched ahead
    app = hashlib.sha1("".join(f"{p}\0{h}\n" for p, h in sorted(files.items())).encode()).hexdigest()[:10]
    (out / "static" / "sw-manifest.json").write_text(json.dumps({"app": app, "files": files, "nofill": nofill}, indent=0, sort_keys=True), encoding="utf-8")
    text = (out / "sw.js").read_text(encoding="utf-8")
    sealed, n = re.subn(r'const APPV = "[^"]*";', f'const APPV = "{app}";', text)
    if n != 1:
        raise SystemExit("sw.js: no APPV to seal")
    (out / "sw.js").write_text(sealed, encoding="utf-8")


def git_commit() -> str:
    """The full 40-character commit of HEAD, or "dev" outside a checkout."""
    try:
        import subprocess
        c = subprocess.run(["git", "rev-parse", "HEAD"], capture_output=True, text=True, cwd=ROOT).stdout.strip()
    except Exception:
        c = ""
    return c if re.fullmatch(r"[0-9a-f]{40}", c) else "dev"


def rustc_version() -> str | None:
    """`rustc -V` of the toolchain that builds the site, None when there is none."""
    try:
        import subprocess
        v = subprocess.run(["rustc", "-V"], capture_output=True, text=True, cwd=ROOT).stdout.strip()
    except Exception:
        v = ""
    return v or None


def lock_sha256() -> str | None:
    lock = ROOT / "Cargo.lock"
    return hashlib.sha256(lock.read_bytes()).hexdigest() if lock.is_file() else None


def write_build_json(out: Path, version: str, commit: str | None = None) -> None:
    """static/build.json: the exact build of the site, {commit, version, rustc, cargo_lock_sha256, modules}; `modules` maps the path (under static/) of every
    .wasm to its SHA-256 (the hashes of `file_hashes`), sorted (the third-party ones under vendor/, e.g. Ketcher's Indigo, are not engine modules). The .wasm files are built after this script in pages.yml, so
    `python3 tools/build_site.py --build-json` writes it again once they are in place."""
    static = out / "static"
    modules = {rel[len("static/"):]: h["sha256"] for rel, h in file_hashes(out).items() if rel.startswith("static/") and not rel.startswith("static/vendor/") and rel.endswith(".wasm")}
    doc = {"commit": commit or git_commit(), "version": version, "rustc": rustc_version(), "cargo_lock_sha256": lock_sha256(), "modules": dict(sorted(modules.items()))}
    (static / "build.json").write_text(json.dumps(doc, indent=1), encoding="utf-8")


def lab_expiry() -> str | None:
    """The expiry date (YYYY-MM-DD, UTC) of the lab variant, or None for the normal build. A lab build without a valid date is refused."""
    if os.environ.get("MZLAB_LAB") != "1":
        return None
    d = os.environ.get("MZLAB_LAB_EXPIRES", "")
    try:
        datetime.date.fromisoformat(d)
    except ValueError:
        raise SystemExit(f"MZLAB_LAB=1 needs MZLAB_LAB_EXPIRES=YYYY-MM-DD, got {d!r}")
    if not re.fullmatch(r"\d{4}-\d{2}-\d{2}", d):
        raise SystemExit(f"MZLAB_LAB_EXPIRES must be YYYY-MM-DD, got {d!r}")
    return d


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--pyodide-dir", type=Path, help="accepted, no effect (the site carries no Python)")
    ap.add_argument("--no-pyodide", action="store_true", help="accepted, no effect (the site carries no Python)")
    ap.add_argument("--out", type=Path, default=ROOT / "site")
    ap.add_argument("--build-json", action="store_true", help="only write static/build.json and seal the service worker again, once the .wasm files are in the site")
    a = ap.parse_args()
    out = a.out
    if a.build_json:
        ver = json.loads((out / "static" / "build.json").read_text(encoding="utf-8"))["version"]
        write_build_json(out, ver)
        seal_sw(out)
        return
    lab = lab_expiry()
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
    if lab:
        # lab: first script of the head (before anything runs), no link preview and no canonical address pointing at the main site, not indexed
        head = '<head><meta charset="utf-8">'
        assert head in html
        html = html.replace(head, head + '<script src="static/lab.js"></script><meta name="robots" content="noindex,nofollow">', 1)
        (static / "lab.js").write_text((ROOT / "tools" / "lab" / "lab.js").read_text(encoding="utf-8").replace("__LAB_EXPIRES__", lab), encoding="utf-8")
        (out / "robots.txt").write_text("User-agent: *\nDisallow: /\n", encoding="utf-8")
        # lab: no example files at all (the demo mzML sets and the two example drawings are not shipped; lab.js hides their entry points)
        shutil.rmtree(static / "esempi")
        for png in static.glob("esempio-*.png"):
            png.unlink()
    else:
        html = html.replace("</head>", preview_tags() + "\n</head>", 1)
    # installable app (PWA): manifest at the site root + theme colour; the local program does not use it
    html = html.replace("</head>", '<link rel="manifest" href="manifest.webmanifest"><meta name="theme-color" content="#ffffff"></head>', 1)
    (out / "manifest.webmanifest").write_text(json.dumps(MANIFEST, ensure_ascii=False, indent=2), encoding="utf-8")
    (out / "index.html").write_text(html, encoding="utf-8")
    (static / "index.html").unlink()
    (out / ".nojekyll").write_text("", encoding="utf-8")
    # program version and git commit for reproducibility (.mzflow)
    commit = git_commit()
    ver_match = re.search(r'__version__\s*=\s*"([^"]+)"', (ROOT / "mzlab" / "__init__.py").read_text(encoding="utf-8"))
    ver = ver_match.group(1) if ver_match else "0.1.0"
    (static / "version.js").write_text(f'window.MZLAB_VERSION = "{ver}";\nwindow.MZLAB_COMMIT = "{commit}";\n', encoding="utf-8")
    pack_mzfinder(static)
    # the catalog of assistant tools beside its page (the module `llm/` reads it on first use)
    shutil.copy2(ROOT / "docs" / "schemi" / "strumenti-1.json", static / "llm" / "strumenti-1.json")
    # the prompts and roles of the assistant's subagents: data fetched on request, the same bytes as in the crate
    shutil.copytree(ROOT / "crates" / "mzlab-llm" / "data" / "assist", static / "llm" / "assist")
    bad = python_files(out)
    if bad:
        raise SystemExit(f"the site must not carry Python: {bad[:5]}")
    # service worker at the site root (scope = the whole site): offline use after the first visit
    sw = (static / "sw.js").read_text(encoding="utf-8")
    (static / "sw.js").unlink()
    big = "v-" + hashlib.sha1(b"".join(f.name.encode() + f.read_bytes() for f in sorted((static / "vendor").rglob("*")) if f.is_file())).hexdigest()[:8]   # the content, not the size: a same-size change must not be served from the old cache
    (out / "sw.js").write_text(sw.replace("__BIG__", big), encoding="utf-8")
    seal_sw(out)
    write_build_json(out, ver, commit)
    write_manifest(out, commit)
    size = sum(f.stat().st_size for f in out.rglob("*") if f.is_file())
    print(f"site/ ready: {size / 1e6:.1f} MB")


if __name__ == "__main__":
    main()
