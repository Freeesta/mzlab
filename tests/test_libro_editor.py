"""Libro editor page (mzlab/web/libro): the Markdown normaliser, the strict page CSP, and the rule that no text of the private Libro is ever in this public repository."""
import re
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
from dati_percorsi import percorso  # noqa: E402

WEB = ROOT / "mzlab" / "web"
NODE = shutil.which("node")


@pytest.mark.skipif(not NODE, reason="node not available")
def test_normalize_cleans_what_the_editor_escapes():
    code = """import { normalize } from %r;
const out = normalize("* uno \\\\[@a; @b]\\n* due `\\\\[x]`\\n\\n\\n\\n~ca \\\\~ 5\\n```\\n\\\\[keep]\\n```\\n");
console.log(JSON.stringify(out));""" % (WEB / "libro" / "norm.js").as_uri()
    out = subprocess.run([NODE, "--input-type=module", "-e", code], check=True, capture_output=True, encoding="utf-8").stdout
    assert out.strip() == '"- uno [@a; @b]\\n- due `\\\\[x]`\\n\\n~ca ~ 5\\n```\\n\\\\[keep]\\n```\\n"'


def test_editor_page_csp_is_strict():
    html = (WEB / "libro" / "editor.html").read_text(encoding="utf-8")
    csp = re.search(r'Content-Security-Policy" content="([^"]+)"', html).group(1)
    directives = dict(d.strip().split(" ", 1) for d in csp.split(";") if d.strip())
    assert directives["default-src"] == "'none'"
    assert directives["script-src"] == "'self'", "no inline script, no eval"
    assert directives["connect-src"] == "https://api.github.com", "the page may only talk to GitHub"
    assert "noindex" in html and not re.search(r"<script(?![^>]*\bsrc=)[^>]*>", html), "no inline script in the page"


def test_no_book_text_files_in_the_public_tree():
    for d in (WEB / "libro", WEB / "vendor" / "libro"):
        assert not [f for f in d.rglob("*") if f.suffix == ".md"], f"Markdown of the Libro must stay in the private repository: {d}"


def test_no_text_of_the_private_libro_in_this_repository():
    libro = percorso("Libro_Untargeted")
    if not libro:
        pytest.skip("private data repository not available")
    lines = {re.sub(r"\s+", " ", ln).strip() for f in libro.rglob("*.md") for ln in f.read_text(encoding="utf-8", errors="replace").splitlines()}
    lines = {ln for ln in lines if len(ln) >= 60 and not ln.startswith(("<!--", "|", "!["))}
    tracked = subprocess.run(["git", "ls-files"], cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout.splitlines()
    hits = []
    for rel in tracked:
        p = ROOT / rel
        if p.suffix not in {".js", ".html", ".md", ".py", ".json", ".css", ".txt", ".rs", ".yml", ".toml"} or "vendor/" in rel or not p.is_file() or p.stat().st_size > 3_000_000:
            continue
        text = re.sub(r"\s+", " ", p.read_text(encoding="utf-8", errors="replace"))
        hits += [f"{rel}: {ln[:60]}…" for ln in lines if ln in text]
    assert not hits, "text of the private Libro found in the public repository:\n" + "\n".join(hits[:10])
